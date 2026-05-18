# Component Communication — yt-downloader

Tauri 2 desktop app. React/TS frontend in `frontend/src/`, Rust backend in `src/`.
Two channels cross the IPC boundary:

1. **Frontend → Backend**: `invoke("cmd_name", args)` → `#[tauri::command]` in `src/commands.rs`.
2. **Backend → Frontend**: `AppEvent` broadcast bus → `bridge_to_tauri` → `emit("app-event", …)` → `listen("app-event", …)` in `App.tsx`.

External processes (`yt-dlp`, `ffmpeg`) and the YouTube Data API sit outside the app boundary.

## Flowchart

```mermaid
flowchart TB
    subgraph FE["Frontend (React / TypeScript) — frontend/src"]
        AppTSX["App.tsx<br/>(top-level state, listen('app-event'))"]
        Comps["components/*<br/>Results · Queue · Settings ·<br/>RecentDropdown · Segmented · Toggle ·<br/>TokenInput · Thumb · Icons"]
        ApiTS["api.ts<br/>invoke() wrapper"]
        TypesTS["types.ts<br/>format.ts · i18n.ts"]
        LocalStore["localStorage<br/>(recent searches)"]
    end

    subgraph IPC["Tauri IPC bridge"]
        Invoke{{"invoke('cmd', args)"}}
        EvtCh{{"event channel<br/>'app-event'"}}
    end

    subgraph BE["Rust backend — src/"]
        Main["main.rs<br/>(tauri::Builder, tokio runtime)"]
        Cmds["commands.rs<br/>#[tauri::command] fns +<br/>ServiceState (managed)"]
        subgraph Svc["service/"]
            AppSvc["AppService<br/>(Arc<Mutex<Inner>>)"]
            Search["search.rs<br/>SearchHandler / SearchState"]
            Events["events.rs<br/>AppEvent enum<br/>bridge_to_tauri()"]
            BusTx(("broadcast<br/>event_tx"))
            ProgRx(("mpsc<br/>progress_rx"))
            Bridge["spawn_progress_bridge<br/>(mpsc → broadcast)"]
        end
        subgraph Dl["download/"]
            Disp["dispatcher.rs<br/>DownloadDispatcher<br/>(MAX_CONCURRENT=5)"]
            Worker["worker.rs<br/>run_worker (per task)"]
            CmdBld["command_builder/<br/>build_args"]
            YtDlpMod["yt_dlp.rs<br/>spawn_and_track"]
            Progress["progress.rs<br/>ProgressEvent · DownloadPhase"]
        end
        subgraph Set["settings/"]
            SetMod["AppSettings"]
            SetPer["persistence.rs<br/>load / save"]
        end
        subgraph ApiMod["api/"]
            YtClient["youtube.rs<br/>YouTubeClient (ehttp)"]
            Parser["parser.rs<br/>SearchKind"]
            ApiTypes["types.rs<br/>YouTubeVideo · VideoId"]
        end
    end

    subgraph Ext["External"]
        YtBin[/"bin/ffmpeg/yt-dlp<br/>(child process)"/]
        FfBin[/"bin/ffmpeg/ffmpeg<br/>(child process via yt-dlp)"/]
        YtApi[("YouTube Data API v3")]
        FS[("File system<br/>temp_dir → download_path")]
        Disk[("settings.json on disk")]
    end

    %% Frontend internal
    AppTSX --> Comps
    Comps --> AppTSX
    AppTSX --> ApiTS
    Comps --> ApiTS
    AppTSX --> LocalStore
    AppTSX --- TypesTS
    Comps --- TypesTS

    %% Invoke path (FE → BE)
    ApiTS -->|"invoke()"| Invoke
    Invoke --> Cmds
    Cmds -->|"state.0.method()"| AppSvc

    %% Event path (BE → FE)
    AppSvc -->|"emit AppEvent"| BusTx
    BusTx --> Events
    Events -->|"handle.emit('app-event')"| EvtCh
    EvtCh -->|"listen()"| AppTSX

    %% Service internals
    AppSvc --> Search
    AppSvc --> Disp
    AppSvc --> SetMod
    SetMod <--> SetPer
    SetPer <--> Disk
    Search -->|emit| BusTx
    AppSvc -->|"submit_search → tokio::spawn"| YtClient
    YtClient -->|HTTPS| YtApi
    YtClient -->|results| Search

    %% Download pipeline
    Disp -->|"tokio::spawn run_worker"| Worker
    Worker --> CmdBld
    Worker --> YtDlpMod
    YtDlpMod -->|spawn| YtBin
    YtBin --> FfBin
    YtBin -->|stdout lines| YtDlpMod
    Worker -->|"ProgressEvent"| ProgRx
    ProgRx --> Bridge
    Bridge -->|"phase update + AppEvent::PhaseChanged"| AppSvc
    Worker -->|move file| FS

    %% Boot wiring
    Main -->|build| AppSvc
    Main -->|register| Cmds
    Main -->|"spawn bridge_to_tauri"| Events

    %% Styling
    classDef ext fill:#fef3c7,stroke:#b45309,color:#000
    classDef ipc fill:#e0e7ff,stroke:#3730a3,color:#000
    class YtBin,FfBin,YtApi,FS,Disk ext
    class Invoke,EvtCh ipc
```

## Communication summary per component

| Component | Talks to | Mechanism |
|---|---|---|
| `App.tsx` / components | `api.ts` | direct TS call |
| `api.ts` | Tauri core | `invoke<T>(cmdName, args)` over IPC |
| `App.tsx` | Tauri events | `listen("app-event", cb)` (`APP_EVENT_CHANNEL`) |
| `commands.rs` | `AppService` | method call through `State<ServiceState>` |
| `AppService` | `SearchHandler`, `DownloadDispatcher`, `AppSettings` | direct call inside `Arc<Mutex<Inner>>` |
| `AppService` | subscribers | `tokio::sync::broadcast<AppEvent>` (cap 256) |
| `YouTubeClient` | YouTube Data API v3 | HTTPS via `ehttp`, spawned on tokio |
| `DownloadDispatcher` | `run_worker` | `tokio::spawn` per worker, round-robin job buckets |
| `run_worker` → progress bridge | `AppService` | `tokio::sync::mpsc<ProgressEvent>` (cap 256) |
| progress bridge → frontend | `Events::bridge_to_tauri` | rebroadcast as `AppEvent::PhaseChanged`, then `emit("app-event")` |
| `yt_dlp.rs` | `yt-dlp` binary | child process; parses stdout into `DownloadPhase` |
| `yt-dlp` | `ffmpeg` | child process (merge/remux) |
| `worker.rs` | filesystem | move temp file → `settings.download_path` |
| `settings::persistence` | disk | `settings.json` load/save on startup + on `update_settings` |
| `App.tsx` | browser | `localStorage` for recent searches |

## Event taxonomy (`AppEvent`)

`SearchSubmitted` · `SearchResolved` · `SearchCleared` · `AutoDownloadStarted` ·
`PhaseChanged` · `DownloadRejected` · `SelectionToggled` · `SettingsUpdated`.
All past-tense facts, serialized as `{ kind, …fields }` (camelCase).

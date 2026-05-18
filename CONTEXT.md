# yt-downloader

Desktop application (Tauri + React) that lets a user resolve a YouTube query, video, or playlist via the YouTube Data API v3 and drive `yt-dlp` to download the results. Target architecture is event-driven: the UI dispatches **Actions**, the service emits **AppEvents**, and the UI reads a derived **ReadModel**.

## Language

### Architecture

**Action**:
A user-initiated intent dispatched from the UI to the service (e.g. `SubmitSearch`, `DownloadSingle`, `UpdateSettings`).
_Avoid_: Command (clashes with `tokio::process::Command` and `command_builder::Command`), Intent, Message.

**AppEvent**:
A past-tense fact emitted by the service when domain state changes (e.g. `SearchResolved`, `PhaseChanged`, `SettingsUpdated`). Every state mutation produces exactly one AppEvent.
_Avoid_: Event (clashes with Tauri's `Event` type), DomainEvent, Notification.

**ReadModel**:
The immutable projection of accumulated AppEvents that the UI consumes. The only shape the frontend ever reads.
_Avoid_: Snapshot (legacy `PollSnapshot` is the pre-EDA polling shape), View, State.

**Handler**:
The service-layer function that consumes one Action, performs side effects, and emits the resulting AppEvents.

### Domain

**VideoId**:
Newtype around a YouTube video ID string. Hash key for all per-video state.
_Avoid_: id (bare), VideoKey.

**SearchKind**:
Classification of a user-entered query string. One of `Query(text)`, `VideoId(id)`, or `PlaylistId(id)`. Decides which YouTube Data API endpoint is called.
_Avoid_: QueryType, Lookup.

**YouTubeVideo**:
A fully resolved video result row: id, title, channel, duration, views, published date, thumbnail URL. Built by `YouTubeClient::resolve` after merging `videos.list` data into the IDs returned by search/playlist endpoints.
_Avoid_: Video (ambiguous), Result, Item.

**SearchState**:
The current phase of the search lifecycle. One of `Idle`, `Pending`, `Showing`, or `AutoDownloading`. Replaces the current scattering of `searched`, `last_query`, `search_rx`, `playlist_mode_active`, and `results` fields.

**DownloadJob**:
A single unit of work handed to a Worker: `VideoId`, `AppSettings` snapshot (captured at dispatch time so later settings changes do not affect in-flight jobs), and a temp directory.
_Avoid_: Task, Download (as a noun).

**DownloadPhase**:
The lifecycle state of one DownloadJob. One of `Queued`, `Downloading(progress)`, `PostProcessing`, `Moving`, `Done`, `Failed(error)`. Each transition is emitted as an AppEvent (`PhaseChanged`).
_Avoid_: DownloadStatus, JobState.

**Worker**:
A tokio task that owns one `VecDeque<DownloadJob>` and processes jobs sequentially. No shared mutable state across workers.

**DownloadManager**:
Coordinator that distributes DownloadJobs across up to `MAX_CONCURRENT` (5) Workers via round-robin. Owns the worker count, not the job queue.
_Avoid_: Scheduler, JobQueue.

**AppSettings**:
The persisted user configuration: format, quality, codec, audio bitrate, download path, extras, theme, API key, etc. Lives in `settings.toml`. Cloned into each DownloadJob at dispatch time.

## Relationships

- A user gesture in the UI produces one **Action**.
- A **Handler** consumes one **Action** and emits zero or more **AppEvents**.
- The service folds **AppEvents** into the **ReadModel**; the UI reads only the **ReadModel**.
- A `SubmitSearch` **Action** triggers a YouTube Data API call, classified by **SearchKind**, ending in a `SearchResolved` **AppEvent** carrying `Vec<YouTubeVideo>`.
- A `DownloadSingle` / `DownloadSelected` **Action** produces one or more **DownloadJobs**, which the **DownloadManager** distributes across **Workers**.
- Each **Worker** emits `PhaseChanged` **AppEvents** as its current **DownloadJob** moves through **DownloadPhases**.
- An `UpdateSettings` **Action** replaces **AppSettings**, persists to disk, and emits `SettingsUpdated`.

## Example dialogue

> **Dev:** "When the user clicks Download on a result row, does the UI mutate the download list directly?"
> **Architect:** "No. The UI dispatches a `DownloadSingle` **Action**. The **Handler** asks the **DownloadManager** to construct a **DownloadJob** and hand it to a **Worker**. The Worker emits `PhaseChanged(id, Queued)` as the first **AppEvent**, the **ReadModel** folds it in, and the UI re-renders from the ReadModel."

> **Dev:** "What if the user searches mid-download?"
> **Architect:** "The `SubmitSearch` **Action** only touches **SearchState**. **DownloadPhase** state for in-flight jobs lives in a different region of the **ReadModel** and is untouched."

## Flagged ambiguities

- **"Command"** appears three times in the codebase: `tokio::process::Command`, `command_builder::Command` (yt-dlp argument vector), and would have collided with the CQRS sense. Resolution: the user-intent type is named **Action**; the existing `Command` types keep their names.
- **"Event"** alone is ambiguous: Tauri ships its own `Event`, and `ProgressEvent` is the current pre-EDA name for the worker → service message. Resolution: always use **AppEvent** for domain events; `ProgressEvent` is treated as a synonym for the `PhaseChanged` AppEvent during migration and will be removed.
- **"Snapshot"** today means the polling response shape (`PollSnapshot` in `commands.rs`). It is **not** the **ReadModel**. The polling path is legacy and will be replaced by Tauri `emit`-based push during the EDA migration.
- **"Video"** without qualification is ambiguous (DB row vs. on-disk file vs. domain object). Resolution: domain object is **YouTubeVideo**; on-disk file has no name yet — flag if it becomes load-bearing.

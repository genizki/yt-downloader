# AppService is Tauri-only — no UI abstraction trait

The original `src/service/` was structured around two trait interfaces (`SettingsInterface`, `DownloadInterface`) so a hypothetical second frontend (egui, web, Swift FFI) could swap in alongside Tauri. That second frontend was never built; the traits had one adapter (`AppService`) and one consumer (Tauri `commands.rs`), and their methods were getter/setter pass-throughs that leaked internal state (`&mut AppSettings`, `&HashMap<VideoId, DownloadPhase>`).

**Decision:** remove both traits. `AppService` exposes its operations as inherent methods. Tauri commands depend on `AppService` directly.

**If multi-frontend returns**, design the new seam from real call sites at that time — do not revive the leaky getter-traits. The current `commands.rs` is small enough (~160 lines) that re-introducing a UI-shaped interface is a half-day of work when there is actually a second adapter to serve.

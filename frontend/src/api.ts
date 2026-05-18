import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppSettings, SearchStatus, VideoPhase, YouTubeVideo } from "./types";

export async function pickDownloadDir(current: string): Promise<string | null> {
  const r = await open({
    directory: true,
    multiple: false,
    defaultPath: current || undefined,
  });
  return typeof r === "string" ? r : null;
}

export const api = {
  getSettings: () => invoke<AppSettings>("get_settings"),
  updateSettings: (settings: AppSettings) =>
    invoke<void>("update_settings", { settings }),
  getResults: () => invoke<YouTubeVideo[]>("get_results"),
  getPhases: () => invoke<VideoPhase[]>("get_phases"),
  getSearchStatus: () => invoke<SearchStatus>("get_search_status"),
  submitSearch: (query: string) => invoke<void>("submit_search", { query }),
  clearSearch: () => invoke<void>("clear_search"),
  downloadSingle: (videoId: string) =>
    invoke<void>("download_single", { videoId }),
};

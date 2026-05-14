import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppSettings, PollSnapshot } from "./types";

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
  submitSearch: (query: string) => invoke<void>("submit_search", { query }),
  clearSearch: () => invoke<void>("clear_search"),
  poll: () => invoke<PollSnapshot>("poll"),
  downloadSingle: (videoId: string) =>
    invoke<void>("download_single", { videoId }),
  downloadSelected: () => invoke<void>("download_selected"),
  toggleSelected: (videoId: string, selected: boolean) =>
    invoke<void>("toggle_selected", { videoId, selected }),
};

export interface YouTubeVideo {
  id: string;
  title: string;
  channel: string;
  durationSeconds: number;
  views: number;
  publishedAt: string;
  thumbnailUrl: string;
}

export type PhaseKind =
  | "queued"
  | "downloading"
  | "post_processing"
  | "moving"
  | "done"
  | "failed";

export type Phase =
  | { kind: "queued" }
  | { kind: "downloading"; progress: number }
  | { kind: "post_processing" }
  | { kind: "moving" }
  | { kind: "done" }
  | { kind: "failed"; error: string };

export interface VideoPhase {
  id: string;
  phase: Phase;
}

export interface PollSnapshot {
  searched: boolean;
  lastQuery: string;
  results: YouTubeVideo[];
  phases: VideoPhase[];
  selected: string[];
}

export interface Extras {
  embedThumbnail: boolean;
  embedMetadata: boolean;
  embedChapters: boolean;
  embedSubtitles: boolean;
  writeSubtitles: boolean;
  skipPlaylists: boolean;
  restrictNames: boolean;
}

export interface PlaylistSettings {
  mode: "Single" | "Playlist";
  playlistStart?: number;
  playlistEnd?: number;
  playlistItems?: string;
  maxDownloads?: number;
  matchTitle?: string;
  rejectTitle?: string;
  date?: string;
  datebefore?: string;
  dateafter?: string;
  minViews?: number;
}

export type CookiesSettings =
  | { kind: "None" }
  | { kind: "File"; path: string }
  | { kind: "Browser"; browser: string };

export interface NetworkSettings {
  rateLimit?: string;
  concurrentFragments?: number;
  retries?: number;
  fragmentRetries?: number;
  cookies: CookiesSettings;
  proxy?: string;
  sourceAddress?: string;
  geoBypass: boolean;
}

export interface MetadataExtrasSettings {
  addMetadata: boolean;
  writeThumbnail: boolean;
  writeInfoJson: boolean;
  writeDescription: boolean;
}

export interface SubtitlesSettings {
  writeAutoSubs: boolean;
  subLangs: string[];
  subFormat?: string;
}

export type SponsorblockSettings =
  | { kind: "None" }
  | { kind: "Remove"; segments: string[] }
  | { kind: "Mark"; segments: string[] };

export interface PostProcessingSettings {
  sponsorblock: SponsorblockSettings;
  splitChapters: boolean;
  downloadSections?: string;
  exec?: string;
  ffmpegLocation?: string;
  postprocessorArgs?: string;
}

export interface MiscSettings {
  simulate: boolean;
  verbose: boolean;
  quiet: boolean;
  noWarnings: boolean;
  sleepInterval?: number;
}

/** Mirrors Rust `AppSettings` (camelCase via serde). */
export interface AppSettings {
  format: string;
  quality: string;
  codec: string;
  audioBitrate: string;
  downloadPath: string;
  maxSize: string;
  protocol: string;
  extras: Extras;
  theme: string;
  language: string;
  authToken: string;
  playlistAutoDownload: boolean;
  youtubeApiKey: string;
  playlist: PlaylistSettings;
  network: NetworkSettings;
  metadataExtras: MetadataExtrasSettings;
  subtitles: SubtitlesSettings;
  postProcessing: PostProcessingSettings;
  misc: MiscSettings;
}

export const AUDIO_FORMATS = ["MP3", "M4A", "AAC", "ALAC", "AIFF", "FLAC"];
export const TOKEN_MASK = "••••••••••••••••••••••••";

export const LANGUAGE_LABELS: Record<string, string> = {
  en: "English",
  de: "Deutsch",
  fr: "Français",
  ja: "日本語",
};

/** Lightweight result-card view derived from `YouTubeVideo`. */
export interface ResultCard {
  id: string;
  title: string;
  author: string;
  duration: string;
  views: string;
  posted: string;
  hue: number;
  thumbnailUrl: string;
}

export interface QueueRow {
  id: string;
  title: string;
  state:
    | "downloading"
    | "queued"
    | "done"
    | "failed"
    | "post_processing"
    | "moving";
  progress: number;
  format: string;
}

const DEFAULT_EXTRAS: Extras = {
  embedThumbnail: false,
  embedMetadata: false,
  embedChapters: false,
  embedSubtitles: false,
  writeSubtitles: false,
  skipPlaylists: false,
  restrictNames: false,
};

const DEFAULT_PLAYLIST: PlaylistSettings = {
  mode: "Single",
};

const DEFAULT_NETWORK: NetworkSettings = {
  cookies: { kind: "None" },
  geoBypass: true,
};

const DEFAULT_METADATA_EXTRAS: MetadataExtrasSettings = {
  addMetadata: false,
  writeThumbnail: false,
  writeInfoJson: false,
  writeDescription: false,
};

const DEFAULT_SUBTITLES: SubtitlesSettings = {
  writeAutoSubs: false,
  subLangs: [],
};

const DEFAULT_POST_PROCESSING: PostProcessingSettings = {
  sponsorblock: { kind: "None" },
  splitChapters: false,
};

const DEFAULT_MISC: MiscSettings = {
  simulate: false,
  verbose: false,
  quiet: false,
  noWarnings: false,
};

export const DEFAULT_APP_SETTINGS: AppSettings = {
  format: "MP4",
  quality: "1080p",
  codec: "H.264",
  audioBitrate: "192 kbps",
  downloadPath: "",
  maxSize: "No limit",
  protocol: "Auto",
  extras: DEFAULT_EXTRAS,
  theme: "System",
  language: "en",
  authToken: "",
  playlistAutoDownload: false,
  youtubeApiKey: "",
  playlist: DEFAULT_PLAYLIST,
  network: DEFAULT_NETWORK,
  metadataExtras: DEFAULT_METADATA_EXTRAS,
  subtitles: DEFAULT_SUBTITLES,
  postProcessing: DEFAULT_POST_PROCESSING,
  misc: DEFAULT_MISC,
};

type PartialSettings = Partial<AppSettings> | null | undefined;

function parseCookies(value: unknown): CookiesSettings {
  if (!value || typeof value !== "object") return { kind: "None" };
  const candidate = value as Partial<CookiesSettings>;
  if (candidate.kind === "File") {
    return { kind: "File", path: typeof candidate.path === "string" ? candidate.path : "" };
  }
  if (candidate.kind === "Browser") {
    return {
      kind: "Browser",
      browser: typeof candidate.browser === "string" ? candidate.browser : "",
    };
  }
  return { kind: "None" };
}

function parseSponsorblock(value: unknown): SponsorblockSettings {
  if (!value || typeof value !== "object") return { kind: "None" };
  const candidate = value as Partial<SponsorblockSettings>;
  if (candidate.kind === "Remove" || candidate.kind === "Mark") {
    return {
      kind: candidate.kind,
      segments: Array.isArray(candidate.segments)
        ? candidate.segments.filter((x): x is string => typeof x === "string")
        : [],
    };
  }
  return { kind: "None" };
}

export function normalizeAppSettings(input: PartialSettings): AppSettings {
  const raw = input ?? {};
  const playlist: Partial<PlaylistSettings> = raw.playlist ?? {};
  const network: Partial<NetworkSettings> = raw.network ?? {};
  const metadataExtras: Partial<MetadataExtrasSettings> = raw.metadataExtras ?? {};
  const subtitles: Partial<SubtitlesSettings> = raw.subtitles ?? {};
  const postProcessing: Partial<PostProcessingSettings> = raw.postProcessing ?? {};
  const misc: Partial<MiscSettings> = raw.misc ?? {};

  return {
    format: raw.format ?? DEFAULT_APP_SETTINGS.format,
    quality: raw.quality ?? DEFAULT_APP_SETTINGS.quality,
    codec: raw.codec ?? DEFAULT_APP_SETTINGS.codec,
    audioBitrate: raw.audioBitrate ?? DEFAULT_APP_SETTINGS.audioBitrate,
    downloadPath: raw.downloadPath ?? DEFAULT_APP_SETTINGS.downloadPath,
    maxSize: raw.maxSize ?? DEFAULT_APP_SETTINGS.maxSize,
    protocol: raw.protocol ?? DEFAULT_APP_SETTINGS.protocol,
    extras: { ...DEFAULT_EXTRAS, ...(raw.extras ?? {}) },
    theme: raw.theme ?? DEFAULT_APP_SETTINGS.theme,
    language: raw.language ?? DEFAULT_APP_SETTINGS.language,
    authToken: raw.authToken ?? DEFAULT_APP_SETTINGS.authToken,
    playlistAutoDownload: raw.playlistAutoDownload ?? DEFAULT_APP_SETTINGS.playlistAutoDownload,
    youtubeApiKey: raw.youtubeApiKey ?? DEFAULT_APP_SETTINGS.youtubeApiKey,
    playlist: {
      mode: playlist.mode === "Playlist" ? "Playlist" : "Single",
      playlistStart: playlist.playlistStart,
      playlistEnd: playlist.playlistEnd,
      playlistItems: playlist.playlistItems,
      maxDownloads: playlist.maxDownloads,
      matchTitle: playlist.matchTitle,
      rejectTitle: playlist.rejectTitle,
      date: playlist.date,
      datebefore: playlist.datebefore,
      dateafter: playlist.dateafter,
      minViews: playlist.minViews,
    },
    network: {
      rateLimit: network.rateLimit,
      concurrentFragments: network.concurrentFragments,
      retries: network.retries,
      fragmentRetries: network.fragmentRetries,
      cookies: parseCookies(network.cookies),
      proxy: network.proxy,
      sourceAddress: network.sourceAddress,
      geoBypass: network.geoBypass ?? DEFAULT_NETWORK.geoBypass,
    },
    metadataExtras: {
      addMetadata: metadataExtras.addMetadata ?? DEFAULT_METADATA_EXTRAS.addMetadata,
      writeThumbnail: metadataExtras.writeThumbnail ?? DEFAULT_METADATA_EXTRAS.writeThumbnail,
      writeInfoJson: metadataExtras.writeInfoJson ?? DEFAULT_METADATA_EXTRAS.writeInfoJson,
      writeDescription: metadataExtras.writeDescription ?? DEFAULT_METADATA_EXTRAS.writeDescription,
    },
    subtitles: {
      writeAutoSubs: subtitles.writeAutoSubs ?? DEFAULT_SUBTITLES.writeAutoSubs,
      subLangs: Array.isArray(subtitles.subLangs)
        ? subtitles.subLangs.filter((x): x is string => typeof x === "string")
        : [],
      subFormat: subtitles.subFormat,
    },
    postProcessing: {
      sponsorblock: parseSponsorblock(postProcessing.sponsorblock),
      splitChapters: postProcessing.splitChapters ?? DEFAULT_POST_PROCESSING.splitChapters,
      downloadSections: postProcessing.downloadSections,
      exec: postProcessing.exec,
      ffmpegLocation: postProcessing.ffmpegLocation,
      postprocessorArgs: postProcessing.postprocessorArgs,
    },
    misc: {
      simulate: misc.simulate ?? DEFAULT_MISC.simulate,
      verbose: misc.verbose ?? DEFAULT_MISC.verbose,
      quiet: misc.quiet ?? DEFAULT_MISC.quiet,
      noWarnings: misc.noWarnings ?? DEFAULT_MISC.noWarnings,
      sleepInterval: misc.sleepInterval,
    },
  };
}

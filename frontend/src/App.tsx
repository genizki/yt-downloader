import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Icon } from "./components/Icons";
import { Settings } from "./components/Settings";
import { ResultRow } from "./components/Results";
import { RecentDropdown } from "./components/RecentDropdown";
import {
  AppEvent,
  AppSettings,
  Phase,
  ResultCard,
  VideoPhase,
  YouTubeVideo,
  normalizeAppSettings,
} from "./types";
import { api } from "./api";
import {
  formatDuration,
  formatRelative,
  formatViews,
  hueFromId,
} from "./format";
import { I18nContext, translate } from "./i18n";

const RECENT_KEY = "yt-dlp-gui:recent-searches";
const MAX_RECENT = 8;

export type RecentEntry = { label: string; query: string };

function loadRecent(): RecentEntry[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    const out: RecentEntry[] = [];
    for (const item of parsed) {
      if (typeof item === "string") {
        out.push({ label: item, query: item });
      } else if (
        item &&
        typeof item === "object" &&
        typeof (item as RecentEntry).label === "string" &&
        typeof (item as RecentEntry).query === "string"
      ) {
        out.push({
          label: (item as RecentEntry).label,
          query: (item as RecentEntry).query,
        });
      }
    }
    return out;
  } catch {
    return [];
  }
}

function persistRecent(list: RecentEntry[]) {
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(list));
  } catch {
    /* ignore quota errors */
  }
}

// Dedupe by query (the submitted text) so URL+title pairs collapse correctly
// even when the resolved label differs from a previous raw-URL entry.
function pushRecent(list: RecentEntry[], entry: RecentEntry): RecentEntry[] {
  const next = [entry, ...list.filter((x) => x.query !== entry.query)].slice(
    0,
    MAX_RECENT,
  );
  persistRecent(next);
  return next;
}

function isYoutubeUrl(s: string): "video" | "playlist" | null {
  if (/[?&]list=/.test(s)) return "playlist";
  if (
    /youtube\.com\/watch\?v=/.test(s) ||
    /youtu\.be\//.test(s) ||
    /youtube\.com\/shorts\//.test(s) ||
    /youtube\.com\/embed\//.test(s) ||
    /youtube\.com\/live\//.test(s)
  ) {
    return "video";
  }
  return null;
}

function toResultCard(v: YouTubeVideo): ResultCard {
  return {
    id: v.id,
    title: v.title,
    author: v.channel,
    duration: formatDuration(v.durationSeconds),
    views: formatViews(v.views),
    posted: formatRelative(v.publishedAt),
    thumbnailUrl: v.thumbnailUrl,
    hue: hueFromId(v.id),
  };
}

interface BatchSummary {
  active: number;
  done: number;
  failed: number;
  total: number;
}

function summarizePhases(phases: VideoPhase[]): BatchSummary {
  let active = 0;
  let done = 0;
  let failed = 0;
  for (const { phase } of phases) {
    switch (phase.kind) {
      case "queued":
      case "downloading":
      case "post_processing":
      case "moving":
        active++;
        break;
      case "done":
        done++;
        break;
      case "failed":
        failed++;
        break;
    }
  }
  return { active, done, failed, total: phases.length };
}

export default function App() {
  const [query, setQuery] = useState("");
  const [searched, setSearched] = useState(false);
  const [focused, setFocused] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [results, setResults] = useState<YouTubeVideo[]>([]);
  const [phases, setPhases] = useState<VideoPhase[]>([]);
  const [lastQuery, setLastQuery] = useState("");
  const [autoDownloadingCount, setAutoDownloadingCount] = useState<
    number | null
  >(null);
  const [noApiKey, setNoApiKey] = useState(false);
  const [pollMs, setPollMs] = useState<number>(0); // synthetic "search took X ms"
  const [recent, setRecent] = useState<RecentEntry[]>(() => loadRecent());
  const inputRef = useRef<HTMLInputElement>(null);
  const searchStartRef = useRef<number | null>(null);

  // ── Load settings on mount ───────────────────────────────────────────────
  useEffect(() => {
    api
      .getSettings()
      .then((next) => setSettings(normalizeAppSettings(next)))
      .catch((e) => console.error("get_settings failed", e));
  }, []);

  // ── Persist settings on every change (debounced via microtask batching) ─
  useEffect(() => {
    if (!settings) return;
    api
      .updateSettings(settings)
      .catch((e) => console.error("update_settings failed", e));
  }, [settings]);

  // ── Hydrate read-model state on mount ───────────────────────────────────
  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      api.getResults(),
      api.getPhases(),
      api.getSearchStatus(),
    ])
      .then(([nextResults, nextPhases, status]) => {
        if (cancelled) return;
        setResults(nextResults);
        setPhases(nextPhases);
        setSearched(status.searched);
        setLastQuery(status.lastQuery);
        setAutoDownloadingCount(status.autoDownloadingCount);
        setNoApiKey(status.noApiKey);
      })
      .catch((e) => console.error("initial read-model hydration failed", e));

    return () => {
      cancelled = true;
    };
  }, []);

  // ── Keep recent labels in sync when result titles arrive ────────────────
  useEffect(() => {
    if (!(searched && results.length > 0 && lastQuery)) return;
    const lq = lastQuery;
    const firstTitle = results[0].title;
    setRecent((prev) => {
      const idx = prev.findIndex((e) => e.query === lq);
      if (idx === -1) return prev;
      const entry = prev[idx];
      if (entry.label !== entry.query) return prev;
      const kind = isYoutubeUrl(entry.query);
      if (kind === null) return prev;
      const newLabel = kind === "playlist" ? `📃 ${firstTitle}` : firstTitle;
      if (newLabel === entry.label) return prev;
      const next = prev.slice();
      next[idx] = { label: newLabel, query: entry.query };
      persistRecent(next);
      return next;
    });
  }, [searched, results, lastQuery]);

  const handleAppEvent = useCallback((event: AppEvent) => {
    switch (event.kind) {
      case "searchSubmitted":
        setLastQuery(event.query);
        setSearched(true);
        setNoApiKey(!event.hasApiKey);
        setAutoDownloadingCount(null);
        setResults([]);
        setPhases([]);
        return;
      case "searchResolved":
        setSearched(true);
        setNoApiKey(false);
        setAutoDownloadingCount(null);
        void api
          .getResults()
          .then((nextResults) => {
            setResults(nextResults);
            if (searchStartRef.current != null && nextResults.length > 0) {
              setPollMs(Date.now() - searchStartRef.current);
              searchStartRef.current = null;
            }
          })
          .catch((e) =>
            console.error("get_results failed after searchResolved", e),
          );
        return;
      case "searchCleared":
        setSearched(false);
        setLastQuery("");
        setResults([]);
        setPhases([]);
        setAutoDownloadingCount(null);
        setNoApiKey(false);
        setPollMs(0);
        return;
      case "autoDownloadStarted":
        setSearched(true);
        setNoApiKey(false);
        setAutoDownloadingCount(event.count);
        void api
          .getResults()
          .then((nextResults) => setResults(nextResults))
          .catch((e) =>
            console.error("get_results failed after autoDownloadStarted", e),
          );
        return;
      case "phaseChanged":
        setPhases((prev) => {
          const idx = prev.findIndex((p) => p.id === event.videoId);
          if (idx === -1)
            return [...prev, { id: event.videoId, phase: event.phase }];
          const next = prev.slice();
          next[idx] = { id: event.videoId, phase: event.phase };
          return next;
        });
        return;
      case "downloadRejected":
        console.warn("download rejected", event.videoId, event.reason);
        return;
      case "settingsUpdated":
        return;
    }
  }, []);

  // ── Subscribe to backend AppEvent stream ────────────────────────────────
  useEffect(() => {
    const unlistenP = listen<AppEvent>("app-event", (e) => {
      handleAppEvent(e.payload);
    });
    return () => {
      void unlistenP
        .then((unlisten) => unlisten())
        .catch((e) => console.error("failed to unlisten app-event", e));
    };
  }, [handleAppEvent]);

  // ── Cmd/Ctrl-K focuses the searchbar ────────────────────────────────────
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // ── Freeze background bloom while searching ─────────────────────────────
  useEffect(() => {
    document.body.classList.toggle("is-static", searched);
  }, [searched]);

  // ── Apply theme to <html data-theme="…"> ────────────────────────────────
  useEffect(() => {
    const theme = settings?.theme ?? "System";
    const root = document.documentElement;
    const apply = (resolved: "light" | "dark") => {
      root.setAttribute("data-theme", resolved);
    };
    if (theme === "Light") {
      apply("light");
      return;
    }
    if (theme === "Dark") {
      apply("dark");
      return;
    }
    // System: follow prefers-color-scheme and subscribe to changes.
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    apply(mql.matches ? "dark" : "light");
    const onChange = (e: MediaQueryListEvent) =>
      apply(e.matches ? "dark" : "light");
    mql.addEventListener("change", onChange);
    return () => mql.removeEventListener("change", onChange);
  }, [settings?.theme]);

  const submit = useCallback(
    async (q?: string) => {
      const value = (q ?? query).trim();
      if (!value) return;
      setQuery(value);
      setRecent((prev) => pushRecent(prev, { label: value, query: value }));
      searchStartRef.current = Date.now();
      setPollMs(0);
      try {
        await api.submitSearch(value);
      } catch (e) {
        console.error("submit_search failed", e);
      }
      inputRef.current?.blur();
    },
    [query],
  );

  const reset = useCallback(async () => {
    setQuery("");
    try {
      await api.clearSearch();
    } catch (e) {
      console.error("clear_search failed", e);
    }
    setTimeout(() => inputRef.current?.focus(), 250);
  }, []);

  // ── Escape aborts an active search (returns to the hero view) ───────────
  useEffect(() => {
    if (!searched) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        reset();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [searched, reset]);

  const onResultClick = useCallback((videoId: string) => {
    api
      .downloadSingle(videoId)
      .catch((e) => console.error("download_single failed", e));
  }, []);

  const cards = useMemo<ResultCard[]>(
    () => results.map(toResultCard),
    [results],
  );
  const phaseById = useMemo(
    () => new Map<string, Phase>(phases.map((p) => [p.id, p.phase])),
    [phases],
  );
  const batch = useMemo<BatchSummary>(() => summarizePhases(phases), [phases]);

  const setSettingsUpdater = useCallback(
    (updater: (s: AppSettings) => AppSettings) => {
      setSettings((prev) => (prev ? updater(prev) : prev));
    },
    [],
  );

  const lang = settings?.language ?? "en";
  const t = (key: string, vars?: Record<string, string | number>) =>
    translate(lang, key, vars);

  return (
    <I18nContext.Provider value={lang}>
      <div className="app">
        <header className="topbar">
          <div className="topbar-left">
            <button
              className={`brand ${searched ? "brand--visible" : ""}`}
              onClick={reset}
              aria-label={t("topbar.new_search")}
            >
              <span className="brand-mark" />
              <span className="brand-name">yt-dlp</span>
            </button>
          </div>
          <button
            className="icon-btn gear"
            onClick={() => setSettingsOpen(true)}
            aria-label={t("topbar.open_settings")}
          >
            <Icon.Gear width={18} height={18} />
          </button>
        </header>

        <main className={`stage ${searched ? "stage--searched" : ""}`}>
          <div className="search-wrap">
            {!searched && (
              <div className="hero">
                <div className="hero-eyebrow">yt-dlp</div>
                <h1 className="hero-title">{t("search.hero")}</h1>
              </div>
            )}

            <div className={`searchbar ${focused ? "searchbar--focused" : ""}`}>
              <Icon.Search className="searchbar-icon" width={18} height={18} />
              <input
                ref={inputRef}
                className="searchbar-input"
                type="text"
                placeholder={t("search.placeholder")}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onFocus={() => setFocused(true)}
                onBlur={() => setTimeout(() => setFocused(false), 120)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") submit();
                  if (e.key === "Escape") e.currentTarget.blur();
                }}
              />
              {!query && !searched && (
                <kbd className="kbd">
                  <span>⌘</span>
                  <span>K</span>
                </kbd>
              )}
              {(query || searched) && (
                <button
                  className="clear"
                  onClick={() => (searched ? reset() : setQuery(""))}
                  aria-label="Clear"
                >
                  <Icon.X width={14} height={14} />
                </button>
              )}
              <RecentDropdown
                visible={focused && !query && !searched && recent.length > 0}
                items={recent}
                onPick={(q) => {
                  setQuery(q);
                  submit(q);
                }}
                onRemove={(q) => {
                  setRecent((prev) => {
                    const next = prev.filter((e) => e.query !== q);
                    if (next.length === prev.length) return prev;
                    persistRecent(next);
                    return next;
                  });
                }}
              />
            </div>

            {searched && (
              <div className="results-meta">
                <span
                  dangerouslySetInnerHTML={{
                    __html: t("search.results_count_template", {
                      count: `<strong>${results.length}</strong>`,
                      query: `<em>${recent.filter((item) => item.query === lastQuery).map((item) => item.label)}</em>`,
                    }),
                  }}
                />
                <span className="results-time">
                  {batch.total > 0
                    ? t("batch.summary_template", {
                        done: batch.done,
                        total: batch.total,
                        active: batch.active,
                      })
                    : autoDownloadingCount != null
                      ? t("search.results_pending")
                      : pollMs > 0
                        ? t("search.results_time_template", {
                            ms: (pollMs / 1000).toFixed(2),
                          })
                        : t("search.results_pending")}
                </span>
              </div>
            )}

            {searched && (
              <div className="results">
                {cards.length === 0 ? (
                  <EmptyHint
                    apiKeySet={!noApiKey && !!settings?.youtubeApiKey}
                    t={t}
                  />
                ) : (
                  cards.map((r, i) => (
                    <ResultRow
                      key={r.id}
                      r={r}
                      idx={i}
                      phase={phaseById.get(r.id)}
                      onClick={() => onResultClick(r.id)}
                    />
                  ))
                )}
              </div>
            )}
          </div>
        </main>

        {settings && (
          <Settings
            open={settingsOpen}
            onClose={() => setSettingsOpen(false)}
            settings={settings}
            setSettings={setSettingsUpdater}
          />
        )}
      </div>
    </I18nContext.Provider>
  );
}

function EmptyHint({
  apiKeySet,
  t,
}: {
  apiKeySet: boolean;
  t: (key: string) => string;
}) {
  return (
    <div className="results-meta" style={{ justifyContent: "center" }}>
      {apiKeySet ? t("search.empty_with_key") : t("search.empty_no_key")}
    </div>
  );
}

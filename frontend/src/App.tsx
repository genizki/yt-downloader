import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Icon } from "./components/Icons";
import { Settings } from "./components/Settings";
import { QueueButton } from "./components/Queue";
import { ResultRow } from "./components/Results";
import { RecentDropdown } from "./components/RecentDropdown";
import {
  AppSettings,
  Phase,
  QueueRow,
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

function loadRecent(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed)
      ? parsed.filter((x) => typeof x === "string")
      : [];
  } catch {
    return [];
  }
}

function pushRecent(list: string[], q: string): string[] {
  const next = [q, ...list.filter((x) => x !== q)].slice(0, MAX_RECENT);
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(next));
  } catch {
    /* ignore quota errors */
  }
  return next;
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

function buildQueue(
  results: YouTubeVideo[],
  phases: VideoPhase[],
  settings: AppSettings | null,
): QueueRow[] {
  const phaseById = new Map(phases.map((p) => [p.id, p.phase]));
  const titleById = new Map(results.map((v) => [v.id, v.title]));
  const fmtLabel = settings ? `${settings.format} ${settings.quality}` : "";
  const rows: QueueRow[] = [];
  for (const { id, phase } of phases) {
    const title = titleById.get(id) ?? id;
    const progress = phase.kind === "downloading" ? phase.progress : 0;
    rows.push({ id, title, state: phase.kind, progress, format: fmtLabel });
  }
  // Stable order: downloading first, then queued, then terminal states.
  const rank = (s: QueueRow["state"]) =>
    s === "downloading"
      ? 0
      : s === "queued"
        ? 1
        : s === "post_processing" || s === "moving"
          ? 2
          : 3;
  rows.sort((a, b) => rank(a.state) - rank(b.state));
  void phaseById; // silence unused-var lint; map kept for future per-id lookups
  return rows;
}

export default function App() {
  const [query, setQuery] = useState("");
  const [searched, setSearched] = useState(false);
  const [focused, setFocused] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [queueOpen, setQueueOpen] = useState(false);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [results, setResults] = useState<YouTubeVideo[]>([]);
  const [phases, setPhases] = useState<VideoPhase[]>([]);
  const [lastQuery, setLastQuery] = useState("");
  const [pollMs, setPollMs] = useState<number>(0); // synthetic "search took X ms"
  const [recent, setRecent] = useState<string[]>(() => loadRecent());
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

  // ── Poll backend for search/progress state ──────────────────────────────
  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      if (cancelled) return;
      try {
        const snap = await api.poll();
        if (cancelled) return;
        setResults(snap.results);
        setPhases(snap.phases);
        setSearched(snap.searched);
        setLastQuery(snap.lastQuery);
        if (
          snap.searched &&
          searchStartRef.current != null &&
          snap.results.length > 0
        ) {
          setPollMs(Date.now() - searchStartRef.current);
          searchStartRef.current = null;
        }
      } catch (e) {
        console.error("poll failed", e);
      }
    };
    tick();
    const id = setInterval(tick, 300);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

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
      setRecent((prev) => pushRecent(prev, value));
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
  const queueItems = useMemo<QueueRow[]>(
    () => buildQueue(results, phases, settings),
    [results, phases, settings],
  );

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
            <QueueButton
              items={queueItems}
              open={queueOpen}
              setOpen={setQueueOpen}
            />
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
              />
            </div>

            {searched && (
              <div className="results-meta">
                <span
                  dangerouslySetInnerHTML={{
                    __html: t("search.results_count_template", {
                      count: `<strong>${results.length}</strong>`,
                      query: `<em>${lastQuery}</em>`,
                    }),
                  }}
                />
                <span className="results-time">
                  {pollMs > 0
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
                  <EmptyHint apiKeySet={!!settings?.youtubeApiKey} t={t} />
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

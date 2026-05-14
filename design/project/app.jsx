const { useState, useEffect, useRef, useCallback, useMemo } = React;

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "blur": 22,
  "tint": 0.55,
  "radius": 26,
  "saturation": 180,
  "bloom": 1,
  "hue": 30,
  "breathe": true,
  "breatheSpeed": 8
}/*EDITMODE-END*/;

// ----- Mock data -----
const MOCK_RESULTS = [
  { id: "v1", title: "Building a minimalist Linux setup from scratch — i3wm, neovim, tmux walkthrough", author: "Distrotube", duration: "1:24:07", views: "284K views", posted: "3 weeks ago", hue: 28 },
  { id: "v2", title: "How submarines actually navigate underwater (and why GPS doesn't work)", author: "Real Engineering", duration: "0:18:42", views: "2.1M views", posted: "5 months ago", hue: 220 },
  { id: "v3", title: "Lo-fi study mix — slow piano & vinyl crackle for deep focus sessions", author: "Chillhop Music", duration: "2:47:12", views: "8.7M views", posted: "1 year ago", hue: 340 },
  { id: "v4", title: "I rebuilt my woodworking shop in a 12x16 shed — full tour & layout", author: "Foureyes Furniture", duration: "0:32:18", views: "612K views", posted: "2 months ago", hue: 90 },
  { id: "v5", title: "Why every modern keyboard has the same exact layout (and the few that don't)", author: "Technology Connections", duration: "0:41:55", views: "1.4M views", posted: "6 days ago", hue: 165 },
];

const RECENT_SEARCHES = [
  "lofi study mix 1 hour",
  "linux ricing 2025",
  "how submarines work",
  "woodworking shop tour",
];

const QUEUE_ITEMS = [
  { id: "q1", title: "How submarines actually navigate underwater", state: "downloading", progress: 0.62, format: "MP4 1080p" },
  { id: "q2", title: "Lo-fi study mix — slow piano & vinyl crackle", state: "downloading", progress: 0.18, format: "MP3 320" },
  { id: "q3", title: "Building a minimalist Linux setup from scratch", state: "queued", progress: 0, format: "MP4 720p" },
];

// Audio formats — toggling any of these greys out video settings
const AUDIO_FORMATS = ["MP3", "M4A", "AAC", "ALAC", "AIFF", "FLAC"];

// Fixed-length mask used when a token is stored — never leak the real length
const TOKEN_MASK = "••••••••••••••••••••••••";

// Boolean toggles in the Extras section
const EXTRAS = [
  { key: "embedThumbnail",  label: "Embed thumbnail",     hint: "Attach the video's thumbnail as cover art." },
  { key: "embedMetadata",   label: "Embed metadata",      hint: "Write title, uploader, and description into the file." },
  { key: "embedChapters",   label: "Embed chapters",      hint: "Include chapter markers from the source." },
  { key: "embedSubtitles",  label: "Embed subtitles",     hint: "Mux available subtitle tracks into the container." },
  { key: "writeSubtitles",  label: "Write subtitles to file", hint: "Save subtitles as a separate .srt/.vtt file." },
  { key: "skipPlaylists",   label: "Skip playlists",      hint: "Download single video only, ignore playlist URLs." },
  { key: "restrictNames",   label: "Restrict filenames",  hint: "ASCII only, no spaces or special characters." },
];

// ----- Thumbnail placeholder -----
function Thumb({ hue, duration }) {
  const bg = `oklch(0.92 0.02 ${hue})`;
  const stripe = `oklch(0.86 0.03 ${hue})`;
  return (
    <div className="thumb">
      <svg viewBox="0 0 160 90" preserveAspectRatio="none" className="thumb-svg">
        <defs>
          <pattern id={`p-${hue}`} width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(35)">
            <rect width="6" height="6" fill={bg} />
            <line x1="0" y1="0" x2="0" y2="6" stroke={stripe} strokeWidth="1.5" />
          </pattern>
        </defs>
        <rect width="160" height="90" fill={`url(#p-${hue})`} />
      </svg>
      <span className="thumb-duration">{duration}</span>
    </div>
  );
}

// ----- Icons -----
const Icon = {
  Gear: (p) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  ),
  Search: (p) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.5-3.5" />
    </svg>
  ),
  X: (p) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M18 6 6 18M6 6l12 12" />
    </svg>
  ),
  Clock: (p) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3 2" />
    </svg>
  ),
  Download: (p) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M12 4v11m0 0 4-4m-4 4-4-4" />
      <path d="M4 19h16" />
    </svg>
  ),
  Stack: (p) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="m3 7 9-4 9 4-9 4-9-4Z" />
      <path d="m3 12 9 4 9-4" />
      <path d="m3 17 9 4 9-4" />
    </svg>
  ),
  Eye: (p) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  ),
  EyeOff: (p) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c6.5 0 10 7 10 7a17.3 17.3 0 0 1-3.06 4.04" />
      <path d="M6.6 6.6A17.4 17.4 0 0 0 2 11s3.5 7 10 7a9.5 9.5 0 0 0 5-1.4" />
      <path d="M9.88 9.88a3 3 0 0 0 4.24 4.24" />
      <path d="m2 2 20 20" />
    </svg>
  ),
};

// ----- Toggle (boolean switch) -----
function Toggle({ value, onChange, ariaLabel }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={value}
      aria-label={ariaLabel}
      className={`tgl ${value ? "tgl--on" : ""}`}
      onClick={() => onChange(!value)}
    >
      <span className="tgl-thumb" />
    </button>
  );
}

// ----- Token input — masked, click-to-edit, eye toggle -----
function TokenInput({ value, onChange }) {
  const [editing, setEditing] = useState(false);
  const [revealed, setRevealed] = useState(false);
  const ref = useRef(null);

  // When entering edit mode, focus the real input
  useEffect(() => {
    if (editing && ref.current) ref.current.focus();
  }, [editing]);

  const hasValue = value.length > 0;
  // Display logic:
  //   editing + revealed → real text, editable
  //   editing + masked   → password input, editable (caret visible, dots)
  //   !editing + has value → static dot mask of fixed length (no length leak)
  //   !editing + empty   → placeholder shown via real input field

  if (!editing) {
    return (
      <div className="token">
        <button
          type="button"
          className="token-display"
          onClick={() => setEditing(true)}
          aria-label="Edit token"
        >
          {hasValue ? (
            <span className="token-mask">{revealed ? value : TOKEN_MASK}</span>
          ) : (
            <span className="token-placeholder">Paste token here</span>
          )}
        </button>
        <button
          type="button"
          className="token-eye"
          onClick={() => setRevealed((r) => !r)}
          disabled={!hasValue}
          title={revealed ? "Hide token" : "Show token"}
          aria-label={revealed ? "Hide token" : "Show token"}
          aria-pressed={revealed}
        >
          {revealed ? <Icon.EyeOff width="16" height="16" /> : <Icon.Eye width="16" height="16" />}
        </button>
      </div>
    );
  }

  return (
    <div className="token token--editing">
      <input
        ref={ref}
        className="token-input"
        type={revealed ? "text" : "password"}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onBlur={() => setEditing(false)}
        onKeyDown={(e) => { if (e.key === "Escape" || e.key === "Enter") e.currentTarget.blur(); }}
        placeholder="Paste token here"
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="off"
        spellCheck={false}
        data-1p-ignore="true"
        data-lpignore="true"
        name="yt-dlp-auth-token"
      />
      <button
        type="button"
        className="token-eye"
        onMouseDown={(e) => e.preventDefault()} /* keep input focus */
        onClick={() => setRevealed((r) => !r)}
        title={revealed ? "Hide token" : "Show token"}
        aria-label={revealed ? "Hide token" : "Show token"}
        aria-pressed={revealed}
      >
        {revealed ? <Icon.EyeOff width="16" height="16" /> : <Icon.Eye width="16" height="16" />}
      </button>
    </div>
  );
}

// ----- Settings overlay -----
function Settings({ open, onClose, settings, setSettings }) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  const set = (k) => (v) => setSettings((s) => ({ ...s, [k]: v }));
  const isAudioOnly = AUDIO_FORMATS.includes(settings.format);

  return (
    <div className={`settings ${open ? "settings--open" : ""}`} aria-hidden={!open}>
      <header className="settings-header">
        <div>
          <div className="settings-eyebrow">Preferences</div>
          <h2 className="settings-title">Settings</h2>
        </div>
        <button className="icon-btn" onClick={onClose} aria-label="Close settings">
          <Icon.X width="18" height="18" />
          <span className="kbd-tiny">Esc</span>
        </button>
      </header>

      <div className="settings-body">
        <Section label="Downloads">
          <Row label="Format" hint="Container used when saving the file. Audio-only formats disable video options.">
            <Segmented
              value={settings.format}
              onChange={set("format")}
              options={["MP4", "MKV", "WebM", "MOV", "MP3", "M4A", "AAC", "ALAC", "AIFF", "FLAC"]}
              accents={AUDIO_FORMATS}
            />
          </Row>
          <Row
            label="Video quality"
            hint={isAudioOnly ? "Disabled — current format is audio-only" : "Maximum resolution to fetch"}
            disabled={isAudioOnly}
          >
            <Segmented
              value={settings.quality}
              onChange={set("quality")}
              options={["360p", "720p", "1080p", "1440p", "2160p"]}
              disabled={isAudioOnly}
            />
          </Row>
          <Row
            label="Video codec"
            hint={isAudioOnly ? "Disabled — current format is audio-only" : "Preferred video codec"}
            disabled={isAudioOnly}
          >
            <Segmented
              value={settings.codec}
              onChange={set("codec")}
              options={["H.264", "H.265", "VP9", "AV1"]}
              disabled={isAudioOnly}
            />
          </Row>
          <Row label="Audio quality" hint="Bitrate target for audio">
            <Segmented
              value={settings.audio}
              onChange={set("audio")}
              options={["96 kbps", "128 kbps", "192 kbps", "256 kbps", "320 kbps"]}
            />
          </Row>
          <Row label="Download path" hint="Files will be saved here">
            <input
              className="text-input"
              type="text"
              value={settings.path}
              onChange={(e) => set("path")(e.target.value)}
            />
          </Row>
        </Section>

        <Section label="Constraints">
          <Row label="Maximum file size" hint="Skip downloads larger than this. Useful on metered connections.">
            <Segmented
              value={settings.maxSize}
              onChange={set("maxSize")}
              options={["No limit", "50 MB", "100 MB", "500 MB", "1 GB"]}
            />
          </Row>
          <Row label="Streaming protocol" hint="Transport used to fetch the media stream.">
            <Segmented
              value={settings.protocol}
              onChange={set("protocol")}
              options={["Auto", "HTTPS", "HTTP", "HLS (m3u8)", "DASH"]}
            />
          </Row>
        </Section>

        <Section label="Extras">
          <div className="extras-grid">
            {EXTRAS.map((x) => (
              <div key={x.key} className="extra">
                <div className="extra-text">
                  <div className="extra-label">{x.label}</div>
                  <div className="extra-hint">{x.hint}</div>
                </div>
                <Toggle
                  value={!!settings.extras[x.key]}
                  onChange={(v) => setSettings((s) => ({ ...s, extras: { ...s.extras, [x.key]: v } }))}
                  ariaLabel={x.label}
                />
              </div>
            ))}
          </div>
        </Section>

        <Section label="Appearance">
          <Row label="Theme" hint="Currently only light is implemented">
            <Segmented value={settings.theme} onChange={set("theme")} options={["Light", "Dark", "System"]} />
          </Row>
          <Row label="Language" hint="Interface language">
            <Segmented
              value={settings.language}
              onChange={set("language")}
              options={["English", "Deutsch", "Français", "日本語"]}
            />
          </Row>
        </Section>

        <div className="auth-divider" role="separator" aria-hidden="true" />

        <Section label="Authentication">
          <div className="auth-intro">
            Optional. Provide a YouTube/Google access token to access private,
            age-restricted, or members-only content.
          </div>
          <Row
            label="YouTube / Google access token"
            hint={
              <>
                <a
                  className="auth-help"
                  href="https://github.com/yt-dlp/yt-dlp/wiki/Extractors#exporting-youtube-cookies"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  How to get a token?
                </a>
              </>
            }
            full
          >
            <TokenInput
              value={settings.authToken}
              onChange={(v) => setSettings((s) => ({ ...s, authToken: v }))}
            />
          </Row>
          <Row label="" hint="" full>
            <button
              type="button"
              className="btn-clear"
              disabled={!settings.authToken}
              onClick={() => setSettings((s) => ({ ...s, authToken: "" }))}
            >
              Clear token
            </button>
          </Row>
        </Section>
      </div>

      <footer className="settings-footer">
        <span className="settings-meta">v0.1.0 · build 24a91f</span>
        <button className="btn-primary" onClick={onClose}>Done</button>
      </footer>
    </div>
  );
}

function Section({ label, children }) {
  return (
    <section className="sec">
      <div className="sec-label">{label}</div>
      <div className="sec-rows">{children}</div>
    </section>
  );
}

function Row({ label, hint, children, disabled, full }) {
  return (
    <div className={`row ${disabled ? "row--disabled" : ""} ${full ? "row--full" : ""}`}>
      {!full && (
        <div className="row-text">
          <div className="row-label">{label}</div>
          {hint && <div className="row-hint">{hint}</div>}
        </div>
      )}
      {full && (label || hint) && (
        <div className="row-text row-text--full">
          {label && <div className="row-label">{label}</div>}
          {hint && <div className="row-hint">{hint}</div>}
        </div>
      )}
      <div className={`row-control ${full ? "row-control--full" : ""}`}>{children}</div>
    </div>
  );
}

function Segmented({ value, onChange, options, disabled, accents }) {
  return (
    <div className={`seg ${disabled ? "seg--disabled" : ""}`}>
      {options.map((o) => (
        <button
          key={o}
          disabled={disabled}
          className={`seg-btn ${value === o ? "seg-btn--on" : ""} ${accents && accents.includes(o) ? "seg-btn--accent" : ""}`}
          onClick={() => !disabled && onChange(o)}
        >
          {o}
        </button>
      ))}
    </div>
  );
}

// ----- Search results -----
function ResultRow({ r, idx }) {
  return (
    <div className="result" style={{ animationDelay: `${80 + idx * 55}ms` }}>
      <Thumb hue={r.hue} duration={r.duration} />
      <div className="result-text">
        <h3 className="result-title">{r.title}</h3>
        <div className="result-author">{r.author}</div>
        <div className="result-meta">
          <span className="result-duration">{r.duration}</span>
          <span className="result-dot">·</span>
          <span>{r.views}</span>
          <span className="result-dot">·</span>
          <span>{r.posted}</span>
        </div>
      </div>
    </div>
  );
}

// ----- Recent searches dropdown -----
function RecentDropdown({ visible, items, onPick }) {
  if (!visible) return null;
  return (
    <div className="recent">
      <div className="recent-label">Recent</div>
      {items.map((q) => (
        <button key={q} className="recent-item" onMouseDown={(e) => { e.preventDefault(); onPick(q); }}>
          <Icon.Clock width="14" height="14" />
          <span>{q}</span>
        </button>
      ))}
    </div>
  );
}

// ----- Queue button + popover (top-left) -----
function QueueButton({ items, open, setOpen }) {
  const ref = useRef(null);
  useEffect(() => {
    if (!open) return;
    const onClick = (e) => { if (ref.current && !ref.current.contains(e.target)) setOpen(false); };
    const onKey = (e) => e.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", onClick);
    window.addEventListener("keydown", onKey);
    return () => { document.removeEventListener("mousedown", onClick); window.removeEventListener("keydown", onKey); };
  }, [open, setOpen]);

  const active = items.filter((i) => i.state === "downloading").length;

  return (
    <div className="queue-wrap" ref={ref}>
      <button
        className={`queue-btn ${open ? "queue-btn--on" : ""}`}
        onClick={() => setOpen(!open)}
        aria-label="Toggle download queue"
      >
        <Icon.Stack width="16" height="16" />
        <span className="queue-btn-label">Queue</span>
        {active > 0 && <span className="queue-btn-badge">{active}</span>}
      </button>
      {open && (
        <div className="queue-pop">
          <div className="queue-head">
            <div className="queue-eyebrow">Download Queue</div>
            <span className="queue-count">{items.length}</span>
          </div>
          <div className="queue-list">
            {items.map((q) => (
              <div key={q.id} className="qitem">
                <div className="qitem-row">
                  <div className="qitem-title">{q.title}</div>
                  <div className="qitem-format">{q.format}</div>
                </div>
                {q.state === "downloading" ? (
                  <>
                    <div className="qbar"><div className="qbar-fill" style={{ width: `${q.progress * 100}%` }} /></div>
                    <div className="qitem-meta">
                      <span>{Math.round(q.progress * 100)}%</span>
                      <span className="qstate">downloading</span>
                    </div>
                  </>
                ) : (
                  <div className="qitem-meta">
                    <span>—</span>
                    <span className="qstate qstate--queued">queued</span>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ----- App -----
function App() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);
  useEffect(() => {
    const r = document.documentElement.style;
    r.setProperty("--tw-blur", `${t.blur}px`);
    r.setProperty("--tw-tint", String(t.tint));
    r.setProperty("--tw-radius", `${t.radius}px`);
    r.setProperty("--tw-saturation", `${t.saturation}%`);
    r.setProperty("--tw-bloom-strength", String(t.bloom));
    r.setProperty("--tw-hue", String(t.hue));
    r.setProperty("--tw-breathe-duration", `${t.breatheSpeed}s`);
  }, [t]);

  const [query, setQuery] = useState("");
  const [searched, setSearched] = useState(false);
  const [focused, setFocused] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [queueOpen, setQueueOpen] = useState(false);
  const [settings, setSettings] = useState({
    format: "MP4",
    quality: "1080p",
    codec: "H.264",
    audio: "256 kbps",
    path: "~/Downloads/yt-dlp",
    maxSize: "No limit",
    protocol: "Auto",
    extras: {
      embedThumbnail: true,
      embedMetadata: true,
      embedChapters: false,
      embedSubtitles: false,
      writeSubtitles: false,
      skipPlaylists: false,
      restrictNames: false,
    },
    theme: "Light",
    language: "English",
    authToken: "",
  });
  const inputRef = useRef(null);

  useEffect(() => {
    const onKey = (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Freeze background bloom once a search is active OR if the user disabled breathe
  useEffect(() => {
    const stop = searched || !t.breathe;
    document.body.classList.toggle("is-static", stop);
  }, [searched, t.breathe]);

  const submit = useCallback((q) => {
    const value = (q ?? query).trim();
    if (!value) return;
    setQuery(value);
    setSearched(true);
    inputRef.current?.blur();
  }, [query]);

  const reset = () => {
    setSearched(false);
    setQuery("");
    setTimeout(() => inputRef.current?.focus(), 250);
  };

  return (
    <div className="app">
      <TweaksPanel title="Liquid Glass">
        <TweakSection label="Glass" />
        <TweakSlider label="Blur" value={t.blur} min={0} max={60} unit="px" onChange={(v) => setTweak("blur", v)} />
        <TweakSlider label="Tint" value={t.tint} min={0} max={1} step={0.05} onChange={(v) => setTweak("tint", v)} />
        <TweakSlider label="Saturation" value={t.saturation} min={100} max={260} unit="%" onChange={(v) => setTweak("saturation", v)} />
        <TweakSlider label="Radius" value={t.radius} min={8} max={40} unit="px" onChange={(v) => setTweak("radius", v)} />
        <TweakSection label="Ambient bloom" />
        <TweakSlider label="Hue" value={t.hue} min={0} max={360} unit="°" onChange={(v) => setTweak("hue", v)} />
        <TweakSlider label="Bloom" value={t.bloom} min={0} max={1.6} step={0.05} onChange={(v) => setTweak("bloom", v)} />
        <TweakToggle label="Breathing" value={t.breathe} onChange={(v) => setTweak("breathe", v)} />
        <TweakSlider label="Breath cycle" value={t.breatheSpeed} min={4} max={20} unit="s" onChange={(v) => setTweak("breatheSpeed", v)} />
      </TweaksPanel>

      <header className="topbar">
        <div className="topbar-left">
          <button
            className={`brand ${searched ? "brand--visible" : ""}`}
            onClick={reset}
            aria-label="New search"
          >
            <span className="brand-mark" />
            <span className="brand-name">yt-dlp</span>
          </button>
          <QueueButton items={QUEUE_ITEMS} open={queueOpen} setOpen={setQueueOpen} />
        </div>
        <button className="icon-btn gear" onClick={() => setSettingsOpen(true)} aria-label="Open settings">
          <Icon.Gear width="18" height="18" />
        </button>
      </header>

      <main className={`stage ${searched ? "stage--searched" : ""}`}>
        <div className="search-wrap">
          {!searched && (
            <div className="hero">
              <div className="hero-eyebrow">yt-dlp</div>
              <h1 className="hero-title">What do you want to download?</h1>
            </div>
          )}

          <div className={`searchbar ${focused ? "searchbar--focused" : ""}`}>
            <Icon.Search className="searchbar-icon" width="18" height="18" />
            <input
              ref={inputRef}
              className="searchbar-input"
              type="text"
              placeholder="Start typing for search"
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
              <kbd className="kbd"><span>⌘</span><span>K</span></kbd>
            )}
            {query && (
              <button className="clear" onClick={() => setQuery("")} aria-label="Clear">
                <Icon.X width="14" height="14" />
              </button>
            )}
            <RecentDropdown
              visible={focused && !query && !searched}
              items={RECENT_SEARCHES}
              onPick={(q) => { setQuery(q); submit(q); }}
            />
          </div>

          {searched && (
            <div className="results-meta">
              <span>About <strong>{MOCK_RESULTS.length}</strong> results for <em>"{query}"</em></span>
              <span className="results-time">in 0.21s</span>
            </div>
          )}

          {searched && (
            <div className="results">
              {MOCK_RESULTS.map((r, i) => <ResultRow key={r.id} r={r} idx={i} />)}
            </div>
          )}
        </div>
      </main>

      <Settings
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        settings={settings}
        setSettings={setSettings}
      />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App />);

import { ReactNode, useEffect } from "react";
import { Icon } from "./Icons";
import { Toggle } from "./Toggle";
import { Segmented } from "./Segmented";
import { TokenInput } from "./TokenInput";
import { AppSettings, AUDIO_FORMATS, LANGUAGE_LABELS } from "../types";
import { pickDownloadDir } from "../api";
import { useT } from "../i18n";

const EXTRAS: { key: keyof AppSettings["extras"]; tkey: string }[] = [
  { key: "embedThumbnail", tkey: "extras.embedThumbnail" },
  { key: "embedMetadata", tkey: "extras.embedMetadata" },
  { key: "embedChapters", tkey: "extras.embedChapters" },
  { key: "embedSubtitles", tkey: "extras.embedSubtitles" },
  { key: "writeSubtitles", tkey: "extras.writeSubtitles" },
  { key: "skipPlaylists", tkey: "extras.skipPlaylists" },
  { key: "restrictNames", tkey: "extras.restrictNames" },
];

function parseOptionalNumber(value: string): number | undefined {
  if (!value.trim()) return undefined;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function parseCsv(value: string): string[] {
  return value
    .split(",")
    .map((x) => x.trim())
    .filter(Boolean);
}

interface Props {
  open: boolean;
  onClose: () => void;
  settings: AppSettings;
  setSettings: (updater: (s: AppSettings) => AppSettings) => void;
}

export function Settings({ open, onClose, settings, setSettings }: Props) {
  const t = useT();

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  const set =
    <K extends keyof AppSettings>(k: K) =>
    (v: AppSettings[K]) =>
      setSettings((s) => ({ ...s, [k]: v }));
  const setPlaylist =
    <K extends keyof AppSettings["playlist"]>(k: K) =>
    (v: AppSettings["playlist"][K]) =>
      setSettings((s) => ({ ...s, playlist: { ...s.playlist, [k]: v } }));
  const setNetwork =
    <K extends keyof AppSettings["network"]>(k: K) =>
    (v: AppSettings["network"][K]) =>
      setSettings((s) => ({ ...s, network: { ...s.network, [k]: v } }));
  const setMetadataExtras =
    <K extends keyof AppSettings["metadataExtras"]>(k: K) =>
    (v: AppSettings["metadataExtras"][K]) =>
      setSettings((s) => ({
        ...s,
        metadataExtras: { ...s.metadataExtras, [k]: v },
      }));
  const setSubtitles =
    <K extends keyof AppSettings["subtitles"]>(k: K) =>
    (v: AppSettings["subtitles"][K]) =>
      setSettings((s) => ({ ...s, subtitles: { ...s.subtitles, [k]: v } }));
  const setPostProcessing =
    <K extends keyof AppSettings["postProcessing"]>(k: K) =>
    (v: AppSettings["postProcessing"][K]) =>
      setSettings((s) => ({
        ...s,
        postProcessing: { ...s.postProcessing, [k]: v },
      }));
  const setMisc =
    <K extends keyof AppSettings["misc"]>(k: K) =>
    (v: AppSettings["misc"][K]) =>
      setSettings((s) => ({ ...s, misc: { ...s.misc, [k]: v } }));
  const isAudioOnly = AUDIO_FORMATS.includes(settings.format);

  return (
    <div className={`settings ${open ? "settings--open" : ""}`} aria-hidden={!open}>
      <header className="settings-header">
        <div>
          <div className="settings-eyebrow">{t("settings.eyebrow")}</div>
          <h2 className="settings-title">{t("settings.title")}</h2>
        </div>
        <button className="icon-btn" onClick={onClose} aria-label={t("settings.close")}>
          <Icon.X width={18} height={18} />
          <span className="kbd-tiny">Esc</span>
        </button>
      </header>

      <div className="settings-body">
        <Section label={t("settings.section.downloads")}>
          <Row
            label={t("settings.format.label")}
            hint={t("settings.format.hint")}
          >
            <Segmented
              value={settings.format}
              onChange={set("format")}
              options={["MP4", "MKV", "WebM", "MOV", "MP3", "M4A", "AAC", "ALAC", "AIFF", "FLAC"]}
              accents={AUDIO_FORMATS}
            />
          </Row>
          <Row
            label={t("settings.video_quality.label")}
            hint={
              isAudioOnly
                ? t("settings.video_quality.disabled")
                : t("settings.video_quality.hint")
            }
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
            label={t("settings.video_codec.label")}
            hint={
              isAudioOnly
                ? t("settings.video_codec.disabled")
                : t("settings.video_codec.hint")
            }
            disabled={isAudioOnly}
          >
            <Segmented
              value={settings.codec}
              onChange={set("codec")}
              options={["H.264", "H.265", "VP9", "AV1"]}
              disabled={isAudioOnly}
            />
          </Row>
          <Row
            label={t("settings.audio_quality.label")}
            hint={t("settings.audio_quality.hint")}
          >
            <Segmented
              value={settings.audioBitrate}
              onChange={set("audioBitrate")}
              options={["96 kbps", "128 kbps", "192 kbps", "256 kbps", "320 kbps"]}
            />
          </Row>
          <Row
            label={t("settings.download_path.label")}
            hint={t("settings.download_path.hint")}
          >
            <div className="path-picker">
              <input
                className="text-input path-display"
                type="text"
                readOnly
                value={settings.downloadPath}
              />
              <button
                type="button"
                className="btn-primary"
                onClick={async () => {
                  const picked = await pickDownloadDir(settings.downloadPath);
                  if (picked) set("downloadPath")(picked);
                }}
              >
                {t("settings.download_path.browse")}
              </button>
            </div>
          </Row>
        </Section>

        <Section label={t("settings.section.constraints")}>
          <Row
            label={t("settings.max_size.label")}
            hint={t("settings.max_size.hint")}
          >
            <Segmented
              value={settings.maxSize}
              onChange={set("maxSize")}
              options={["No limit", "50 MB", "100 MB", "500 MB", "1 GB"]}
            />
          </Row>
          <Row
            label={t("settings.protocol.label")}
            hint={t("settings.protocol.hint")}
          >
            <Segmented
              value={settings.protocol}
              onChange={set("protocol")}
              options={["Auto", "HTTPS", "HTTP", "HLS", "DASH"]}
            />
          </Row>
        </Section>

        <Section label={t("settings.section.extras")}>
          <div className="extras-grid">
            {EXTRAS.map((x) => {
              const label = t(`${x.tkey}.label`);
              const hint = t(`${x.tkey}.hint`);
              return (
                <div key={x.key} className="extra">
                  <div className="extra-text">
                    <div className="extra-label">{label}</div>
                    <div className="extra-hint">{hint}</div>
                  </div>
                  <Toggle
                    value={!!settings.extras[x.key]}
                    onChange={(v) =>
                      setSettings((s) => ({ ...s, extras: { ...s.extras, [x.key]: v } }))
                    }
                    ariaLabel={label}
                  />
                </div>
              );
            })}
          </div>
        </Section>

        <Section label="Erweiterte Optionen">
          <AccordionGroup label="Playlist">
            <Row
              label="Modus"
              hint="Einzelvideo oder komplette Playlist verarbeiten"
            >
              <Segmented
                value={settings.playlist.mode}
                onChange={(v) =>
                  setPlaylist("mode")(v === "Playlist" ? "Playlist" : "Single")
                }
                options={["Single", "Playlist"]}
              />
            </Row>
            <Row label="Startindex" hint="Ab welchem Playlist-Eintrag gestartet wird">
              <input
                className="text-input"
                type="number"
                value={settings.playlist.playlistStart ?? ""}
                onChange={(e) =>
                  setPlaylist("playlistStart")(parseOptionalNumber(e.currentTarget.value))
                }
              />
            </Row>
            <Row label="Endindex" hint="Bis zu welchem Playlist-Eintrag geladen wird">
              <input
                className="text-input"
                type="number"
                value={settings.playlist.playlistEnd ?? ""}
                onChange={(e) =>
                  setPlaylist("playlistEnd")(parseOptionalNumber(e.currentTarget.value))
                }
              />
            </Row>
            <Row
              label="Playlist Items"
              hint="Freier yt-dlp Ausdruck, z.B. 1,3,5-9"
            >
              <input
                className="text-input"
                type="text"
                value={settings.playlist.playlistItems ?? ""}
                onChange={(e) => setPlaylist("playlistItems")(e.currentTarget.value || undefined)}
              />
            </Row>
            <Row label="Max Downloads" hint="Maximale Anzahl an Downloads">
              <input
                className="text-input"
                type="number"
                value={settings.playlist.maxDownloads ?? ""}
                onChange={(e) =>
                  setPlaylist("maxDownloads")(parseOptionalNumber(e.currentTarget.value))
                }
              />
            </Row>
            <Row label="Titel-Match" hint="Nur Titel, die diesem Muster entsprechen">
              <input
                className="text-input"
                type="text"
                value={settings.playlist.matchTitle ?? ""}
                onChange={(e) => setPlaylist("matchTitle")(e.currentTarget.value || undefined)}
              />
            </Row>
            <Row label="Titel-Filter (Ausschluss)" hint="Titel-Muster, die ausgeschlossen werden">
              <input
                className="text-input"
                type="text"
                value={settings.playlist.rejectTitle ?? ""}
                onChange={(e) => setPlaylist("rejectTitle")(e.currentTarget.value || undefined)}
              />
            </Row>
            <Row label="Datum" hint="Exaktes Datum (yt-dlp Format, z.B. YYYYMMDD)">
              <input
                className="text-input"
                type="text"
                value={settings.playlist.date ?? ""}
                onChange={(e) => setPlaylist("date")(e.currentTarget.value || undefined)}
              />
            </Row>
            <Row label="Datum vorher" hint="Nur Videos vor diesem Datum">
              <input
                className="text-input"
                type="text"
                value={settings.playlist.datebefore ?? ""}
                onChange={(e) => setPlaylist("datebefore")(e.currentTarget.value || undefined)}
              />
            </Row>
            <Row label="Datum nachher" hint="Nur Videos nach diesem Datum">
              <input
                className="text-input"
                type="text"
                value={settings.playlist.dateafter ?? ""}
                onChange={(e) => setPlaylist("dateafter")(e.currentTarget.value || undefined)}
              />
            </Row>
            <Row label="Minimale Views" hint="Nur Videos mit mindestens X Aufrufen">
              <input
                className="text-input"
                type="number"
                value={settings.playlist.minViews ?? ""}
                onChange={(e) => setPlaylist("minViews")(parseOptionalNumber(e.currentTarget.value))}
              />
            </Row>
          </AccordionGroup>

          <AccordionGroup label="Netzwerk">
            <Row label="Rate Limit" hint="Bandbreitenlimit, z.B. 2M oder 500K">
              <input
                className="text-input"
                type="text"
                value={settings.network.rateLimit ?? ""}
                onChange={(e) => setNetwork("rateLimit")(e.currentTarget.value || undefined)}
              />
            </Row>
            <Row label="Concurrent Fragments" hint="Parallele Fragment-Downloads">
              <input
                className="text-input"
                type="number"
                value={settings.network.concurrentFragments ?? ""}
                onChange={(e) =>
                  setNetwork("concurrentFragments")(parseOptionalNumber(e.currentTarget.value))
                }
              />
            </Row>
            <Row label="Retries" hint="Wiederholungen für den Hauptdownload">
              <input
                className="text-input"
                type="number"
                value={settings.network.retries ?? ""}
                onChange={(e) => setNetwork("retries")(parseOptionalNumber(e.currentTarget.value))}
              />
            </Row>
            <Row label="Fragment Retries" hint="Wiederholungen für Fragmente">
              <input
                className="text-input"
                type="number"
                value={settings.network.fragmentRetries ?? ""}
                onChange={(e) =>
                  setNetwork("fragmentRetries")(parseOptionalNumber(e.currentTarget.value))
                }
              />
            </Row>
            <Row label="Cookies-Quelle" hint="Quelle für Auth-/Session-Cookies">
              <Segmented
                value={settings.network.cookies.kind}
                onChange={(v) =>
                  setNetwork("cookies")(
                    v === "File"
                      ? { kind: "File", path: "" }
                      : v === "Browser"
                        ? { kind: "Browser", browser: "" }
                        : { kind: "None" },
                  )
                }
                options={["None", "File", "Browser"]}
              />
            </Row>
            {settings.network.cookies.kind === "File" && (
              <Row label="Cookie-Datei" hint="Pfad zur Cookie-Datei">
                <input
                  className="text-input"
                  type="text"
                  value={settings.network.cookies.path}
                  onChange={(e) =>
                    setNetwork("cookies")({ kind: "File", path: e.currentTarget.value })
                  }
                />
              </Row>
            )}
            {settings.network.cookies.kind === "Browser" && (
              <Row label="Browser" hint="Browsername für Cookie-Import">
                <input
                  className="text-input"
                  type="text"
                  value={settings.network.cookies.browser}
                  onChange={(e) =>
                    setNetwork("cookies")({
                      kind: "Browser",
                      browser: e.currentTarget.value,
                    })
                  }
                />
              </Row>
            )}
            <Row label="Proxy" hint="Proxy-URL">
              <input
                className="text-input"
                type="text"
                value={settings.network.proxy ?? ""}
                onChange={(e) => setNetwork("proxy")(e.currentTarget.value || undefined)}
              />
            </Row>
            <Row label="Source Address" hint="Quell-IP/Adresse für ausgehende Verbindungen">
              <input
                className="text-input"
                type="text"
                value={settings.network.sourceAddress ?? ""}
                onChange={(e) => setNetwork("sourceAddress")(e.currentTarget.value || undefined)}
              />
            </Row>
            <Row label="Geo Bypass" hint="Regionale Sperren nach Möglichkeit umgehen">
              <Toggle
                value={settings.network.geoBypass}
                onChange={setNetwork("geoBypass")}
                ariaLabel="Geo Bypass"
              />
            </Row>
          </AccordionGroup>

          <AccordionGroup label="Metadata Extras">
            <Row label="Metadaten hinzufügen" hint="Schreibt Medien-Metadaten in die Datei">
              <Toggle
                value={settings.metadataExtras.addMetadata}
                onChange={setMetadataExtras("addMetadata")}
                ariaLabel="Metadaten hinzufügen"
              />
            </Row>
            <Row label="Thumbnail schreiben" hint="Speichert das Vorschaubild lokal">
              <Toggle
                value={settings.metadataExtras.writeThumbnail}
                onChange={setMetadataExtras("writeThumbnail")}
                ariaLabel="Thumbnail schreiben"
              />
            </Row>
            <Row label="Info JSON schreiben" hint="Exportiert zusätzliche Metadaten als JSON">
              <Toggle
                value={settings.metadataExtras.writeInfoJson}
                onChange={setMetadataExtras("writeInfoJson")}
                ariaLabel="Info JSON schreiben"
              />
            </Row>
            <Row label="Beschreibung schreiben" hint="Speichert die Videobeschreibung als Datei">
              <Toggle
                value={settings.metadataExtras.writeDescription}
                onChange={setMetadataExtras("writeDescription")}
                ariaLabel="Beschreibung schreiben"
              />
            </Row>
          </AccordionGroup>

          <AccordionGroup label="Untertitel">
            <Row label="Automatische Untertitel" hint="Lädt automatisch generierte Untertitel">
              <Toggle
                value={settings.subtitles.writeAutoSubs}
                onChange={setSubtitles("writeAutoSubs")}
                ariaLabel="Automatische Untertitel"
              />
            </Row>
            <Row
              label="Sprachen"
              hint="Kommagetrennte Sprachcodes, z.B. de,en.*"
            >
              <input
                className="text-input"
                type="text"
                value={settings.subtitles.subLangs.join(",")}
                onChange={(e) => setSubtitles("subLangs")(parseCsv(e.currentTarget.value))}
              />
            </Row>
            <Row label="Format" hint="Untertitel-Format, z.B. vtt, srt">
              <input
                className="text-input"
                type="text"
                value={settings.subtitles.subFormat ?? ""}
                onChange={(e) => setSubtitles("subFormat")(e.currentTarget.value || undefined)}
              />
            </Row>
          </AccordionGroup>

          <AccordionGroup label="Post Processing">
            <Row label="SponsorBlock Modus" hint="Sponsoring-Segmente entfernen oder markieren">
              <Segmented
                value={settings.postProcessing.sponsorblock.kind}
                onChange={(v) =>
                  setPostProcessing("sponsorblock")(
                    v === "Remove"
                      ? { kind: "Remove", segments: [] }
                      : v === "Mark"
                        ? { kind: "Mark", segments: [] }
                        : { kind: "None" },
                  )
                }
                options={["None", "Remove", "Mark"]}
              />
            </Row>
            {settings.postProcessing.sponsorblock.kind !== "None" && (
              <Row
                label="SponsorBlock Segmente"
                hint="Kommagetrennte Segmentnamen"
              >
                <input
                  className="text-input"
                  type="text"
                  value={settings.postProcessing.sponsorblock.segments.join(",")}
                  onChange={(e) =>
                    setPostProcessing("sponsorblock")({
                      kind: settings.postProcessing.sponsorblock.kind,
                      segments: parseCsv(e.currentTarget.value),
                    })
                  }
                />
              </Row>
            )}
            <Row label="Kapitel trennen" hint="Kapitel als separate Dateien exportieren">
              <Toggle
                value={settings.postProcessing.splitChapters}
                onChange={setPostProcessing("splitChapters")}
                ariaLabel="Kapitel trennen"
              />
            </Row>
            <Row label="Download Sections" hint="Bereiche per yt-dlp Ausdruck begrenzen">
              <input
                className="text-input"
                type="text"
                value={settings.postProcessing.downloadSections ?? ""}
                onChange={(e) =>
                  setPostProcessing("downloadSections")(e.currentTarget.value || undefined)
                }
              />
            </Row>
            <Row label="Exec" hint="Befehl nach Download ausführen">
              <input
                className="text-input"
                type="text"
                value={settings.postProcessing.exec ?? ""}
                onChange={(e) => setPostProcessing("exec")(e.currentTarget.value || undefined)}
              />
            </Row>
            <Row label="FFmpeg Location" hint="Pfad zu ffmpeg/ffprobe">
              <input
                className="text-input"
                type="text"
                value={settings.postProcessing.ffmpegLocation ?? ""}
                onChange={(e) =>
                  setPostProcessing("ffmpegLocation")(e.currentTarget.value || undefined)
                }
              />
            </Row>
            <Row label="Postprocessor Args" hint="Zusätzliche Argumente für Postprozessoren">
              <input
                className="text-input"
                type="text"
                value={settings.postProcessing.postprocessorArgs ?? ""}
                onChange={(e) =>
                  setPostProcessing("postprocessorArgs")(e.currentTarget.value || undefined)
                }
              />
            </Row>
          </AccordionGroup>

          <AccordionGroup label="Misc">
            <Row label="Simulate" hint="Nur simulieren, nicht herunterladen">
              <Toggle
                value={settings.misc.simulate}
                onChange={setMisc("simulate")}
                ariaLabel="Simulate"
              />
            </Row>
            <Row label="Verbose" hint="Ausführliche Logs">
              <Toggle
                value={settings.misc.verbose}
                onChange={setMisc("verbose")}
                ariaLabel="Verbose"
              />
            </Row>
            <Row label="Quiet" hint="Reduzierte Ausgabe">
              <Toggle
                value={settings.misc.quiet}
                onChange={setMisc("quiet")}
                ariaLabel="Quiet"
              />
            </Row>
            <Row label="No Warnings" hint="Warnungen unterdrücken">
              <Toggle
                value={settings.misc.noWarnings}
                onChange={setMisc("noWarnings")}
                ariaLabel="No Warnings"
              />
            </Row>
            <Row label="Sleep Interval" hint="Pause zwischen Requests in Sekunden">
              <input
                className="text-input"
                type="number"
                value={settings.misc.sleepInterval ?? ""}
                onChange={(e) => setMisc("sleepInterval")(parseOptionalNumber(e.currentTarget.value))}
              />
            </Row>
          </AccordionGroup>
        </Section>

        <Section label={t("settings.section.appearance")}>
          <Row label={t("settings.theme.label")} hint={t("settings.theme.hint")}>
            <Segmented
              value={settings.theme}
              onChange={set("theme")}
              options={["Light", "Dark", "System"]}
            />
          </Row>
          <Row label={t("settings.language.label")} hint={t("settings.language.hint")}>
            <Segmented
              value={settings.language}
              onChange={set("language")}
              options={["en", "de", "fr", "ja"]}
              optionLabels={LANGUAGE_LABELS}
            />
          </Row>
        </Section>

        <div className="auth-divider" role="separator" aria-hidden="true" />

        <Section label={t("settings.section.authentication")}>
          <div className="auth-intro">{t("settings.auth.intro")}</div>
          <Row
            label={t("settings.auth.token.label")}
            hint={
              <a
                className="auth-help"
                href="https://github.com/yt-dlp/yt-dlp/wiki/Extractors#exporting-youtube-cookies"
                target="_blank"
                rel="noopener noreferrer"
              >
                {t("settings.auth.help_link")}
              </a>
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
              {t("settings.auth.clear")}
            </button>
          </Row>
        </Section>

        <Section label={t("settings.section.api")}>
          <Row
            label={t("settings.api.key.label")}
            hint={t("settings.api.key.hint")}
            full
          >
            <TokenInput
              value={settings.youtubeApiKey}
              onChange={(v) => setSettings((s) => ({ ...s, youtubeApiKey: v }))}
            />
          </Row>
          <Row
            label={t("settings.api.playlist.label")}
            hint={t("settings.api.playlist.hint")}
          >
            <Toggle
              value={settings.playlistAutoDownload}
              onChange={set("playlistAutoDownload")}
              ariaLabel={t("settings.api.playlist.label")}
            />
          </Row>
        </Section>
      </div>

      <footer className="settings-footer">
        <span className="settings-meta">v0.1.0</span>
        <button className="btn-primary" onClick={onClose}>
          {t("settings.done")}
        </button>
      </footer>
    </div>
  );
}

function AccordionGroup({
  label,
  children,
  defaultOpen = false,
}: {
  label: string;
  children: ReactNode;
  defaultOpen?: boolean;
}) {
  return (
    <details className="sec-accordion" open={defaultOpen}>
      <summary className="sec-accordion__summary">{label}</summary>
      <div className="sec-accordion__content">{children}</div>
    </details>
  );
}

function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="sec">
      <div className="sec-label">{label}</div>
      <div className="sec-rows">{children}</div>
    </section>
  );
}

interface RowProps {
  label: string;
  hint: ReactNode;
  children: ReactNode;
  disabled?: boolean;
  full?: boolean;
}

function Row({ label, hint, children, disabled, full }: RowProps) {
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

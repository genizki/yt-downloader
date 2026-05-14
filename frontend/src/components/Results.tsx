import { Thumb } from "./Thumb";
import { ResultCard, Phase } from "../types";
import { useT } from "../i18n";

interface Props {
  r: ResultCard;
  idx: number;
  phase?: Phase;
  onClick: () => void;
}

export function ResultRow({ r, idx, phase, onClick }: Props) {
  const t = useT();
  const phaseLabel = renderPhase(phase, t);
  return (
    <div
      className="result"
      style={{ animationDelay: `${80 + idx * 55}ms` }}
      onClick={onClick}
      role="button"
      tabIndex={0}
    >
      {/*<Thumb hue={r.hue} duration={r.duration} />*/}
      {/*<Thumb hue={(r.hue, r.thumbnailUrl)} />*/}
      <Thumb hue={r.hue} img={r.thumbnailUrl} duration={r.duration} />
      <div className="result-text">
        <h3 className="result-title">{r.title}</h3>
        <div className="result-author">{r.author}</div>
        <div className="result-meta">
          <span className="result-duration">{r.duration}</span>
          <span className="result-dot">·</span>
          <span>{r.views}</span>
          <span className="result-dot">·</span>
          <span>{r.posted}</span>
          {phaseLabel && (
            <>
              <span className="result-dot">·</span>
              <span className="qstate">{phaseLabel}</span>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

function renderPhase(
  p: Phase | undefined,
  t: (key: string) => string,
): string | null {
  if (!p) return null;
  switch (p.kind) {
    case "queued":
      return t("download.queued");
    case "downloading":
      return `${Math.round(p.progress * 100)}%`;
    case "post_processing":
      return t("download.post_processing");
    case "moving":
      return t("download.moving");
    case "done":
      return t("download.done");
    case "failed":
      return t("download.failed");
  }
}

import { Thumb } from "./Thumb";
import { ResultCard, Phase } from "../types";
import { useT } from "../i18n";

interface Props {
  r: ResultCard;
  idx: number;
  phase?: Phase;
  onClick: () => void;
}

const STATE_KEYS: Record<Exclude<Phase["kind"], "downloading">, string> = {
  queued: "download.queued",
  post_processing: "download.post_processing",
  moving: "download.moving",
  done: "download.done",
  failed: "download.failed",
};

export function ResultRow({ r, idx, phase, onClick }: Props) {
  const t = useT();
  return (
    <div
      className={`result ${phase ? `result--${phase.kind}` : ""}`}
      style={{ animationDelay: `${80 + idx * 55}ms` }}
      onClick={onClick}
      role="button"
      tabIndex={0}
    >
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
        </div>
        {phase && <PhaseIndicator phase={phase} t={t} />}
      </div>
    </div>
  );
}

function PhaseIndicator({
  phase,
  t,
}: {
  phase: Phase;
  t: (key: string) => string;
}) {
  if (phase.kind === "downloading") {
    const pct = Math.round(phase.progress * 100);
    return (
      <div className="result-phase">
        <div className="qbar">
          <div className="qbar-fill" style={{ width: `${pct}%` }} />
        </div>
        <div className="result-phase-meta">
          <span>{pct}%</span>
          <span className="qstate">{t("download.downloading")}</span>
        </div>
      </div>
    );
  }
  return (
    <div className="result-phase">
      <div className="result-phase-meta">
        <span>—</span>
        <span className={`qstate qstate--${phase.kind}`}>
          {t(STATE_KEYS[phase.kind])}
        </span>
      </div>
    </div>
  );
}

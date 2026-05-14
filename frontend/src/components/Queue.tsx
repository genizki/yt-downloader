import { useEffect, useRef } from "react";
import { Icon } from "./Icons";
import { QueueRow } from "../types";
import { useT } from "../i18n";

const STATE_KEYS: Record<QueueRow["state"], string> = {
  downloading: "download.downloading",
  queued: "download.queued",
  done: "download.done",
  failed: "download.failed",
  post_processing: "download.post_processing",
  moving: "download.moving",
};

interface Props {
  items: QueueRow[];
  open: boolean;
  setOpen: (v: boolean) => void;
}

export function QueueButton({ items, open, setOpen }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const t = useT();

  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", onClick);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onClick);
      window.removeEventListener("keydown", onKey);
    };
  }, [open, setOpen]);

  const active = items.filter((i) => i.state === "downloading").length;

  return (
    <div className="queue-wrap" ref={ref}>
      <button
        className={`queue-btn ${open ? "queue-btn--on" : ""}`}
        onClick={() => setOpen(!open)}
        aria-label={t("queue.toggle")}
      >
        <Icon.Stack width={16} height={16} />
        <span className="queue-btn-label">{t("queue.label")}</span>
        {active > 0 && <span className="queue-btn-badge">{active}</span>}
      </button>
      {open && (
        <div className="queue-pop">
          <div className="queue-head">
            <div className="queue-eyebrow">{t("queue.heading")}</div>
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
                    <div className="qbar">
                      <div className="qbar-fill" style={{ width: `${q.progress * 100}%` }} />
                    </div>
                    <div className="qitem-meta">
                      <span>{Math.round(q.progress * 100)}%</span>
                      <span className="qstate">{t("download.downloading")}</span>
                    </div>
                  </>
                ) : (
                  <div className="qitem-meta">
                    <span>—</span>
                    <span className={`qstate qstate--${q.state}`}>{t(STATE_KEYS[q.state])}</span>
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

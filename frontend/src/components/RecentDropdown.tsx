import { Icon } from "./Icons";
import { useT } from "../i18n";
import type { RecentEntry } from "../App";

interface Props {
  visible: boolean;
  items: RecentEntry[];
  onPick: (q: string) => void;
  onRemove: (q: string) => void;
}

export function RecentDropdown({ visible, items, onPick, onRemove }: Props) {
  const t = useT();
  if (!visible) return null;
  return (
    <div className="recent">
      <div className="recent-label">{t("search.recent")}</div>
      {items.map((entry) => (
        <button
          key={entry.query}
          className="recent-item"
          onMouseDown={(e) => {
            e.preventDefault();
            onPick(entry.query);
          }}
        >
          <Icon.Clock width={14} height={14} />
          <span>{entry.label}</span>
          <span
            role="button"
            tabIndex={-1}
            aria-label={t("search.recent_remove")}
            className="recent-item-remove"
            onMouseDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onRemove(entry.query);
            }}
          >
            <Icon.X width={14} height={14} />
          </span>
        </button>
      ))}
    </div>
  );
}

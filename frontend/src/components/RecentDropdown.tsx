import { Icon } from "./Icons";
import { useT } from "../i18n";

interface Props {
  visible: boolean;
  items: string[];
  onPick: (q: string) => void;
}

export function RecentDropdown({ visible, items, onPick }: Props) {
  const t = useT();
  if (!visible) return null;
  return (
    <div className="recent">
      <div className="recent-label">{t("search.recent")}</div>
      {items.map((q) => (
        <button
          key={q}
          className="recent-item"
          onMouseDown={(e) => {
            e.preventDefault();
            onPick(q);
          }}
        >
          <Icon.Clock width={14} height={14} />
          <span>{q}</span>
        </button>
      ))}
    </div>
  );
}

import { useEffect, useRef, useState } from "react";
import { Icon } from "./Icons";
import { TOKEN_MASK } from "../types";

interface Props {
  value: string;
  onChange: (v: string) => void;
}

export function TokenInput({ value, onChange }: Props) {
  const [editing, setEditing] = useState(false);
  const [revealed, setRevealed] = useState(false);
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing && ref.current) ref.current.focus();
  }, [editing]);

  const hasValue = value.length > 0;

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
          {revealed ? <Icon.EyeOff width={16} height={16} /> : <Icon.Eye width={16} height={16} />}
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
        onKeyDown={(e) => {
          if (e.key === "Escape" || e.key === "Enter") e.currentTarget.blur();
        }}
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
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => setRevealed((r) => !r)}
        title={revealed ? "Hide token" : "Show token"}
        aria-label={revealed ? "Hide token" : "Show token"}
        aria-pressed={revealed}
      >
        {revealed ? <Icon.EyeOff width={16} height={16} /> : <Icon.Eye width={16} height={16} />}
      </button>
    </div>
  );
}

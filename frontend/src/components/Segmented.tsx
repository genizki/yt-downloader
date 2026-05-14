interface Props {
  value: string;
  onChange: (v: string) => void;
  options: string[];
  disabled?: boolean;
  accents?: string[];
  /** Optional value → display-label map. Falls back to raw option string. */
  optionLabels?: Record<string, string>;
}

export function Segmented({ value, onChange, options, disabled, accents, optionLabels }: Props) {
  return (
    <div className={`seg ${disabled ? "seg--disabled" : ""}`}>
      {options.map((o) => (
        <button
          key={o}
          disabled={disabled}
          className={`seg-btn ${value === o ? "seg-btn--on" : ""} ${
            accents && accents.includes(o) ? "seg-btn--accent" : ""
          }`}
          onClick={() => !disabled && onChange(o)}
        >
          {optionLabels?.[o] ?? o}
        </button>
      ))}
    </div>
  );
}

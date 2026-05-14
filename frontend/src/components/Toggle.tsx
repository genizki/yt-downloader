interface Props {
  value: boolean;
  onChange: (v: boolean) => void;
  ariaLabel?: string;
}

export function Toggle({ value, onChange, ariaLabel }: Props) {
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

/**
 * Formatting helpers used by the result list + queue, matching the
 * conventions used by the JSX prototype (e.g. "284K views", "3 weeks ago").
 */

export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const mm = m.toString().padStart(2, "0");
  const ss = sec.toString().padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${ss}` : `0:${mm}:${ss}`;
}

export function formatViews(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B views`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M views`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K views`;
  return `${n} views`;
}

export function formatRelative(iso: string, now: Date = new Date()): string {
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return "";
  const diffSec = Math.max(0, Math.floor((now.getTime() - t) / 1000));
  const units: [number, string][] = [
    [60, "second"],
    [60, "minute"],
    [24, "hour"],
    [7, "day"],
    [4.345, "week"],
    [12, "month"],
    [Infinity, "year"],
  ];
  let value = diffSec;
  let label = "second";
  for (const [div, name] of units) {
    if (value < div) {
      label = name;
      break;
    }
    value = value / div;
    label = name;
  }
  const rounded = Math.max(1, Math.floor(value));
  return `${rounded} ${label}${rounded === 1 ? "" : "s"} ago`;
}

/** Deterministic hue in 0..360 from a string id. Matches the visual goal of
 * giving each card a stable color stripe pattern even with no thumbnail. */
export function hueFromId(id: string): number {
  let hash = 0;
  for (let i = 0; i < id.length; i++) {
    hash = (hash * 31 + id.charCodeAt(i)) >>> 0;
  }
  return hash % 360;
}

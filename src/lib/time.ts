const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

export function shortAge(timestamp: number, now = Date.now()): string {
  const elapsed = Math.max(0, now - timestamp);
  if (elapsed < MINUTE) return "now";
  if (elapsed < HOUR) return `${Math.floor(elapsed / MINUTE)}m`;
  if (elapsed < DAY) return `${Math.floor(elapsed / HOUR)}h`;
  if (elapsed < 7 * DAY) return `${Math.floor(elapsed / DAY)}d`;
  return new Date(timestamp).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

const WINDOWS: Record<string, string> = {
  five_hour: "5-hour",
  seven_day: "weekly",
  weekly: "weekly",
  monthly: "monthly",
};

/** Anthropic names its windows `five_hour`; a name we do not know is shown as
 *  it came rather than dropped. */
export const windowLabel = (kind: string) => WINDOWS[kind] ?? kind.replace(/_/g, " ");

/** `null` once the reset is behind us: the window has already rolled over, so
 *  the time we were told is no longer about anything. */
export function resetLabel(resetsAt: number, now = Date.now()): string | null {
  const at = resetsAt * 1000;
  if (at <= now) return null;
  const today = new Date(at).toDateString() === new Date(now).toDateString();
  return new Date(at).toLocaleString(undefined, {
    ...(today ? {} : { weekday: "short" }),
    hour: "numeric",
    minute: "2-digit",
  });
}

import { modelLabel, PICKER } from "../lib/models";
import { resetLabel, shortAge, windowLabel } from "../lib/time";
import type { Provider, RateLimit } from "../lib/types";
import SpectralDot from "./SpectralDot";

interface TriggerProps {
  providerId: string;
  model: string;
  open: boolean;
  onToggle: () => void;
}

export function ModelTrigger({ providerId, model, open, onToggle }: TriggerProps) {
  return (
    <button
      {...{ [PICKER]: "" }}
      type="button"
      aria-expanded={open}
      onClick={onToggle}
      className="flex shrink-0 items-center gap-1.5 rounded-md px-1.5 py-1 text-[12px] whitespace-nowrap text-muted hover:text-ink"
    >
      <SpectralDot providerId={providerId} />
      {modelLabel(model)}
    </button>
  );
}

/** Older than this and the window is reported with its age: what it says about
 *  the reset time stays true, but whether the user is still inside it may not. */
const STALE_AFTER = 10 * 60_000;

/** The subscription window, which spans every session the user has run rather
 *  than this app's share, so it is deliberately not phrased as Starlux's usage.
 *  There is no percentage to show: the provider reports which window and when it
 *  resets, and inventing a number from our own token counts would be a guess. */
function SubscriptionWindow({ limit }: { limit: RateLimit }) {
  const resets = limit.resetsAt === null ? null : resetLabel(limit.resetsAt);
  // Past its reset, the window has already rolled over and we know nothing
  // about the one that replaced it.
  if (!resets) return null;

  const limited = limit.status !== "allowed";
  const stale = Date.now() - limit.observedAt * 1000 > STALE_AFTER;

  return (
    <p
      className={`px-3 pt-1 pb-2 font-mono text-[10px] ${limited ? "text-class-m" : "text-faint"}`}
      title={`Your ${windowLabel(limit.kind)} limit across every Claude session, not Starlux's alone`}
    >
      {limited ? `${limit.status.replace(/_/g, " ")} · ` : ""}
      {windowLabel(limit.kind)} resets {resets}
      {limit.usingOverage ? " · overage" : ""}
      {stale ? ` · ${shortAge(limit.observedAt * 1000)} ago` : ""}
    </p>
  );
}

interface MenuProps {
  providers: Provider[];
  providerId: string;
  model: string;
  limits: Record<string, RateLimit>;
  onSelect: (providerId: string, model: string) => void;
  className?: string;
}

/** Positioned by its window: the Workspace floats it over the thread, the Quick
 *  Bar puts it in transparent space the window grows for. */
export function ModelMenu({
  providers,
  providerId,
  model,
  limits,
  onSelect,
  className = "",
}: MenuProps) {
  return (
    <div {...{ [PICKER]: "" }} className={className}>
      <div className="ml-auto max-h-56 w-52 overflow-y-auto rounded-lg border border-rule bg-haze shadow-xl shadow-black/40">
        {providers.map(({ id, name, binary, models, availability }) => (
          <div key={id}>
            <p className="border-b border-rule/60 px-3 py-1.5 font-mono text-[10px] tracking-wide text-faint uppercase">
              {name}
              {availability.state === "ready" && availability.plan
                ? ` · ${availability.plan}`
                : ""}
            </p>

            {/* Listed even when it cannot be run: a provider that disappears
                from the menu while signed out reads as uninstalled. */}
            {availability.state !== "ready" ? (
              <p className="px-3 py-1.5 text-[11.5px] text-faint">
                {availability.state === "signedOut"
                  ? `Signed out — run \`${binary} login\``
                  : `Not found — install \`${binary}\``}
              </p>
            ) : (
              models.map((option) => {
                const current = id === providerId && option === model;
                return (
                  <button
                    key={option}
                    type="button"
                    onClick={() => onSelect(id, option)}
                    className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-[12.5px] hover:bg-white/6 ${
                      current ? "text-ink" : "text-muted"
                    }`}
                  >
                    <SpectralDot providerId={id} className={current ? "" : "opacity-25"} />
                    {modelLabel(option)}
                  </button>
                );
              })
            )}

            {/* Not for a provider that cannot be run: a window it is no longer
                inside is the one thing worse than showing nothing. */}
            {availability.state === "ready" && limits[id] ? (
              <SubscriptionWindow limit={limits[id]} />
            ) : null}
          </div>
        ))}
      </div>
    </div>
  );
}

import { PICKER } from "../lib/models";
import { resetLabel, shortAge, windowLabel } from "../lib/time";
import type { Provider, RateLimit } from "../lib/types";
import SpectralDot from "./SpectralDot";

interface TriggerProps {
  providers: Provider[];
  providerId: string;
  open: boolean;
  onToggle: () => void;
}

/** Which CLI answers. Named in full, because the name is the whole claim: it
 *  says what is being run, not what Starlux is. */
export function ProviderTrigger({ providers, providerId, open, onToggle }: TriggerProps) {
  const current = providers.find((provider) => provider.id === providerId);

  return (
    <button
      {...{ [PICKER]: "" }}
      type="button"
      aria-expanded={open}
      onClick={onToggle}
      className="flex shrink-0 items-center gap-1.5 rounded-md px-1.5 py-1 text-[12px] whitespace-nowrap text-muted hover:text-ink"
    >
      <SpectralDot providerId={providerId} />
      {current?.name ?? providerId}
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
function SubscriptionWindow({ limit, provider }: { limit: RateLimit; provider: string }) {
  const resets = limit.resetsAt === null ? null : resetLabel(limit.resetsAt);
  // Past its reset, the window has already rolled over and we know nothing
  // about the one that replaced it.
  if (!resets) return null;

  const limited = limit.status !== "allowed";
  const stale = Date.now() - limit.observedAt * 1000 > STALE_AFTER;

  return (
    <p
      className={`pt-0.5 font-mono text-[10px] ${limited ? "text-class-m" : "text-faint"}`}
      title={`Your ${windowLabel(limit.kind)} limit across every ${provider} session, not Starlux's alone`}
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
  limits: Record<string, RateLimit>;
  onSelect: (providerId: string) => void;
  className?: string;
}

export function ProviderMenu({
  providers,
  providerId,
  limits,
  onSelect,
  className = "",
}: MenuProps) {
  return (
    <div {...{ [PICKER]: "" }} className={className}>
      <div className="ml-auto w-60 overflow-hidden rounded-lg border border-rule bg-haze shadow-xl shadow-black/40">
        {providers.map((provider) => {
          const { availability } = provider;
          const ready = availability.state === "ready";
          const current = provider.id === providerId;

          return (
            <button
              key={provider.id}
              type="button"
              disabled={!ready}
              onClick={() => onSelect(provider.id)}
              // Three states worth telling apart at a glance: the one running,
              // the ones that could, and the ones that cannot until something
              // is installed or signed in to.
              className={`flex w-full flex-col items-start gap-0 px-3 py-2 text-left text-[12.5px] ${
                ready ? "hover:bg-white/6" : "cursor-default"
              } ${current ? "text-ink" : ready ? "text-muted" : "text-faint"}`}
            >
              <span className="flex items-center gap-2">
                <SpectralDot providerId={provider.id} className={current ? "" : "opacity-25"} />
                {provider.name}
                {availability.state === "ready" && availability.plan ? (
                  <span className="font-mono text-[10px] text-faint uppercase">
                    {availability.plan}
                  </span>
                ) : null}
              </span>

              {/* Named even when it cannot be run: a provider that disappears
                  while signed out reads as uninstalled, and the two have
                  different fixes. */}
              {ready ? (
                limits[provider.id] ? (
                  <SubscriptionWindow limit={limits[provider.id]} provider={provider.name} />
                ) : null
              ) : (
                <span className="pt-0.5 text-[11px] text-faint">
                  {availability.state === "signedOut"
                    ? `Signed out — run \`${provider.login}\``
                    : `Not found — install \`${provider.binary}\``}
                </span>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}

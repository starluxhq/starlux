import { modelLabel, PICKER } from "../lib/models";
import type { Provider } from "../lib/types";
import SpectralDot from "./SpectralDot";

interface TriggerProps {
  providerId: string;
  model: string | null;
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

interface MenuProps {
  providers: Provider[];
  providerId: string;
  model: string | null;
  onSelect: (providerId: string, model: string | null) => void;
}

/** Sits in the bar's flow rather than floating over it: the window is only as
 *  tall as its content, so an overlay would be clipped by the window edge. */
export function ModelMenu({ providers, providerId, model, onSelect }: MenuProps) {
  return (
    <div {...{ [PICKER]: "" }} className="max-h-56 overflow-y-auto px-3 pt-2">
      <div className="ml-auto w-52 overflow-hidden rounded-lg border border-rule bg-haze">
        {providers.map((provider) => (
          <div key={provider.id}>
            <p className="border-b border-rule/60 px-3 py-1.5 font-mono text-[10px] tracking-wide text-faint uppercase">
              {provider.name}
            </p>
            {[null, ...provider.models].map((option) => {
              const current = provider.id === providerId && option === model;
              return (
                <button
                  key={option ?? "default"}
                  type="button"
                  onClick={() => onSelect(provider.id, option)}
                  className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-[12.5px] hover:bg-white/6 ${
                    current ? "text-ink" : "text-muted"
                  }`}
                >
                  <SpectralDot providerId={provider.id} className={current ? "" : "opacity-25"} />
                  {modelLabel(option)}
                </button>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}

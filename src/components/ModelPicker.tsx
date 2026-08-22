import { modelLabel, PICKER } from "../lib/models";
import type { Provider } from "../lib/types";
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

interface MenuProps {
  providers: Provider[];
  providerId: string;
  model: string;
  onSelect: (providerId: string, model: string) => void;
  className?: string;
}

/** Positioned by its window: the Workspace floats it over the thread, the Quick
 *  Bar puts it in transparent space the window grows for. */
export function ModelMenu({ providers, providerId, model, onSelect, className = "" }: MenuProps) {
  return (
    <div {...{ [PICKER]: "" }} className={className}>
      <div className="ml-auto max-h-56 w-52 overflow-y-auto rounded-lg border border-rule bg-haze shadow-xl shadow-black/40">
        {providers.map((provider) => (
          <div key={provider.id}>
            <p className="border-b border-rule/60 px-3 py-1.5 font-mono text-[10px] tracking-wide text-faint uppercase">
              {provider.name}
            </p>
            {provider.models.map((option) => {
              const current = provider.id === providerId && option === model;
              return (
                <button
                  key={option}
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

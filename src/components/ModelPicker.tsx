import { modelLabel, modelName, PICKER } from "../lib/models";
import type { Provider } from "../lib/types";

interface TriggerProps {
  model: string;
  open: boolean;
  onToggle: () => void;
}

/** Which model of the chosen provider. The provider is named beside it, so the
 *  vendor prefix an id like `opencode-go/glm-5.3` carries is redundant here and
 *  only the model's own name is shown. The menu still lists it in full. */
export function ModelTrigger({ model, open, onToggle }: TriggerProps) {
  return (
    <button
      {...{ [PICKER]: "" }}
      type="button"
      aria-expanded={open}
      onClick={onToggle}
      className="flex shrink-0 items-center rounded-md px-1.5 py-1 text-[12px] whitespace-nowrap text-muted hover:text-ink"
    >
      {modelName(model)}
    </button>
  );
}

interface MenuProps {
  provider: Provider | undefined;
  model: string;
  onSelect: (model: string) => void;
  className?: string;
}

/** Positioned by its window: the Workspace floats it over the thread, the Quick
 *  Bar puts it in transparent space the window grows for. */
export function ModelMenu({ provider, model, onSelect, className = "" }: MenuProps) {
  if (!provider) return null;

  return (
    <div {...{ [PICKER]: "" }} className={className}>
      <div className="ml-auto max-h-56 w-56 overflow-y-auto rounded-lg border border-rule bg-haze shadow-xl shadow-black/40">
        {provider.models.map((option) => (
          <button
            key={option}
            type="button"
            onClick={() => onSelect(option)}
            className={`block w-full px-3 py-1.5 text-left text-[12.5px] hover:bg-white/6 ${
              option === model ? "text-ink" : "text-muted"
            }`}
          >
            {modelLabel(option)}
          </button>
        ))}
      </div>
    </div>
  );
}

import { byVendor, modelName, PICKER } from "../lib/models";
import type { Provider } from "../lib/types";

interface TriggerProps {
  model: string;
  open: boolean;
  onToggle: () => void;
}

/** Which model of the chosen provider. The provider is named beside it, so the
 *  vendor prefix an id like `opencode-go/glm-5.3` carries is redundant here and
 *  only the model's own name is shown. */
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
      <div className="ml-auto max-h-72 w-56 overflow-y-auto rounded-lg border border-rule bg-haze shadow-xl shadow-black/40">
        {byVendor(provider.models).map((vendor) => (
          <div key={vendor.name ?? ""}>
            {/* Sticky because thirty models scroll past in one menu, and which
                account you are picking from is the thing worth keeping on
                screen while you do. */}
            {vendor.name ? (
              <p className="sticky top-0 bg-haze px-3 pt-2.5 pb-1 font-mono text-[10px] tracking-wide text-faint uppercase">
                {vendor.name}
              </p>
            ) : null}

            {vendor.models.map((option) => (
              <button
                key={option.id}
                type="button"
                onClick={() => onSelect(option.id)}
                className={`block w-full px-3 py-1.5 text-left text-[12.5px] hover:bg-white/6 ${
                  option.id === model ? "text-ink" : "text-muted"
                }`}
              >
                {modelName(option.id)}
              </button>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

import { PICKER } from "../lib/models";

/** Read as a level, not a name: the words are the model's own, and `xhigh`
 *  beside `max` only reads as an order if the order is what is shown. */
const LEVEL = "text-[12px] whitespace-nowrap";

interface TriggerProps {
  efforts: string[];
  effort: string | null;
  open: boolean;
  onToggle: () => void;
}

/** How hard to think. Absent from the bar entirely where the model offers no
 *  choice — most of them — rather than shown as a control that does nothing. */
export function EffortTrigger({ efforts, effort, open, onToggle }: TriggerProps) {
  if (efforts.length === 0) return null;

  return (
    <button
      {...{ [PICKER]: "" }}
      type="button"
      aria-expanded={open}
      aria-label="Thinking level"
      onClick={onToggle}
      className={`flex shrink-0 items-center rounded-md px-1.5 py-1 hover:text-ink ${LEVEL} ${
        effort ? "text-muted" : "text-faint"
      }`}
    >
      {effort ?? "auto"}
    </button>
  );
}

interface MenuProps {
  efforts: string[];
  effort: string | null;
  onSelect: (effort: string | null) => void;
  className?: string;
}

export function EffortMenu({ efforts, effort, onSelect, className = "" }: MenuProps) {
  if (efforts.length === 0) return null;

  const row = (value: string | null, label: string) => (
    <button
      key={label}
      type="button"
      onClick={() => onSelect(value)}
      className={`block w-full px-3 py-1.5 text-left text-[12.5px] hover:bg-white/6 ${
        value === effort ? "text-ink" : "text-muted"
      }`}
    >
      {label}
    </button>
  );

  return (
    <div {...{ [PICKER]: "" }} className={className}>
      <div className="ml-auto w-40 overflow-hidden rounded-lg border border-rule bg-haze shadow-xl shadow-black/40">
        {/* Not a rung on the ladder: it sends no flag at all, which is the only
            way to ask for whatever the provider would have done anyway. */}
        {row(null, "auto")}
        <div className="mx-3 border-t border-white/6" />
        {efforts.map((level) => row(level, level))}
      </div>
    </div>
  );
}

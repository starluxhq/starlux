interface ToggleProps {
  on: boolean;
  label: string;
  /** A tool no installed provider offers cannot be granted to anything, so the
   *  switch says so rather than storing a setting nothing will read. */
  disabled?: boolean;
  onChange: (on: boolean) => void;
}

/** A switch, not a checkbox: what it changes takes effect the moment it moves,
 *  and there is no form around it to submit. */
export default function Toggle({ on, label, disabled = false, onChange }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!on)}
      className={`relative h-[18px] w-8 shrink-0 rounded-full transition-colors duration-150 motion-reduce:transition-none ${
        disabled
          ? "cursor-not-allowed bg-rule/40"
          : on
            ? "bg-class-a/70"
            : "bg-rule hover:bg-rule/80"
      }`}
    >
      <span
        aria-hidden
        className={`absolute top-[3px] size-3 rounded-full transition-[left] duration-150 motion-reduce:transition-none ${
          on ? "left-[17px]" : "left-[3px]"
        } ${disabled ? "bg-faint" : on ? "bg-void" : "bg-muted"}`}
      />
    </button>
  );
}

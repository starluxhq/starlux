import type { Context } from "../lib/types";

/** Past this the conversation is close enough to the model's limit that the
 *  number is worth noticing rather than merely available. */
const CROWDED = 0.8;

/** How full this conversation is. Deliberately a share rather than a token
 *  count: the number that changes a decision is how much room is left, and the
 *  exact figures are one hover away. */
export default function ContextMeter({ context }: { context: Context }) {
  const share = Math.min(1, context.used / context.window);

  return (
    <span
      title={`${context.used.toLocaleString()} of ${context.window.toLocaleString()} tokens carried into the next turn`}
      className={`shrink-0 font-mono text-[10px] whitespace-nowrap ${
        share >= CROWDED ? "text-class-m" : "text-faint"
      }`}
    >
      {Math.round(share * 100)}%
    </span>
  );
}

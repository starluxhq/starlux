import type { CustomRendererProps } from "streamdown";
import { isRecord, widgetFor } from "../../lib/markdown/widgets/registry";

function Placeholder() {
  return (
    <div className="my-4 h-24 animate-pulse rounded-lg border border-rule bg-dust/40" aria-hidden />
  );
}

/** Shown when the payload is finished but unusable, which keeps a malformed
 *  widget legible rather than swallowing the answer it belonged to. */
function Unreadable({ code }: { code: string }) {
  return (
    <figure className="my-4 overflow-hidden rounded-lg border border-rule bg-dust/40">
      <figcaption className="border-b border-rule px-3 py-2 font-mono text-[10px] tracking-wide text-muted uppercase">
        Unrecognised widget
      </figcaption>
      <pre className="overflow-x-auto px-3 py-2 font-mono text-[11px] leading-relaxed text-faint">
        {code}
      </pre>
    </figure>
  );
}

export default function Widget({ code, isIncomplete }: CustomRendererProps) {
  // Half a JSON object is the normal state mid-stream; parsing it would only
  // throw once per delta.
  if (isIncomplete) return <Placeholder />;

  let parsed: unknown;
  try {
    parsed = JSON.parse(code);
  } catch {
    return <Unreadable code={code} />;
  }

  if (!isRecord(parsed) || typeof parsed.type !== "string") return <Unreadable code={code} />;

  const definition = widgetFor(parsed.type);
  if (!definition?.match(parsed)) return <Unreadable code={code} />;

  const Render = definition.component;
  return <Render data={parsed} />;
}

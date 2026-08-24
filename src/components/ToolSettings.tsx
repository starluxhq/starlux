import Toggle from "./Toggle";
import { isReady, type Provider, type ToolId, type Tools } from "../lib/types";

interface ToolSettingsProps {
  tools: Tools;
  providers: Provider[];
  onChange: (id: ToolId, on: boolean) => void;
}

/** What each switch actually turns on, in the user's words rather than the
 *  provider's. Every CLI calls these something different; the adapters
 *  translate, and this is the only name anyone has to read. */
const CATALOGUE: { id: ToolId; name: string; does: string }[] = [
  {
    id: "webSearch",
    name: "Web search",
    does: "Search the web and read the results.",
  },
  {
    id: "webFetch",
    name: "Web fetch",
    does: "Read a page at an address you name.",
  },
];

export default function ToolSettings({ tools, providers, onChange }: ToolSettingsProps) {
  // Only providers that could actually run it: a tool nobody signed in to has
  // to offer is a switch with nothing behind it.
  const offering = (id: ToolId) =>
    providers.filter((provider) => isReady(provider) && provider.tools.includes(id));

  return (
    <div className="space-y-5">
      <p className="max-w-prose text-[12.5px] leading-relaxed text-muted">
        These apply to every conversation, including questions asked from the Quick Bar. Nothing
        else is granted here — a run still cannot reach your files unless that conversation was
        given a folder.
      </p>

      <div className="overflow-hidden rounded-lg border border-rule">
        {CATALOGUE.map((tool, at) => {
          const available = offering(tool.id);
          const known = providers.length > 0;
          const granted = tools[tool.id];
          const unavailable = known && available.length === 0;

          return (
            <div
              key={tool.id}
              className={`flex items-start gap-4 px-4 py-3.5 ${at > 0 ? "border-t border-rule" : ""}`}
            >
              <div className="min-w-0 flex-1">
                <p className="text-[13px] text-ink">{tool.name}</p>
                <p className="mt-0.5 text-[12px] leading-relaxed text-muted">{tool.does}</p>
                {known ? (
                  <p className="mt-1 font-mono text-[10px] tracking-wide text-faint uppercase">
                    {unavailable
                      ? "No signed-in provider offers this"
                      : available.map((provider) => provider.name).join(" · ")}
                  </p>
                ) : null}
              </div>

              <div className="pt-0.5">
                {/* A grant can always be given back, even where nothing is
                    left to spend it: what cannot be done is make a new one
                    nothing would honour. */}
                <Toggle
                  on={granted}
                  label={tool.name}
                  disabled={unavailable && !granted}
                  onChange={(on) => onChange(tool.id, on)}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

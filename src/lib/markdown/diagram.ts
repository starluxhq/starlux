import type { DiagramPlugin } from "streamdown";

type Mermaid = Awaited<typeof import("mermaid")>["default"];

/** Matches the palette an answer is drawn in. Mermaid's own themes assume a
 *  light page and leave pale nodes floating on the Starlux surface. */
const THEME = {
  background: "#12141a",
  primaryColor: "#1c1f28",
  primaryTextColor: "#eef1f6",
  primaryBorderColor: "#333846",
  secondaryColor: "#12141a",
  tertiaryColor: "#08090c",
  lineColor: "#848c9f",
  textColor: "#eef1f6",
  mainBkg: "#1c1f28",
  nodeBorder: "#333846",
  clusterBkg: "#12141a",
  clusterBorder: "#333846",
  titleColor: "#eef1f6",
  edgeLabelBackground: "#12141a",
  // Sequence diagrams take none of the above and have to be named separately.
  actorBkg: "#1c1f28",
  actorBorder: "#333846",
  actorTextColor: "#eef1f6",
  actorLineColor: "#5a6173",
  signalColor: "#eef1f6",
  signalTextColor: "#eef1f6",
  labelBoxBkgColor: "#1c1f28",
  labelBoxBorderColor: "#333846",
  labelTextColor: "#eef1f6",
  loopTextColor: "#eef1f6",
  noteBkgColor: "#12141a",
  noteBorderColor: "#333846",
  noteTextColor: "#a9c7ff",
  altBackground: "#08090c",
  sequenceNumberColor: "#08090c",
  fontFamily: '"Instrument Sans", ui-sans-serif, system-ui, sans-serif',
};

const BASE = { startOnLoad: false, theme: "base" as const, themeVariables: THEME };

/** Reassigned by `initialize`, so a render queued straight after it waits for
 *  the configured instance rather than racing an unconfigured one. */
let pending: Promise<Mermaid> | null = null;

/** Configured here rather than only in `initialize`, which Streamdown does not
 *  always call — without this a diagram can render in Mermaid's light default. */
function load(): Promise<Mermaid> {
  pending ??= import("mermaid").then((module) => {
    module.default.initialize(BASE);
    return module.default;
  });
  return pending;
}

export const diagram: DiagramPlugin = {
  name: "mermaid",
  type: "diagram",
  language: "mermaid",

  getMermaid(config) {
    return {
      initialize: (overrides) => {
        pending = load().then((instance) => {
          instance.initialize({ ...config, ...overrides, ...BASE });
          return instance;
        });
      },
      render: (id, source) => load().then((instance) => instance.render(id, source)),
    };
  },
};

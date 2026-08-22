import type { PluginConfig } from "streamdown";

import Artifact from "../../components/widgets/Artifact";
import Chart from "../../components/widgets/Chart";
import Form from "../../components/widgets/Form";
import Table from "../../components/widgets/Table";
import Widget from "../../components/widgets/Widget";
import { code } from "./code";
import { diagram } from "./diagram";
import { math } from "./math";
import { registerWidget } from "./widgets/registry";
import { isChartWidget, isFormWidget, isTableWidget } from "./widgets/shapes";

/** One fence language for every widget, with the kind carried in the payload.
 *  Streamdown matches a renderer on the language alone, so a fence per kind
 *  would mean registering each one here as well as teaching the model about it. */
export const WIDGET_LANGUAGE = "starlux-widget";

/** Interactive documents, framed rather than parsed, so they get their own file. */
export const ARTIFACT_LANGUAGE = "starlux-artifact";

registerWidget("table", { match: isTableWidget, component: Table });
registerWidget("chart", { match: isChartWidget, component: Chart });
registerWidget("form", { match: isFormWidget, component: Form });

export const markdown: PluginConfig = {
  code,
  math,
  mermaid: diagram,
  renderers: [
    { language: WIDGET_LANGUAGE, component: Widget },
    { language: ARTIFACT_LANGUAGE, component: Artifact },
  ],
};

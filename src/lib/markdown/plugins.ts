import type { PluginConfig } from "streamdown";

import Chart from "../../components/widgets/Chart";
import Table from "../../components/widgets/Table";
import Widget from "../../components/widgets/Widget";
import { code } from "./code";
import { math } from "./math";
import { registerWidget } from "./widgets/registry";
import { isChartWidget, isTableWidget } from "./widgets/shapes";

/** One fence language for every widget, with the kind carried in the payload.
 *  Streamdown matches a renderer on the language alone, so a fence per kind
 *  would mean registering each one here as well as teaching the model about it. */
export const WIDGET_LANGUAGE = "starlux-widget";

registerWidget("table", { match: isTableWidget, component: Table });
registerWidget("chart", { match: isChartWidget, component: Chart });

export const markdown: PluginConfig = {
  code,
  math,
  renderers: [{ language: WIDGET_LANGUAGE, component: Widget }],
};

import { isRecord, isStringArray } from "./registry";

export interface TableWidget {
  title?: string;
  columns: string[];
  rows: (string | number)[][];
}

export interface ChartWidget {
  chart: "bar" | "line";
  title?: string;
  x: string[];
  series: { label: string; values: number[] }[];
}

const isCell = (value: unknown): value is string | number =>
  typeof value === "string" || typeof value === "number";

const hasTitle = (data: Record<string, unknown>) =>
  data.title === undefined || typeof data.title === "string";

export function isTableWidget(
  data: Record<string, unknown>,
): data is TableWidget & Record<string, unknown> {
  return (
    hasTitle(data) &&
    isStringArray(data.columns) &&
    Array.isArray(data.rows) &&
    data.rows.every((row) => Array.isArray(row) && row.every(isCell))
  );
}

export function isChartWidget(
  data: Record<string, unknown>,
): data is ChartWidget & Record<string, unknown> {
  return (
    (data.chart === "bar" || data.chart === "line") &&
    hasTitle(data) &&
    isStringArray(data.x) &&
    Array.isArray(data.series) &&
    data.series.length > 0 &&
    data.series.every(
      (entry) =>
        isRecord(entry) &&
        typeof entry.label === "string" &&
        Array.isArray(entry.values) &&
        entry.values.every((value) => typeof value === "number" && Number.isFinite(value)),
    )
  );
}

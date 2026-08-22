import { isRecord, isStringArray } from "./registry";

export interface TableWidget {
  title?: string;
  columns: string[];
  rows: (string | number)[][];
}

export type FormField =
  | { name: string; label: string; kind: "text" | "number"; value?: string | number }
  | { name: string; label: string; kind: "checkbox"; value?: boolean }
  | { name: string; label: string; kind: "select"; options: string[]; value?: string };

export interface FormWidget {
  title?: string;
  /** `{name}` for each field. Absent or naming a field that does not exist and
   *  the submitted turn falls back to a labelled list of the values. */
  submit?: string;
  fields: FormField[];
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

function isFormField(field: unknown): field is FormField {
  if (!isRecord(field) || typeof field.name !== "string" || !field.name) return false;
  if (typeof field.label !== "string" || !field.label) return false;

  switch (field.kind) {
    case "text":
      return field.value === undefined || typeof field.value === "string";
    case "number":
      return field.value === undefined || typeof field.value === "number";
    case "checkbox":
      return field.value === undefined || typeof field.value === "boolean";
    case "select":
      return (
        isStringArray(field.options) &&
        field.options.length > 0 &&
        (field.value === undefined ||
          (typeof field.value === "string" && field.options.includes(field.value)))
      );
    default:
      return false;
  }
}

export function isFormWidget(
  data: Record<string, unknown>,
): data is FormWidget & Record<string, unknown> {
  return (
    hasTitle(data) &&
    (data.submit === undefined || typeof data.submit === "string") &&
    Array.isArray(data.fields) &&
    data.fields.length > 0 &&
    data.fields.every(isFormField) &&
    // Two fields of the same name would make `{name}` ambiguous and let one
    // silently overwrite the other's value.
    new Set(data.fields.map((field) => (field as FormField).name)).size === data.fields.length
  );
}

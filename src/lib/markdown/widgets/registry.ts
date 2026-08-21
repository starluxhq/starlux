import type { ComponentType } from "react";

/** A widget owns validating its own payload: the JSON comes from a model, so a
 *  shape that does not match is expected traffic rather than a bug. */
export interface WidgetDefinition<T> {
  match: (data: Record<string, unknown>) => data is T & Record<string, unknown>;
  component: ComponentType<{ data: T }>;
}

// The registry exists so later renderers — sandboxed HTML, diagrams — are a
// registration rather than another branch in the dispatcher.
const registry = new Map<string, WidgetDefinition<never>>();

export function registerWidget<T>(type: string, definition: WidgetDefinition<T>): void {
  registry.set(type, definition as unknown as WidgetDefinition<never>);
}

export function widgetFor(type: string): WidgetDefinition<never> | undefined {
  return registry.get(type);
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export const isStringArray = (value: unknown): value is string[] =>
  Array.isArray(value) && value.every((item) => typeof item === "string");

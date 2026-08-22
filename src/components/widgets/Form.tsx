import { Fragment, type FormEvent } from "react";
import { useTurnId } from "../../lib/turn";
import type { FormField, FormWidget } from "../../lib/markdown/widgets/shapes";
import { useChat } from "../../stores/useChat";
import { draftKey, useForms, type FormValue } from "../../stores/useForms";

const PLACEHOLDER = /\{([A-Za-z0-9_.-]+)\}/g;

const defaultOf = (field: FormField): FormValue => {
  if (field.kind === "checkbox") return field.value ?? false;
  if (field.kind === "select") return field.value ?? field.options[0];
  return field.value === undefined ? "" : String(field.value);
};

const shown = (field: FormField, value: FormValue): string =>
  field.kind === "checkbox" ? (value ? "yes" : "no") : String(value);

/** Returns `null` when the template names a field the form does not have, so a
 *  half-substituted sentence is never what gets asked. */
function fill(
  template: string,
  fields: FormField[],
  values: Record<string, FormValue>,
): string | null {
  let unknown = false;
  const text = template.replace(PLACEHOLDER, (whole, name: string) => {
    const field = fields.find((candidate) => candidate.name === name);
    if (!field) {
      unknown = true;
      return whole;
    }
    return shown(field, values[field.name]);
  });
  return unknown ? null : text.trim();
}

const labelled = (data: FormWidget, values: Record<string, FormValue>): string =>
  [data.title, ...data.fields.map((field) => `${field.label}: ${shown(field, values[field.name])}`)]
    .filter(Boolean)
    .join("\n");

const CONTROL =
  "w-full rounded-md border border-rule bg-void/60 px-2 py-1 text-[12.5px] text-ink " +
  "focus:border-class-a focus:outline-none";

const LABEL = "font-mono text-[10px] tracking-wide text-faint uppercase";

export default function Form({ data }: { data: FormWidget }) {
  const key = draftKey(useTurnId(), data);
  const draft = useForms((state) => state.drafts[key]);
  const setValue = useForms((state) => state.setValue);
  const send = useChat((state) => state.send);
  const streaming = useChat((state) => state.status === "streaming");

  const values = Object.fromEntries(
    data.fields.map((field) => [field.name, draft?.[field.name] ?? defaultOf(field)]),
  );
  // Every field is one the model asked for, so a blank is an unanswered
  // question rather than an omission worth sending.
  const incomplete = data.fields.some((field) => values[field.name] === "");

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (incomplete || streaming) return;
    const filled = data.submit ? fill(data.submit, data.fields, values) : null;
    void send(filled || labelled(data, values));
  };

  return (
    <form
      onSubmit={submit}
      className="my-4 overflow-hidden rounded-lg border border-rule bg-dust/40"
    >
      {data.title ? (
        <p className="border-b border-rule px-3 py-2 font-mono text-[10px] tracking-wide text-muted uppercase">
          {data.title}
        </p>
      ) : null}

      {/* Label beside the control rather than above it: five stacked fields is
          most of the Quick Bar, and the answer has to still be on screen. */}
      <div className="grid grid-cols-[minmax(0,8rem)_1fr] items-center gap-x-3 gap-y-2 px-3 py-3">
        {data.fields.map((field) => {
          const id = `${key}-${field.name}`;
          const value = values[field.name];

          return (
            <Fragment key={field.name}>
              <label htmlFor={id} className={LABEL}>
                {field.label}
              </label>
              {field.kind === "checkbox" ? (
                <input
                  id={id}
                  type="checkbox"
                  checked={value === true}
                  onChange={(event) => setValue(key, field.name, event.target.checked)}
                  className="size-3.5 justify-self-start accent-class-a"
                />
              ) : field.kind === "select" ? (
                <select
                  id={id}
                  value={String(value)}
                  onChange={(event) => setValue(key, field.name, event.target.value)}
                  className={CONTROL}
                >
                  {field.options.map((option) => (
                    <option key={option} value={option}>
                      {option}
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  id={id}
                  type={field.kind === "number" ? "number" : "text"}
                  value={String(value)}
                  onChange={(event) => setValue(key, field.name, event.target.value)}
                  className={CONTROL}
                />
              )}
            </Fragment>
          );
        })}
      </div>

      <div className="border-t border-rule px-3 py-2">
        <button
          type="submit"
          disabled={incomplete || streaming}
          className="font-mono text-[10px] tracking-wider text-class-a uppercase disabled:text-faint"
        >
          Ask
        </button>
      </div>
    </form>
  );
}

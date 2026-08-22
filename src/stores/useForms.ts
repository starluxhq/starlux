import { create } from "zustand";

export type FormValue = string | boolean;

interface FormState {
  drafts: Record<string, Record<string, FormValue>>;
  setValue: (key: string, name: string, value: FormValue) => void;
}

/** Held outside the component because a form is rendered from an assistant
 *  turn's markdown: `Answer` re-renders it on every delta and replaces the text
 *  wholesale when the run ends, and neither leaves component state standing. */
export const useForms = create<FormState>((set) => ({
  drafts: {},
  setValue: (key, name, value) =>
    set((state) => ({
      drafts: { ...state.drafts, [key]: { ...state.drafts[key], [name]: value } },
    })),
}));

/** Identity has to come from the payload, since a renderer is handed the fence
 *  body and nothing else. The turn scopes it so the same form asked twice keeps
 *  two answers. */
export function draftKey(turnId: string, payload: unknown): string {
  const text = JSON.stringify(payload);
  let hash = 5381;
  for (let index = 0; index < text.length; index += 1) {
    hash = ((hash << 5) + hash + text.charCodeAt(index)) | 0;
  }
  return `${turnId}:${(hash >>> 0).toString(36)}`;
}

import type { Model } from "./types";

/** `opus` is what the CLI is given; `Opus` is what the bar shows. */
const capitalise = (model: string) => model.charAt(0).toUpperCase() + model.slice(1);

/** Just the model's own name, for where the vendor is already named above or
 *  beside it. Whether to capitalise is decided from the whole id: strip the
 *  vendor first and `opencode-go/glm-5.3` comes back as `Glm-5.3`. */
export const modelName = (model: string) =>
  model.includes("/") ? model.slice(model.lastIndexOf("/") + 1) : capitalise(model);

export interface Vendor {
  /** `null` where the ids carry no vendor at all, as Claude's `opus` does. */
  name: string | null;
  models: Model[];
}

/** opencode answers `models` with every vendor it can reach at once — thirty
 *  ids from two accounts, in one flat list — so the half you are signed in to
 *  is not apparent from the list. The prefix is a provider id opencode takes
 *  back (`opencode models <provider>`), which makes it the honest heading.
 *
 *  Vendors keep the order the CLI gave them rather than being alphabetised:
 *  a list that arrives ranked should stay ranked. */
export const byVendor = (models: Model[]): Vendor[] =>
  models.reduce<Vendor[]>((vendors, model) => {
    const slash = model.id.indexOf("/");
    const name = slash === -1 ? null : model.id.slice(0, slash);
    const group = vendors.find((vendor) => vendor.name === name);
    if (group) group.models.push(model);
    else vendors.push({ name, models: [model] });
    return vendors;
  }, []);

/** Marks both halves of the model picker, so a click anywhere in it is not a
 *  dismissal. Lives here rather than beside them: a component file that exports
 *  anything else gives up Fast Refresh for everything in it. */
export const PICKER = "data-model-picker";

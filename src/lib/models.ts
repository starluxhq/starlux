/** `opus` is what the CLI is given; `Opus` is what the bar shows. A provider
 *  that names its models `vendor/model` is left alone: both halves carry
 *  meaning, and there are enough of them that the prefix is what tells two
 *  apart. */
export const modelLabel = (model: string) =>
  model.includes("/") ? model : model.charAt(0).toUpperCase() + model.slice(1);

/** Just the model's own name, for where the provider is already named beside
 *  it. Whether to capitalise is decided from the whole id: strip the vendor
 *  first and `opencode-go/glm-5.3` comes back as `Glm-5.3`. */
export const modelName = (model: string) =>
  model.includes("/") ? model.slice(model.lastIndexOf("/") + 1) : modelLabel(model);

/** Marks both halves of the model picker, so a click anywhere in it is not a
 *  dismissal. Lives here rather than beside them: a component file that exports
 *  anything else gives up Fast Refresh for everything in it. */
export const PICKER = "data-model-picker";

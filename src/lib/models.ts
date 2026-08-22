/** `opus` is what the CLI is given; `Opus` is what the bar shows. */
export const modelLabel = (model: string) => model.charAt(0).toUpperCase() + model.slice(1);

/** Marks both halves of the model picker, so a click anywhere in it is not a
 *  dismissal. Lives here rather than beside them: a component file that exports
 *  anything else gives up Fast Refresh for everything in it. */
export const PICKER = "data-model-picker";

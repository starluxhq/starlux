export type Platform = "macos" | "windows" | "linux";

/** What the window has to draw for itself. macOS keeps its traffic lights and
 *  only makes the bar around them transparent; Linux has no vibrancy API, so
 *  the surface behind the page is opaque there. */
export const platform: Platform = navigator.userAgent.includes("Mac")
  ? "macos"
  : navigator.userAgent.includes("Windows")
    ? "windows"
    : "linux";

/** Tags the document so the stylesheet can pick a surface treatment. */
export function markPlatform() {
  document.documentElement.dataset.platform = platform;
}

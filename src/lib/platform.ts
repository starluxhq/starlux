/** Tags the document so the stylesheet can pick a surface treatment: Linux has
 *  no vibrancy API, macOS and Windows composite it behind the webview. */
export function markPlatform() {
  const agent = navigator.userAgent;
  const platform = agent.includes("Mac")
    ? "macos"
    : agent.includes("Windows")
      ? "windows"
      : "linux";
  document.documentElement.dataset.platform = platform;
}

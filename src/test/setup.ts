import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// Without `globals`, React Testing Library never registers its own.
afterEach(cleanup);

// Enough of the bridge for a component tree to be built outside the app. Some
// modules reach for their own window at import time, so this cannot be left to
// the tests that happen to need it.
Object.defineProperty(window, "__TAURI_INTERNALS__", {
  value: {
    metadata: {
      currentWindow: { label: "workspace" },
      currentWebview: { label: "workspace", windowLabel: "workspace" },
    },
    transformCallback: (callback: unknown) => callback,
    invoke: () => Promise.resolve(null),
    plugins: {},
  },
  writable: true,
});

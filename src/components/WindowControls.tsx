import { useEffect, useState } from "react";
import {
  closeWindow,
  minimiseWindow,
  onWindowResized,
  toggleMaximiseWindow,
  windowIsMaximised,
} from "../lib/chrome";

const BUTTON =
  "flex size-7 shrink-0 items-center justify-center rounded-md text-muted hover:bg-white/6 hover:text-ink";

const STROKE = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.4,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

/** The buttons the toolkit stopped drawing. Not rendered on macOS, where the
 *  traffic lights are still the system's own. */
export default function WindowControls() {
  const [maximised, setMaximised] = useState(false);

  useEffect(() => {
    const sync = () => void windowIsMaximised().then(setMaximised);
    sync();
    return onWindowResized(sync);
  }, []);

  return (
    <div className="flex shrink-0 items-center gap-0.5">
      <button
        type="button"
        onClick={() => void minimiseWindow()}
        aria-label="Minimise"
        className={BUTTON}
      >
        <svg viewBox="0 0 16 16" aria-hidden className="size-3">
          <path d="M3.5 8h9" {...STROKE} />
        </svg>
      </button>

      <button
        type="button"
        onClick={() => void toggleMaximiseWindow()}
        aria-label={maximised ? "Restore" : "Maximise"}
        className={BUTTON}
      >
        <svg viewBox="0 0 16 16" aria-hidden className="size-3">
          {maximised ? (
            <path d="M5.5 5.5v-2h7v7h-2M3.5 6.5h7v6h-7z" {...STROKE} />
          ) : (
            <path d="M3.5 3.5h9v9h-9z" {...STROKE} />
          )}
        </svg>
      </button>

      <button
        type="button"
        onClick={() => void closeWindow()}
        aria-label="Close"
        className={`${BUTTON} hover:bg-class-m/15 hover:text-class-m`}
      >
        <svg viewBox="0 0 16 16" aria-hidden className="size-3">
          <path d="M4 4l8 8M12 4l-8 8" {...STROKE} />
        </svg>
      </button>
    </div>
  );
}

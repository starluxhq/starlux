import { useEffect } from "react";
import { hideQuickBar, openWorkspace } from "../lib/ipc";

export default function QuickBar() {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void hideQuickBar();
        return;
      }
      if (event.key.toLowerCase() === "e" && (event.metaKey || event.ctrlKey)) {
        event.preventDefault();
        void openWorkspace();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <div className="flex h-full flex-col overflow-hidden rounded-xl border border-white/10 bg-neutral-950/80 backdrop-blur-xl">
      <div className="flex items-center gap-3 px-5 py-4">
        <span className="text-sm text-neutral-500">Ask anything</span>
      </div>
      <div className="flex-1 overflow-y-auto border-t border-white/5 px-5 py-4 text-sm text-neutral-400">
        <button
          type="button"
          onClick={() => void openWorkspace()}
          className="rounded-md border border-white/10 px-3 py-1.5 text-xs text-neutral-300 hover:bg-white/5"
        >
          Expand
        </button>
      </div>
    </div>
  );
}

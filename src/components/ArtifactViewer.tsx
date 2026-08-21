import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { storeArtifact } from "../lib/ipc";

interface ArtifactViewerProps {
  html: string;
  title: string;
  /** `inline` sits in the thread at a fixed height; `pane` fills its column. */
  variant: "inline" | "pane";
  onExpand?: () => void;
  onCollapse?: () => void;
}

export default function ArtifactViewer({
  html,
  title,
  variant,
  onExpand,
  onCollapse,
}: ArtifactViewerProps) {
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let current = true;
    setFailed(false);
    void storeArtifact(html).then(
      (id) => {
        if (current) setSrc(convertFileSrc(id, "artifact"));
      },
      () => {
        if (current) setFailed(true);
      },
    );
    return () => {
      current = false;
    };
  }, [html]);

  return (
    <figure
      className={
        variant === "pane"
          ? "flex h-full min-h-0 flex-col overflow-hidden bg-dust/40"
          : "my-4 flex flex-col overflow-hidden rounded-lg border border-rule bg-dust/40"
      }
    >
      <figcaption className="flex shrink-0 items-center justify-between border-b border-rule px-3 py-2">
        <span className="truncate font-mono text-[10px] tracking-wide text-muted uppercase">
          {title}
        </span>
        {onExpand ? (
          <button
            type="button"
            onClick={onExpand}
            className="shrink-0 font-mono text-[10px] tracking-wide text-muted uppercase hover:text-ink"
          >
            Expand
          </button>
        ) : null}
        {onCollapse ? (
          <button
            type="button"
            onClick={onCollapse}
            aria-label="Close artifact"
            className="shrink-0 font-mono text-[10px] tracking-wide text-muted uppercase hover:text-ink"
          >
            Close
          </button>
        ) : null}
      </figcaption>

      {failed ? (
        <p className="px-3 py-4 text-[12.5px] text-class-m">This artifact could not be prepared.</p>
      ) : (
        // `allow-same-origin` is deliberately absent: with it, a frame can reach
        // through and drop its own sandbox, which would put Tauri's IPC back in
        // reach of whatever the model wrote.
        <iframe
          key={src ?? "pending"}
          src={src ?? undefined}
          title={title}
          sandbox="allow-scripts"
          referrerPolicy="no-referrer"
          className={
            variant === "pane" ? "min-h-0 flex-1 border-0 bg-white" : "h-80 border-0 bg-white"
          }
        />
      )}
    </figure>
  );
}

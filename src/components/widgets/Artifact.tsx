import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { CustomRendererProps } from "streamdown";
import { useArtifact } from "../../stores/useArtifact";
import ArtifactViewer from "../ArtifactViewer";

/** Only the Workspace has a column to put an expanded artifact in. */
const canExpand = getCurrentWebviewWindow().label === "workspace";

function titleFrom(meta: string | undefined): string {
  const quoted = meta?.match(/title="([^"]*)"/);
  return quoted?.[1]?.trim() || "Artifact";
}

export default function Artifact({ code, isIncomplete, meta }: CustomRendererProps) {
  const expand = useArtifact((state) => state.expand);
  const title = titleFrom(meta);

  // Framing a half-written document would run it against markup that has not
  // been closed yet, and reload it on every delta after that.
  if (isIncomplete) {
    return (
      <div
        className="my-4 h-80 animate-pulse rounded-lg border border-rule bg-dust/40"
        aria-hidden
      />
    );
  }

  return (
    <ArtifactViewer
      html={code}
      title={title}
      variant="inline"
      onExpand={canExpand ? () => expand({ html: code, title }) : undefined}
    />
  );
}

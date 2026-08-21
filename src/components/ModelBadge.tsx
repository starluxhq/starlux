import SpectralDot from "./SpectralDot";

interface ModelBadgeProps {
  providerId: string;
  name: string;
  model?: string | null;
}

export default function ModelBadge({ providerId, name, model }: ModelBadgeProps) {
  return (
    <span className="inline-flex items-center gap-1.5 font-mono text-[10px] tracking-wide text-muted uppercase">
      <SpectralDot providerId={providerId} />
      {name}
      {model ? <span className="text-faint">· {model}</span> : null}
    </span>
  );
}

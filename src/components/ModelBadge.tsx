import { spectralClass } from "../lib/types";

interface ModelBadgeProps {
  providerId: string;
  name: string;
  model?: string | null;
}

const DOT: Record<string, string> = {
  a: "bg-class-a",
  f: "bg-class-f",
  g: "bg-class-g",
  k: "bg-class-k",
  m: "bg-class-m",
};

export default function ModelBadge({ providerId, name, model }: ModelBadgeProps) {
  return (
    <span className="inline-flex items-center gap-1.5 font-mono text-[10px] tracking-wide text-muted uppercase">
      <span className={`size-[5px] rounded-full ${DOT[spectralClass(providerId)]}`} />
      {name}
      {model ? <span className="text-faint">· {model}</span> : null}
    </span>
  );
}

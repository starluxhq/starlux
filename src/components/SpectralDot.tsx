import { spectralClass } from "../lib/types";

const COLOR: Record<string, string> = {
  a: "bg-class-a",
  f: "bg-class-f",
  g: "bg-class-g",
  k: "bg-class-k",
  m: "bg-class-m",
};

/** Marks which provider a thing came from without spending a word on it. */
export default function SpectralDot({
  providerId,
  className = "",
}: {
  providerId: string;
  className?: string;
}) {
  return (
    <span
      aria-hidden
      className={`size-[5px] shrink-0 rounded-full ${COLOR[spectralClass(providerId)]} ${className}`}
    />
  );
}

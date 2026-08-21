import type { Status } from "../stores/useChat";

interface RailProps {
  status: Status;
  className?: string;
}

/** The spectrum rail: idle hairline, travelling spectrum while streaming. */
export default function Rail({ status, className = "" }: RailProps) {
  const state = status === "streaming" ? "streaming" : status === "error" ? "error" : "idle";
  return (
    <div
      aria-hidden
      data-state={state}
      className={`rail w-[2px] shrink-0 rounded-full ${className}`}
    />
  );
}

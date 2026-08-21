interface KeycapProps {
  children: string;
  label: string;
}

export default function Keycap({ children, label }: KeycapProps) {
  return (
    <span className="inline-flex items-center gap-1.5 text-[10px] text-faint">
      <kbd className="rounded border border-rule bg-haze px-1.5 py-0.5 font-mono text-[10px] text-muted">
        {children}
      </kbd>
      {label}
    </span>
  );
}

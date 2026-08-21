import { useEffect, useRef } from "react";

interface ComposerProps {
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  maxRows?: number;
}

export default function Composer({
  value,
  placeholder,
  onChange,
  onSubmit,
  maxRows = 5,
}: ComposerProps) {
  const ref = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    ref.current?.focus();
  }, []);

  useEffect(() => {
    const textarea = ref.current;
    if (!textarea) return;
    textarea.style.height = "auto";
    const lineHeight = 22;
    textarea.style.height = `${Math.min(textarea.scrollHeight, lineHeight * maxRows)}px`;
  }, [value, maxRows]);

  return (
    <div className="flex items-start gap-3">
      <span aria-hidden className="pt-px font-mono text-[15px] leading-[22px] text-class-k">
        ›
      </span>
      <textarea
        ref={ref}
        rows={1}
        value={value}
        placeholder={placeholder}
        spellCheck={false}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            onSubmit();
          }
        }}
        className="w-full resize-none bg-transparent text-[15px] leading-[22px] text-ink outline-none placeholder:text-faint"
      />
    </div>
  );
}

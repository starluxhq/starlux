import { useCallback, useEffect, useRef } from "react";

interface ComposerProps {
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  maxRows?: number;
  /** Off in the Quick Bar, where the attach button already marks the left edge. */
  marker?: boolean;
}

const LINE_HEIGHT = 22;

export default function Composer({
  value,
  placeholder,
  onChange,
  onSubmit,
  maxRows = 5,
  marker = true,
}: ComposerProps) {
  const ref = useRef<HTMLTextAreaElement>(null);

  const fit = useCallback(() => {
    const textarea = ref.current;
    // Until the row has been laid out the textarea has no width, and WebKit
    // then reports a scrollHeight with no relation to the content.
    if (!textarea || textarea.clientWidth === 0) return;
    textarea.style.height = "0px";
    textarea.style.height = `${Math.min(textarea.scrollHeight, LINE_HEIGHT * maxRows)}px`;
  }, [maxRows]);

  useEffect(() => {
    ref.current?.focus();
  }, []);

  useEffect(fit, [fit, value]);

  useEffect(() => {
    const row = ref.current?.parentElement;
    if (!row) return;
    const observer = new ResizeObserver(fit);
    observer.observe(row);
    return () => observer.disconnect();
  }, [fit]);

  return (
    <div className="flex min-w-0 flex-1 items-start gap-3">
      {marker ? (
        <span aria-hidden className="pt-px font-mono text-[15px] leading-[22px] text-class-k">
          ›
        </span>
      ) : null}
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
        className="min-w-0 flex-1 resize-none bg-transparent text-[15px] leading-[22px] text-ink outline-none placeholder:text-faint"
      />
    </div>
  );
}

import rehypeKatex from "rehype-katex";
import remarkMath from "remark-math";
import type { MathPlugin } from "streamdown";

export const math: MathPlugin = {
  name: "katex",
  type: "math",
  remarkPlugin: remarkMath,
  // A half-typed equation is the normal state while a run streams, so KaTeX must
  // render what it can instead of throwing on the first unbalanced brace.
  rehypePlugin: [rehypeKatex, { throwOnError: false, errorColor: "#ff6f5e" }],
};

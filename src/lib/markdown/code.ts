import type { CodeHighlighterPlugin, HighlightOptions, ThemeInput } from "streamdown";

/** Streamdown declares this shape but does not export it. */
type HighlightResult = NonNullable<ReturnType<CodeHighlighterPlugin["highlight"]>>;

/** Starlux only ever renders on a dark surface, so both theme slots take one. */
const THEME = "vitesse-dark";

/** Grammars are imported one by one rather than through Shiki's bundle, which
 *  reaches every language it ships and drags in the Oniguruma wasm. Each entry
 *  here is a chunk fetched the first time a block claims that language. */
const GRAMMARS = {
  bash: () => import("@shikijs/langs/bash"),
  c: () => import("@shikijs/langs/c"),
  cpp: () => import("@shikijs/langs/cpp"),
  css: () => import("@shikijs/langs/css"),
  diff: () => import("@shikijs/langs/diff"),
  go: () => import("@shikijs/langs/go"),
  html: () => import("@shikijs/langs/html"),
  java: () => import("@shikijs/langs/java"),
  javascript: () => import("@shikijs/langs/javascript"),
  json: () => import("@shikijs/langs/json"),
  lua: () => import("@shikijs/langs/lua"),
  markdown: () => import("@shikijs/langs/markdown"),
  python: () => import("@shikijs/langs/python"),
  ruby: () => import("@shikijs/langs/ruby"),
  rust: () => import("@shikijs/langs/rust"),
  sql: () => import("@shikijs/langs/sql"),
  swift: () => import("@shikijs/langs/swift"),
  toml: () => import("@shikijs/langs/toml"),
  tsx: () => import("@shikijs/langs/tsx"),
  typescript: () => import("@shikijs/langs/typescript"),
  yaml: () => import("@shikijs/langs/yaml"),
} as const;

/** What a fence may say to mean one of the grammars above. */
const ALIASES: Record<string, keyof typeof GRAMMARS> = {
  "c++": "cpp",
  js: "javascript",
  jsx: "tsx",
  md: "markdown",
  py: "python",
  rb: "ruby",
  rs: "rust",
  sh: "bash",
  shell: "bash",
  ts: "typescript",
  yml: "yaml",
  zsh: "bash",
};

function resolve(language: string): keyof typeof GRAMMARS | null {
  const name = language.toLowerCase();
  if (name in GRAMMARS) return name as keyof typeof GRAMMARS;
  return ALIASES[name] ?? null;
}

interface Highlighter {
  codeToTokens: (code: string, options: Record<string, unknown>) => HighlightResult;
}

let loading: Promise<Highlighter> | null = null;
let ready: Highlighter | null = null;

/** Deferred so Shiki stays out of the window's first paint. The JavaScript
 *  regex engine is used in place of Oniguruma to keep the wasm blob out of the
 *  bundle; `forgiving` drops the few patterns it cannot compile rather than
 *  failing the whole grammar. */
function highlighter(): Promise<Highlighter> {
  loading ??= Promise.all([
    import("shiki/core"),
    import("shiki/engine/javascript"),
    import("@shikijs/themes/vitesse-dark"),
  ]).then(([core, engine, theme]) =>
    core.createHighlighterCore({
      themes: [theme.default],
      langs: Object.values(GRAMMARS),
      engine: engine.createJavaScriptRegexEngine({ forgiving: true }),
    }),
  ) as Promise<Highlighter>;
  return loading;
}

/** Bounded so a long conversation cannot pin every block it has ever shown. */
const CACHE_LIMIT = 200;
const cache = new Map<string, HighlightResult>();

function remember(key: string, result: HighlightResult): HighlightResult {
  if (cache.size >= CACHE_LIMIT) {
    const oldest = cache.keys().next().value;
    if (oldest !== undefined) cache.delete(oldest);
  }
  cache.set(key, result);
  return result;
}

function tokenize(instance: Highlighter, code: string, lang: string): HighlightResult {
  return instance.codeToTokens(code, { lang, theme: THEME });
}

export const code: CodeHighlighterPlugin = {
  name: "shiki",
  type: "code-highlighter",
  getThemes: () => [THEME as ThemeInput, THEME as ThemeInput],
  getSupportedLanguages: () => Object.keys(GRAMMARS) as never,
  supportsLanguage: (language) => resolve(language) !== null,

  highlight(options: HighlightOptions, callback) {
    const lang = resolve(options.language);
    if (!lang) return null;

    const key = `${lang} ${options.code}`;
    const hit = cache.get(key);
    if (hit) return hit;

    // Synchronous once the grammars are in, which keeps a streaming block from
    // flashing back to unhighlighted on every delta.
    if (ready) return remember(key, tokenize(ready, options.code, lang));

    void highlighter().then((instance) => {
      ready = instance;
      callback?.(remember(key, tokenize(instance, options.code, lang)));
    });
    return null;
  },
};

// A tiny, dependency-free syntax highlighter. Input is trusted (our own code
// strings); we still HTML-escape every segment before wrapping it in a span.

export type Lang = "red" | "ts" | "graphql" | "yaml" | "bash";

const KEYWORDS: Record<Lang, Set<string>> = {
  red: new Set([
    "abi", "from", "entity", "immutable", "timeseries", "source", "template",
    "handler", "on", "call", "block", "every", "once", "fn", "let", "match",
    "mod", "use", "return", "if", "else", "while", "for", "in", "derived",
    "interface", "implements", "enum", "aggregation", "over", "test",
    "Ok", "Err", "Some", "None", "network", "address", "startBlock", "kind",
  ]),
  ts: new Set([
    "import", "from", "export", "function", "return", "const", "let", "new",
    "void", "if", "else", "class", "extends", "as",
  ]),
  graphql: new Set(["type", "interface", "enum", "implements"]),
  yaml: new Set([]),
  bash: new Set([]),
};

function esc(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function highlightCode(src: string, lang: Lang): string {
  const kw = KEYWORDS[lang];
  const re = /("(?:[^"\\]|\\.)*")|(0x[0-9a-fA-F]+|\b\d[\d_.]*\b)|([A-Za-z_]\w*)/g;
  let out = "";
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(src))) {
    out += esc(src.slice(last, m.index));
    const tok = m[0];
    if (m[1]) {
      out += `<span class="tok-str">${esc(tok)}</span>`;
    } else if (m[2]) {
      out += `<span class="tok-num">${esc(tok)}</span>`;
    } else if (kw.has(tok)) {
      out += `<span class="tok-kw">${tok}</span>`;
    } else if (/^[A-Z]/.test(tok)) {
      out += `<span class="tok-type">${tok}</span>`;
    } else {
      out += esc(tok);
    }
    last = re.lastIndex;
  }
  out += esc(src.slice(last));
  return out;
}

export function highlight(code: string, lang: Lang): string {
  const commentMark =
    lang === "graphql" || lang === "yaml" || lang === "bash" ? "#" : "//";
  return code
    .split("\n")
    .map((line) => {
      const idx = line.indexOf(commentMark);
      if (idx >= 0) {
        return (
          highlightCode(line.slice(0, idx), lang) +
          `<span class="tok-comment">${esc(line.slice(idx))}</span>`
        );
      }
      return highlightCode(line, lang);
    })
    .join("\n");
}

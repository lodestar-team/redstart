"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { highlight } from "@/lib/highlight";

// Bundled ABI the starter snippet imports as `ERC20`, so it type-checks fully.
const ABIS = JSON.stringify({
  ERC20: [
    { type: "event", name: "Transfer", inputs: [
      { name: "from", type: "address", indexed: true },
      { name: "to", type: "address", indexed: true },
      { name: "value", type: "uint256", indexed: false },
    ] },
    { type: "event", name: "Approval", inputs: [
      { name: "owner", type: "address", indexed: true },
      { name: "spender", type: "address", indexed: true },
      { name: "value", type: "uint256", indexed: false },
    ] },
    { type: "function", name: "balanceOf", stateMutability: "view",
      inputs: [{ name: "account", type: "address" }],
      outputs: [{ name: "", type: "uint256" }] },
  ],
});

const DEFAULT_SOURCE = `// Edit me — the panels regenerate as you type.
abi ERC20 from "./abis/ERC20.json"

entity Account {
  id: Id<Bytes>
  balance: BigInt
  label: Option<String>          // nullable — there is no \`null\`
}

entity Transfer immutable {
  id: Id<Bytes>
  from: Account
  to: Account
  value: BigInt
  timestamp: BigInt
}

source Token {
  abi: ERC20
  network: mainnet
  address: 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
  startBlock: 6082465
}

handler on Token.Transfer(event) {
  let sender = Account.loadOrCreate(event.params.from, { balance: BigInt.zero })
  let receiver = Account.loadOrCreate(event.params.to, { balance: BigInt.zero })

  sender.balance = sender.balance - event.params.value
  receiver.balance = receiver.balance + event.params.value

  Transfer.create(event.id, {
    from: event.params.from,
    to: event.params.to,
    value: event.params.value,
    timestamp: event.block.timestamp,
  })
}
`;

type Compiled = {
  ok: boolean;
  schema: string;
  manifest: string;
  mappings: string;
  diagnostics: string[];
  warnings: string[];
};

type CompileFn = (source: string, abisJson: string) => Compiled;

const TABS = [
  { key: "mappings", label: "mappings.ts", lang: "ts" as const },
  { key: "schema", label: "schema.graphql", lang: "graphql" as const },
  { key: "manifest", label: "subgraph.yaml", lang: "yaml" as const },
] as const;

type TabKey = (typeof TABS)[number]["key"];

export function Playground() {
  const compileRef = useRef<CompileFn | null>(null);
  const [source, setSource] = useState(DEFAULT_SOURCE);
  const [result, setResult] = useState<Compiled | null>(null);
  const [tab, setTab] = useState<TabKey>("mappings");
  const [status, setStatus] = useState<"loading" | "ok" | "warn" | "err">("loading");
  const [statusText, setStatusText] = useState("loading engine…");

  const runWith = useCallback((compile: CompileFn, src: string) => {
    let r: Compiled;
    try {
      r = compile(src, ABIS);
    } catch (e) {
      setStatus("err");
      setStatusText("engine error");
      setResult({ ok: false, schema: "", manifest: "", mappings: "", diagnostics: [String(e)], warnings: [] });
      return;
    }
    setResult(r);
    if (r.ok) {
      setStatus(r.warnings.length ? "warn" : "ok");
      setStatusText(r.warnings.length ? `${r.warnings.length} warning(s)` : "compiled");
    } else {
      setStatus("err");
      setStatusText(`${r.diagnostics.length} error(s)`);
    }
  }, []);

  // Load the WASM engine once, client-side.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        // Loaded at runtime from /public, not bundled — hence the bundler ignore
        // hints and the suppression: there is no module here for TS to resolve.
        // @ts-expect-error runtime asset, resolved by the browser at /wasm/…
        const mod = await import(/* webpackIgnore: true */ /* turbopackIgnore: true */ "/wasm/redstart_wasm.js");
        await mod.default();
        if (cancelled) return;
        compileRef.current = mod.compile as CompileFn;
        runWith(mod.compile as CompileFn, DEFAULT_SOURCE);
      } catch (e) {
        if (cancelled) return;
        setStatus("err");
        setStatusText("failed to load engine");
        setResult({ ok: false, schema: "", manifest: "", mappings: "", diagnostics: [String(e)], warnings: [] });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [runWith]);

  // Debounced recompile on edit.
  useEffect(() => {
    if (!compileRef.current) return;
    setStatus((s) => (s === "loading" ? s : s));
    const id = setTimeout(() => {
      if (compileRef.current) runWith(compileRef.current, source);
    }, 180);
    return () => clearTimeout(id);
  }, [source, runWith]);

  const errored = status === "err" && result ? result.diagnostics : null;
  const activeLang = TABS.find((t) => t.key === tab)!.lang;
  const activeCode = result ? (result as Compiled)[tab as "mappings" | "schema" | "manifest"] : "";

  return (
    <div className="grid h-full min-h-0 grid-cols-1 lg:grid-cols-2">
      {/* Editor */}
      <div className="flex min-h-0 flex-col border-b border-line lg:border-b-0 lg:border-r">
        <PaneHead label="source.red">
          <StatusDot status={status} text={statusText} />
        </PaneHead>
        <textarea
          value={source}
          onChange={(e) => setSource(e.target.value)}
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          className="flex-1 resize-none bg-surface p-5 font-mono text-[0.82rem] leading-[1.65] text-ink-soft outline-none"
        />
      </div>

      {/* Output */}
      <div className="flex min-h-0 flex-col">
        <div className="flex items-center gap-1 border-b border-line bg-[#f9f9f8] px-2 py-1.5">
          {TABS.map((t) => (
            <TabButton key={t.key} active={tab === t.key} onClick={() => setTab(t.key)}>
              {t.label}
            </TabButton>
          ))}
          <TabButton active={tab === ("diagnostics" as TabKey)} onClick={() => setTab("diagnostics" as TabKey)} danger={!!errored}>
            diagnostics
          </TabButton>
        </div>

        {tab === ("diagnostics" as TabKey) ? (
          <pre className="flex-1 overflow-auto bg-surface p-5 font-mono text-[0.8rem] leading-[1.7]">
            {errored ? (
              <span className="text-red">{errored.join("\n\n")}</span>
            ) : result?.warnings.length ? (
              <span className="text-[#956400]">
                {result.warnings.map((w) => `warning: ${w}`).join("\n")}
              </span>
            ) : (
              <span className="text-muted">No diagnostics. Clean build.</span>
            )}
          </pre>
        ) : (
          <pre className="flex-1 overflow-auto bg-surface p-5 font-mono text-[0.82rem] leading-[1.65] text-ink-soft">
            <code dangerouslySetInnerHTML={{ __html: highlight(activeCode || "", activeLang) }} />
          </pre>
        )}
      </div>
    </div>
  );
}

function PaneHead({ label, children }: { label: string; children?: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between border-b border-line bg-[#f9f9f8] px-4 py-2 font-mono text-xs text-faint">
      <span>{label}</span>
      {children}
    </div>
  );
}

function StatusDot({ status, text }: { status: string; text: string }) {
  const color =
    status === "ok" ? "bg-[#346538]" : status === "warn" ? "bg-[#956400]" : status === "err" ? "bg-red" : "bg-faint";
  return (
    <span className="inline-flex items-center gap-1.5">
      <span className={`h-1.5 w-1.5 rounded-full ${color}`} />
      {text}
    </span>
  );
}

function TabButton({
  active,
  danger,
  onClick,
  children,
}: {
  active: boolean;
  danger?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`rounded-md px-3 py-1 font-mono text-xs transition-colors ${
        active
          ? "bg-surface text-ink shadow-[0_1px_2px_rgba(0,0,0,0.04)]"
          : danger
            ? "text-red hover:text-red-ink"
            : "text-faint hover:text-muted"
      }`}
    >
      {children}
    </button>
  );
}

"use client";

import { useEffect, useMemo, useState } from "react";
import { CHAINS, DEFAULT_CHAIN_ID } from "@/lib/chains";
import { type AnthropicError, generateSubgraph, type GeneratedSubgraph } from "@/lib/generate";
import { downloadProject, type VerifyResult, verifyProject } from "@/lib/project";
import type { ContractInfo, EventDef } from "@/lib/subgraph-abi";

const KEY_STORAGE = "redstart.anthropicKey";

const KIND_LABEL: Record<ContractInfo["kind"], string> = {
  erc20: "ERC-20 token",
  erc721: "ERC-721 NFT",
  erc1155: "ERC-1155 multi-token",
  unknown: "contract",
};

export function Generator() {
  const [address, setAddress] = useState("");
  const [chainId, setChainId] = useState(DEFAULT_CHAIN_ID);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [contract, setContract] = useState<ContractInfo | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());

  // BYOK: the Anthropic key lives only in sessionStorage — never sent to us.
  const [apiKey, setApiKey] = useState("");
  const [generating, setGenerating] = useState(false);
  const [genError, setGenError] = useState<string | null>(null);
  const [generated, setGenerated] = useState<GeneratedSubgraph | null>(null);

  useEffect(() => {
    // Hydrate the key from sessionStorage on mount (client-only; not available
    // during SSR, so this can't be a lazy initializer without a hydration mismatch).
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setApiKey(sessionStorage.getItem(KEY_STORAGE) ?? "");
  }, []);

  function saveKey(k: string) {
    setApiKey(k);
    if (k) sessionStorage.setItem(KEY_STORAGE, k);
    else sessionStorage.removeItem(KEY_STORAGE);
  }

  async function generate() {
    if (!contract || generating || !apiKey) return;
    setGenerating(true);
    setGenError(null);
    setGenerated(null);
    const events = contract.events.filter((e) => selected.has(e.signature));
    try {
      const result = await generateSubgraph(apiKey, contract, events);
      setGenerated(result);
    } catch (e) {
      setGenError((e as AnthropicError).message ?? "Generation failed.");
    } finally {
      setGenerating(false);
    }
  }

  const canLookup = /^0x[a-fA-F0-9]{40}$/.test(address.trim());

  async function lookup() {
    if (!canLookup || loading) return;
    setLoading(true);
    setError(null);
    setContract(null);
    try {
      const res = await fetch(
        `/api/contract?address=${encodeURIComponent(address.trim())}&chainId=${chainId}`,
      );
      const data = await res.json();
      if (!res.ok) {
        setError(data.error ?? "Lookup failed.");
        return;
      }
      const info = data as ContractInfo;
      setContract(info);
      setSelected(new Set(info.events.map((e) => e.signature)));
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }

  function toggle(sig: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(sig)) next.delete(sig);
      else next.add(sig);
      return next;
    });
  }

  return (
    <div className="mx-auto w-full max-w-3xl px-4 py-10 sm:py-14">
      <header className="mb-8">
        <span className="eyebrow">The Generator</span>
        <h1 className="display mt-3 text-3xl sm:text-4xl">
          Paste a contract. Get a <span className="grad">tested subgraph.</span>
        </h1>
        <p className="mt-3 max-w-xl text-muted">
          Point it at any verified contract. It researches the ABI, proxies and start
          block, then generates best-practices AssemblyScript — verified before you
          ever see it. Your AI, your repo, your keys.
        </p>
      </header>

      {/* Input row */}
      <div className="card p-4">
        <label className="mb-1.5 block font-mono text-xs text-faint">CONTRACT ADDRESS</label>
        <div className="flex flex-col gap-2 sm:flex-row">
          <input
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && lookup()}
            placeholder="0x…"
            spellCheck={false}
            className="min-w-0 flex-1 rounded-lg border border-line-2 bg-bg-2 px-3 py-2.5 font-mono text-sm text-text outline-none placeholder:text-faint focus:border-red"
          />
          <select
            value={chainId}
            onChange={(e) => setChainId(Number(e.target.value))}
            className="rounded-lg border border-line-2 bg-bg-2 px-3 py-2.5 text-sm text-text outline-none focus:border-red"
          >
            {CHAINS.map((c) => (
              <option key={c.id} value={c.id}>
                {c.label}
              </option>
            ))}
          </select>
          <button
            onClick={lookup}
            disabled={!canLookup || loading}
            className="btn justify-center disabled:cursor-not-allowed disabled:opacity-40"
          >
            {loading ? "Researching…" : "Research contract"}
          </button>
        </div>
      </div>

      {error && (
        <div className="mt-4 rounded-lg border border-red/40 bg-red/5 px-4 py-3 text-sm text-red-bright">
          {error}
        </div>
      )}

      {contract && (
        <ContractPanel
          contract={contract}
          selected={selected}
          toggle={toggle}
          apiKey={apiKey}
          onKey={saveKey}
          onGenerate={generate}
          generating={generating}
          genError={genError}
          generated={generated}
        />
      )}
    </div>
  );
}

function ContractPanel({
  contract,
  selected,
  toggle,
  apiKey,
  onKey,
  onGenerate,
  generating,
  genError,
  generated,
}: {
  contract: ContractInfo;
  selected: Set<string>;
  toggle: (sig: string) => void;
  apiKey: string;
  onKey: (k: string) => void;
  onGenerate: () => void;
  generating: boolean;
  genError: string | null;
  generated: GeneratedSubgraph | null;
}) {
  const explorer = useMemo(() => explorerLink(contract), [contract]);
  const canGenerate = apiKey.trim().length > 0 && selected.size > 0 && !generating;

  return (
    <div className="mt-6 space-y-5">
      {/* Summary */}
      <div className="card p-4">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
          <span className="font-medium text-text">{contract.name}</span>
          <span className="tag">{KIND_LABEL[contract.kind]}</span>
          {contract.verified && <Badge tone="green">verified</Badge>}
          {contract.isProxy && <Badge tone="amber">proxy → impl ABI</Badge>}
        </div>
        <dl className="mt-3 grid grid-cols-1 gap-x-6 gap-y-1.5 font-mono text-xs text-muted sm:grid-cols-2">
          <Row label="network" value={contract.graphNetwork} />
          <Row
            label="startBlock"
            value={contract.startBlock != null ? String(contract.startBlock) : "unknown"}
          />
          {contract.implementation && (
            <Row label="implementation" value={shorten(contract.implementation)} />
          )}
          <Row
            label="address"
            value={
              <a href={explorer} target="_blank" rel="noopener noreferrer" className="hover:text-red-bright">
                {shorten(contract.address)} ↗
              </a>
            }
          />
        </dl>
      </div>

      {/* Event selection */}
      <div className="card p-4">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-medium">Events to index</h2>
          <span className="font-mono text-xs text-faint">
            {selected.size}/{contract.events.length} selected
          </span>
        </div>
        {contract.events.length === 0 ? (
          <p className="text-sm text-muted">
            This ABI declares no events. Event handlers are the fast, portable path — a
            contract with no events needs call/block handlers, which is a later feature.
          </p>
        ) : (
          <ul className="space-y-1">
            {contract.events.map((ev) => (
              <EventRow
                key={ev.signature}
                event={ev}
                checked={selected.has(ev.signature)}
                onToggle={() => toggle(ev.signature)}
              />
            ))}
          </ul>
        )}
      </div>

      {/* Connect AI + generate */}
      <div className="card p-4">
        <div className="mb-1.5 flex items-center justify-between">
          <label className="font-mono text-xs text-faint">ANTHROPIC API KEY</label>
          <span className="font-mono text-[0.68rem] text-faint">
            stored in your browser only · never sent to us
          </span>
        </div>
        <div className="flex flex-col gap-2 sm:flex-row">
          <input
            value={apiKey}
            onChange={(e) => onKey(e.target.value)}
            type="password"
            placeholder="sk-ant-…"
            spellCheck={false}
            autoComplete="off"
            className="min-w-0 flex-1 rounded-lg border border-line-2 bg-bg-2 px-3 py-2.5 font-mono text-sm text-text outline-none placeholder:text-faint focus:border-red"
          />
          <button
            onClick={onGenerate}
            disabled={!canGenerate}
            className="btn justify-center disabled:cursor-not-allowed disabled:opacity-40"
          >
            {generating ? "Generating…" : "Generate subgraph"}
          </button>
        </div>
        <p className="mt-2 text-xs text-muted">
          Your key calls Claude directly from this page. Generation runs on your account —
          get one at{" "}
          <a
            href="https://console.anthropic.com/settings/keys"
            target="_blank"
            rel="noopener noreferrer"
            className="text-red-bright hover:underline"
          >
            console.anthropic.com
          </a>
          .
        </p>
      </div>

      {genError && (
        <div className="rounded-lg border border-red/40 bg-red/5 px-4 py-3 text-sm text-red-bright">
          {genError}
        </div>
      )}

      {generated && <FilesPanel files={generated} contract={contract} />}
    </div>
  );
}

const FILE_TABS = [
  { key: "schema", label: "schema.graphql" },
  { key: "manifest", label: "subgraph.yaml" },
  { key: "mappings", label: "src/mapping.ts" },
] as const;

function FilesPanel({
  files,
  contract,
}: {
  files: GeneratedSubgraph;
  contract: ContractInfo;
}) {
  const [tab, setTab] = useState<(typeof FILE_TABS)[number]["key"]>("schema");
  const [verifying, setVerifying] = useState(false);
  const [verdict, setVerdict] = useState<VerifyResult | null>(null);
  const body = files[tab];

  // Auto-run the compile gate as soon as files are generated. A fetch-on-mount
  // with a loading flag is the intended use of an effect here.
  /* eslint-disable react-hooks/set-state-in-effect */
  useEffect(() => {
    let live = true;
    setVerdict(null);
    setVerifying(true);
    verifyProject(files, contract)
      .then((r) => live && setVerdict(r))
      .catch((e) => live && setVerdict({ ok: false, stage: "error", error: String(e?.message ?? e) }))
      .finally(() => live && setVerifying(false));
    return () => {
      live = false;
    };
  }, [files, contract]);
  /* eslint-enable react-hooks/set-state-in-effect */

  return (
    <div className="card overflow-hidden">
      <VerifyBanner verifying={verifying} verdict={verdict} />
      {files.notes && (
        <p className="border-b border-line px-4 py-3 text-sm text-muted">
          <span className="mr-1.5 text-ember">⚡</span>
          {files.notes}
        </p>
      )}
      <div className="flex items-center justify-between border-b border-line px-2">
        <div className="flex">
          {FILE_TABS.map((t) => (
            <button
              key={t.key}
              onClick={() => setTab(t.key)}
              className={`px-3 py-2.5 font-mono text-xs transition-colors ${
                tab === t.key ? "text-red-bright" : "text-faint hover:text-muted"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>
        <CopyButton text={body} />
      </div>
      <pre className="max-h-[28rem] overflow-auto p-4 font-mono text-xs leading-relaxed text-text">
        {body}
      </pre>
      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line px-4 py-3">
        <span className="text-xs text-muted">
          A complete project — schema, manifest, mappings, ABI, CI. Ejects to the
          canonical toolchain unchanged.
        </span>
        <button onClick={() => downloadProject(files, contract)} className="btn">
          Download project (.zip)
        </button>
      </div>
    </div>
  );
}

function VerifyBanner({
  verifying,
  verdict,
}: {
  verifying: boolean;
  verdict: VerifyResult | null;
}) {
  if (verifying) {
    return (
      <div className="flex items-center gap-2 border-b border-line bg-surface px-4 py-2.5 text-sm text-muted">
        <span className="h-2 w-2 animate-pulse rounded-full bg-ember" />
        Compiling with the real Graph toolchain…
      </div>
    );
  }
  if (!verdict) return null;

  if (verdict.ok) {
    return (
      <div className="border-b border-emerald-500/25 bg-emerald-500/10 px-4 py-2.5 text-sm text-emerald-400">
        ✓ Compiles — <span className="text-emerald-300/80">graph codegen &amp; graph build both passed.</span>
      </div>
    );
  }

  // Config-not-set is informational, not a failure of the generated code.
  const isConfig = verdict.stage === "config";
  return (
    <details className="border-b border-line" open={!isConfig}>
      <summary
        className={`cursor-pointer list-none px-4 py-2.5 text-sm ${
          isConfig ? "text-muted" : "text-red-bright"
        }`}
      >
        {isConfig
          ? "Compile check unavailable (verifier not configured yet)."
          : verdict.timedOut
            ? "⚠ Compile check timed out."
            : `✕ Did not compile at ${verdict.stage}. Show details`}
      </summary>
      {(verdict.log || verdict.error) && (
        <pre className="max-h-56 overflow-auto border-t border-line px-4 py-3 font-mono text-xs leading-relaxed text-muted">
          {verdict.log || verdict.error}
        </pre>
      )}
    </details>
  );
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      onClick={() => {
        navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 1200);
      }}
      className="mr-1 rounded-md px-2 py-1 font-mono text-xs text-faint transition-colors hover:bg-surface hover:text-text"
    >
      {copied ? "copied" : "copy"}
    </button>
  );
}

function EventRow({
  event,
  checked,
  onToggle,
}: {
  event: EventDef;
  checked: boolean;
  onToggle: () => void;
}) {
  return (
    <li>
      <label className="flex cursor-pointer items-center gap-3 rounded-md px-2 py-1.5 hover:bg-surface">
        <input
          type="checkbox"
          checked={checked}
          onChange={onToggle}
          className="h-4 w-4 accent-red"
        />
        <span className="font-mono text-sm text-text">{event.name}</span>
        <span className="truncate font-mono text-xs text-faint">
          ({event.inputs.map((i) => `${i.type} ${i.name}`).join(", ")})
        </span>
      </label>
    </li>
  );
}

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex gap-2">
      <dt className="text-faint">{label}</dt>
      <dd className="truncate text-muted">{value}</dd>
    </div>
  );
}

function Badge({ tone, children }: { tone: "green" | "amber"; children: React.ReactNode }) {
  const cls =
    tone === "green"
      ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
      : "border-amber-500/30 bg-amber-500/10 text-amber-400";
  return (
    <span className={`rounded-full border px-2 py-0.5 font-mono text-[0.68rem] ${cls}`}>
      {children}
    </span>
  );
}

function shorten(addr: string): string {
  return `${addr.slice(0, 6)}…${addr.slice(-4)}`;
}

function explorerLink(c: ContractInfo): string {
  // Etherscan V2 multichain explorer deep-link is chain-specific; the address page
  // pattern is consistent enough for the common chains.
  const host: Record<number, string> = {
    1: "etherscan.io",
    42161: "arbiscan.io",
    8453: "basescan.org",
    10: "optimistic.etherscan.io",
    137: "polygonscan.com",
    56: "bscscan.com",
    43114: "snowtrace.io",
    100: "gnosisscan.io",
    534352: "scrollscan.com",
    59144: "lineascan.build",
    11155111: "sepolia.etherscan.io",
    84532: "sepolia.basescan.org",
    421614: "sepolia.arbiscan.io",
  };
  return `https://${host[c.chainId] ?? "etherscan.io"}/address/${c.address}`;
}

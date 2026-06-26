import init, { compile } from "./pkg/redstart_wasm.js";

// The default ABI the starter snippet imports as `ERC20`. Bundled so the example
// type-checks (events + the balanceOf call) entirely in the browser.
const ABIS = {
  ERC20: [
    { type: "event", name: "Transfer", inputs: [
      { name: "from", type: "address", indexed: true },
      { name: "to", type: "address", indexed: true },
      { name: "value", type: "uint256", indexed: false },
    ]},
    { type: "event", name: "Approval", inputs: [
      { name: "owner", type: "address", indexed: true },
      { name: "spender", type: "address", indexed: true },
      { name: "value", type: "uint256", indexed: false },
    ]},
    { type: "function", name: "balanceOf", stateMutability: "view",
      inputs: [{ name: "account", type: "address" }],
      outputs: [{ name: "", type: "uint256" }] },
  ],
};

const DEFAULT_SOURCE = `// Edit me — the panels on the right regenerate as you type.
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
  // Both entities are dirty-tracked and auto-saved at handler end.

  let transfer = Transfer.create(event.id, {
    from: event.params.from,
    to: event.params.to,
    value: event.params.value,
    timestamp: event.block.timestamp,
  })
}
`;

const $ = (id) => document.getElementById(id);
const source = $("source");
const status = $("status");
const abisJson = JSON.stringify(ABIS);

const OUT = {
  mappings: $("out-mappings"),
  schema: $("out-schema"),
  manifest: $("out-manifest"),
  diagnostics: $("out-diagnostics"),
};

function setStatus(text, kind) {
  status.textContent = text;
  status.className = "status" + (kind ? " " + kind : "");
}

function run() {
  let result;
  try {
    result = compile(source.value, abisJson);
  } catch (e) {
    setStatus("engine error", "err");
    OUT.diagnostics.textContent = String(e);
    return;
  }

  OUT.schema.textContent = result.schema || "";
  OUT.manifest.textContent = result.manifest || "";
  OUT.mappings.textContent = result.mappings || "";

  const diagTab = $("diag-tab");
  if (result.ok) {
    const warns = result.warnings || [];
    OUT.diagnostics.textContent = warns.length
      ? warns.map((w) => "warning: " + w).join("\n")
      : "No diagnostics. Clean build.";
    setStatus(warns.length ? warns.length + " warning(s)" : "compiled", "ok");
    diagTab.classList.remove("has-error");
  } else {
    OUT.diagnostics.textContent = (result.diagnostics || []).join("\n\n");
    setStatus(result.diagnostics.length + " error(s)", "err");
    diagTab.classList.add("has-error");
    show("diagnostics");
  }
}

function show(target) {
  for (const [name, el] of Object.entries(OUT)) {
    el.classList.toggle("hidden", name !== target);
  }
  document.querySelectorAll(".tab").forEach((t) =>
    t.classList.toggle("active", t.dataset.target === target)
  );
}

document.getElementById("tabs").addEventListener("click", (e) => {
  if (e.target.classList.contains("tab")) show(e.target.dataset.target);
});

let timer;
source.addEventListener("input", () => {
  clearTimeout(timer);
  setStatus("compiling…");
  timer = setTimeout(run, 200);
});

init().then(() => {
  source.value = DEFAULT_SOURCE;
  run();
}).catch((e) => {
  setStatus("failed to load engine", "err");
  OUT.diagnostics.textContent = String(e);
});

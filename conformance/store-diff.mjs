#!/usr/bin/env node
//
// Field-level store-diff between two subgraph GraphQL endpoints.
//
// Reads the entity types and fields from the generated schema.graphql, queries
// every entity from both endpoints (paginated, pinned to a fixed block for
// determinism), and reports any field that differs. Exit code is non-zero if
// the stores diverge — so it works as a CI gate.
//
// Usage:
//   node store-diff.mjs --a <endpointA> --b <endpointB> --schema <path> [--block N] [--first 1000]
//
// No dependencies — needs Node 18+ (global fetch).

import { readFileSync } from "node:fs";

// ---- args ----
const args = parseArgs(process.argv.slice(2));
for (const k of ["a", "b", "schema"]) {
  if (!args[k]) fail(`missing --${k}`);
}
const PAGE = Number(args.first ?? 1000);
const BLOCK = args.block ? Number(args.block) : null;

// ---- parse schema ----
const entities = parseSchema(readFileSync(args.schema, "utf8"));
if (entities.length === 0) fail("no @entity types found in schema");

// ---- diff ----
let totalDiffs = 0;
for (const entity of entities) {
  const [a, b] = await Promise.all([
    fetchAll(args.a, entity),
    fetchAll(args.b, entity),
  ]);
  totalDiffs += compare(entity, a, b);
}

if (totalDiffs === 0) {
  console.log(`\n\x1b[1;32m✓ stores are identical${BLOCK ? ` at block ${BLOCK}` : ""}\x1b[0m`);
  process.exit(0);
} else {
  console.log(`\n\x1b[1;31m✗ ${totalDiffs} difference(s) found\x1b[0m`);
  process.exit(1);
}

// ---- helpers ----

function compare(entity, a, b) {
  const ids = new Set([...a.keys(), ...b.keys()]);
  let diffs = 0;
  const onlyA = [];
  const onlyB = [];

  for (const id of ids) {
    const ra = a.get(id);
    const rb = b.get(id);
    if (!ra) { onlyB.push(id); diffs++; continue; }
    if (!rb) { onlyA.push(id); diffs++; continue; }
    for (const f of entity.fields) {
      const va = norm(ra[f.name]);
      const vb = norm(rb[f.name]);
      if (va !== vb) {
        diffs++;
        console.log(
          `\x1b[31m  ${entity.name}#${id}.${f.name}\x1b[0m  a=${va}  b=${vb}`
        );
      }
    }
  }
  if (onlyA.length) console.log(`\x1b[31m  ${entity.name}: ${onlyA.length} id(s) only in A\x1b[0m (${onlyA.slice(0, 5).join(", ")}${onlyA.length > 5 ? ", …" : ""})`);
  if (onlyB.length) console.log(`\x1b[31m  ${entity.name}: ${onlyB.length} id(s) only in B\x1b[0m (${onlyB.slice(0, 5).join(", ")}${onlyB.length > 5 ? ", …" : ""})`);

  const status = diffs === 0 ? "\x1b[32m✓\x1b[0m" : "\x1b[31m✗\x1b[0m";
  console.log(`${status} ${entity.name}: ${a.size} (A) / ${b.size} (B), ${diffs} diff(s)`);
  return diffs;
}

function norm(v) {
  if (v === null || v === undefined) return "∅";
  if (typeof v === "object") return v.id ?? JSON.stringify(v); // entity ref -> id
  return String(v);
}

// Fetch all rows of one entity from an endpoint, paginated by id.
async function fetchAll(endpoint, entity) {
  const out = new Map();
  let cursor = "";
  for (;;) {
    const rows = await page(endpoint, entity, cursor);
    if (rows.length === 0) break;
    for (const r of rows) out.set(r.id, r);
    if (rows.length < PAGE) break;
    cursor = rows[rows.length - 1].id;
  }
  return out;
}

async function page(endpoint, entity, cursor) {
  const blockArg = BLOCK != null ? `, block: { number: ${BLOCK} }` : "";
  const sel = ["id", ...entity.fields.map((f) => (f.isRef ? `${f.name} { id }` : f.name))].join(" ");
  const query = `{
    ${entity.query}(first: ${PAGE}, orderBy: id, orderDirection: asc, where: { id_gt: "${cursor}" }${blockArg}) { ${sel} }
  }`;
  const res = await fetch(endpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ query }),
  });
  const json = await res.json();
  if (json.errors) fail(`query error for ${entity.name} on ${endpoint}: ${JSON.stringify(json.errors)}`);
  return json.data[entity.query] ?? [];
}

// Minimal schema.graphql parser: entity types, their non-derived scalar/ref fields.
function parseSchema(src) {
  const typeNames = new Set();
  for (const m of src.matchAll(/type\s+(\w+)\s+@entity/g)) typeNames.add(m[1]);

  const entities = [];
  const blockRe = /type\s+(\w+)\s+@entity[^{]*\{([\s\S]*?)\}/g;
  for (const m of src.matchAll(blockRe)) {
    const name = m[1];
    const body = m[2];
    const fields = [];
    for (const line of body.split("\n")) {
      const fm = line.match(/^\s*(\w+)\s*:\s*([^\s@]+)/);
      if (!fm) continue;
      const fname = fm[1];
      if (fname === "id") continue;
      if (/@derivedFrom/.test(line)) continue;
      const rawType = fm[2];
      if (rawType.startsWith("[")) continue; // skip list fields
      const base = rawType.replace(/[\[\]!]/g, "");
      fields.push({ name: fname, isRef: typeNames.has(base) });
    }
    entities.push({ name, fields, query: pluralize(name) });
  }
  return entities;
}

// graph-node's auto-generated list query: lower-case first letter + "s".
function pluralize(name) {
  return name.charAt(0).toLowerCase() + name.slice(1) + "s";
}

function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    if (argv[i].startsWith("--")) {
      const key = argv[i].slice(2);
      const val = argv[i + 1] && !argv[i + 1].startsWith("--") ? argv[++i] : "true";
      out[key] = val;
    }
  }
  return out;
}

function fail(msg) {
  console.error(`\x1b[1;31mstore-diff: ${msg}\x1b[0m`);
  process.exit(2);
}

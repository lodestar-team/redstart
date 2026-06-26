# Redstart RFCs

Substantial changes to the Redstart language go through a Request For Comments
(RFC) process, so the design is recorded and discussed before it's built. "The
language is specified, not just implemented" is a deliberate goal — it's how a
public-good project stays legible to people who didn't write it.

## When an RFC is needed

Open an RFC for anything that is hard to undo or that changes the contract with
users:

- new syntax or keywords
- new or changed semantics (e.g. how auto-save flushes, what the checker rejects)
- changes to the generated AssemblyScript / schema / manifest output
- removing or deprecating a feature

Bug fixes, docs, refactors, new diagnostics for already-specified rules, and
additive graph-ts surface coverage do **not** need an RFC — just a pull request.

## The lifecycle

1. **Draft.** Copy [`0000-template.md`](./0000-template.md) to
   `0000-my-feature.md` (keep the `0000` until it's accepted) and open a PR.
2. **Discussion.** Iterate in the PR thread. Anyone can comment.
3. **Accepted.** A maintainer merges it; the `0000` is replaced with the next
   free number. Acceptance means "we intend to build this," not "this is built."
4. **Implemented.** When the feature lands, the RFC's status is updated and it
   links to the implementing change.
5. **Superseded / withdrawn.** If a later RFC replaces it, or it's abandoned, the
   status records that.

## Index

| RFC | Title | Status |
|----:|-------|--------|
| [0001](./0001-unified-source-of-truth.md) | Unified source of truth & the eject path | Accepted (foundational) |

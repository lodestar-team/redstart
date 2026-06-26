# What's checked for you

Redstart's value is the errors you *can't* hit. The semantic checker runs across
every module and rejects the entire class of AssemblyScript subgraph footguns
before a single block is indexed.

| Footgun in hand-written AssemblyScript | In Redstart |
|---|---|
| Forgetting `.save()` | Impossible — entities are dirty-tracked and auto-saved |
| Arithmetic on a nullable value miscompiling | Compile error — unwrap the `Option` first |
| Dereferencing a `null` load result | Compile error — `match` the `Option<Entity>` |
| Unguarded contract call aborting on revert | Compile error — `match` the `Result` |
| `==` vs `===` confusion | Not expressible — one equality, lowered correctly |
| Array prefill / length bugs | Handled by the lowering |
| Manifest event signature drifting from the ABI | Compile error — signatures are derived by reference |
| Writing to a `@derivedFrom` field | Compile error — derived fields are read-only |
| Schema/manifest/handler name drift | Impossible — all three are projections of one AST |

Other diagnostics the checker raises:

- unknown source, event, entity, or type
- missing required source settings (`abi`, `network`, `address`, `startBlock`)
- a derived field whose back-reference doesn't exist
- a required field left uninitialised at creation
- `.value` accessed without a surrounding `match`

The guiding principle: **make the broken state unrepresentable**, so the
diagnostic is a *parse* or *type* error you see in your editor, not a revert you
discover three hours into a sync.

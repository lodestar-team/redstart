//! Human explanations for diagnostic codes — the data behind `redstart explain`.
//!
//! Errors are the product (the "errors teach" principle). Every code carries not
//! just *what* tripped, but *the footgun it prevents* and *the canonical fix*.

/// A teaching explanation for one diagnostic code.
#[derive(Debug, Clone, Copy)]
pub struct Explanation {
    /// The bare code, e.g. `E062`.
    pub code: &'static str,
    /// A short title.
    pub title: &'static str,
    /// What triggers the diagnostic.
    pub summary: &'static str,
    /// The subgraph footgun this makes unrepresentable (empty if purely structural).
    pub prevents: &'static str,
    /// The canonical fix.
    pub fix: &'static str,
}

/// Look up the explanation for a code. Accepts `E062`, `e062`, or
/// `redstart::check::E062`.
#[must_use]
pub fn explain(code: &str) -> Option<&'static Explanation> {
    let bare = code.rsplit("::").next().unwrap_or(code);
    EXPLANATIONS
        .iter()
        .find(|e| e.code.eq_ignore_ascii_case(bare))
}

/// Every known explanation, in code order — for listing.
#[must_use]
pub fn all() -> &'static [Explanation] {
    EXPLANATIONS
}

static EXPLANATIONS: &[Explanation] = &[
    Explanation {
        code: "E001",
        title: "unknown generic type",
        summary: "A generic like `Id<…>` or `Option<…>` used a base type Redstart doesn't know.",
        prevents: "",
        fix: "Use a known generic: `Id<Bytes>`, `Id<String>`, `Option<T>`, or a list `[T]`.",
    },
    Explanation {
        code: "E002",
        title: "unknown type",
        summary: "A field or signature referenced a type that isn't a scalar, entity, enum, or interface.",
        prevents: "",
        fix: "Declare the entity/enum/interface, or use a built-in scalar (Bytes, BigInt, BigDecimal, String, Boolean, Int8, Timestamp).",
    },
    Explanation {
        code: "E003",
        title: "unknown interface",
        summary: "`implements` named an interface that isn't declared.",
        prevents: "",
        fix: "Declare it with `interface Name { … }`.",
    },
    Explanation {
        code: "E004",
        title: "missing interface field",
        summary: "An entity claims to implement an interface but omits one of its fields.",
        prevents: "A schema that the canonical toolchain rejects for an incomplete interface.",
        fix: "Add every field the interface declares to the implementing entity.",
    },
    Explanation {
        code: "E005",
        title: "aggregation sources unknown entity",
        summary: "An `aggregation … over X` referenced an entity `X` that doesn't exist.",
        prevents: "",
        fix: "Point the aggregation at a declared `timeseries` entity.",
    },
    Explanation {
        code: "E010",
        title: "duplicate entity",
        summary: "Two entities share a name across the project.",
        prevents: "Schema/handler ambiguity over which entity a name refers to.",
        fix: "Each entity must be declared exactly once across all modules — rename or remove one.",
    },
    Explanation {
        code: "E020",
        title: "derived field on a non-entity",
        summary: "A `derived from` field must reference an entity type.",
        prevents: "",
        fix: "Derived fields look like `swaps: [Swap] derived from pool`.",
    },
    Explanation {
        code: "E021",
        title: "derived field has no back-reference",
        summary: "The field named by `derived from` doesn't exist on the target entity.",
        prevents: "A runtime `unexpected null` when reading the (non-existent) relation.",
        fix: "Add the referenced back-reference field to the target entity.",
    },
    Explanation {
        code: "E030",
        title: "source missing a required setting",
        summary: "A `source` block is missing a required setting.",
        prevents: "A manifest that fails to deploy.",
        fix: "A source needs `abi`, `network`, `address`, and `startBlock`.",
    },
    Explanation {
        code: "E031",
        title: "template missing a required setting",
        summary: "A `template` block is missing a required setting.",
        prevents: "A manifest that fails to deploy.",
        fix: "Add the missing key (templates need `abi` and `network`).",
    },
    Explanation {
        code: "E032",
        title: "unknown ABI",
        summary: "A source or template referenced an ABI that wasn't imported.",
        prevents: "",
        fix: "Import it: `abi Name from \"./abis/Name.json\"`.",
    },
    Explanation {
        code: "E040",
        title: "unknown source",
        summary: "A handler targets a source or template that isn't declared.",
        prevents: "",
        fix: "Declare it with a `source` (or `template`) block.",
    },
    Explanation {
        code: "E041",
        title: "event not found in ABI",
        summary: "`handler on Src.Event` named an event the ABI doesn't contain.",
        prevents: "A handler that silently never fires because its signature can't keccak-match the chain.",
        fix: "Check the event name and casing against the ABI.",
    },
    Explanation {
        code: "E042",
        title: "function not found in ABI",
        summary: "A call handler bound a contract function the ABI doesn't contain.",
        prevents: "A call handler that silently never fires.",
        fix: "Check the function name against the ABI.",
    },
    Explanation {
        code: "E050",
        title: "unknown entity",
        summary: "A handler referenced an entity type that isn't declared.",
        prevents: "",
        fix: "Declare the entity, or fix the name.",
    },
    Explanation {
        code: "E051",
        title: "incomplete initializer",
        summary: "An entity was created without all of its required (non-`Option`) fields.",
        prevents: "graph-node's `missing value for non-nullable field`, or a silent null-merge.",
        fix: "Initialise every required field at creation, or make the field `Option<T>`.",
    },
    Explanation {
        code: "E052",
        title: "unknown field in initializer",
        summary: "An initializer set a field the entity doesn't declare.",
        prevents: "",
        fix: "Use only fields declared on the entity.",
    },
    Explanation {
        code: "E053",
        title: "assignment to a derived field",
        summary: "Code assigned to a `@derivedFrom` field.",
        prevents: "A silent no-op write — derived fields are virtual and read-only.",
        fix: "Write the back-reference on the child entity instead; the relation derives automatically.",
    },
    Explanation {
        code: "E054",
        title: "unknown field",
        summary: "Code accessed a field the entity doesn't declare.",
        prevents: "",
        fix: "Use a field declared on the entity.",
    },
    Explanation {
        code: "E060",
        title: "`.value` of a contract call read directly",
        summary: "Code touched `.value` of a contract call without matching its `Result`.",
        prevents: "An unhandled reverted `eth_call` aborting the handler — a non-determinism / PoI hazard.",
        fix: "`match call { Ok(v) => { … } Err(e) => { … } }` before using the value.",
    },
    Explanation {
        code: "E061",
        title: "arithmetic on an `Option`",
        summary: "Code did arithmetic on a possibly-absent `Option` value.",
        prevents: "AssemblyScript's nullable-arithmetic miscompile (silently wrong numbers).",
        fix: "Unwrap first: `match` it, or use `.unwrapOr(default)` before the arithmetic.",
    },
    Explanation {
        code: "E062",
        title: "dereference of a nullable value",
        summary: "Code accessed a field on a nullable value — a `load`/`loadInBlock`/`ipfs.cat` result.",
        prevents: "A null dereference that aborts the handler at runtime, three hours into a sync.",
        fix: "`match x { Some(v) => { … } None => { … } }` — these host calls can return nothing.",
    },
    Explanation {
        code: "E070",
        title: "non-exhaustive `match`",
        summary: "A `match` didn't cover every variant.",
        prevents: "An unhandled case slipping through at runtime.",
        fix: "Handle every variant, or add a `_ => { … }` wildcard arm.",
    },
    Explanation {
        code: "E071",
        title: "contract has no such function",
        summary: "Code called a function the bound contract's ABI doesn't declare.",
        prevents: "",
        fix: "Check the function name against the ABI (only view/pure calls are supported).",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_flexible_and_complete() {
        assert!(explain("E062").is_some());
        assert_eq!(explain("e062").unwrap().code, "E062");
        assert_eq!(explain("redstart::check::E062").unwrap().code, "E062");
        assert!(explain("E999").is_none());
        // Every entry is well-formed.
        for e in all() {
            assert!(e.code.starts_with('E'));
            assert!(!e.title.is_empty() && !e.summary.is_empty() && !e.fix.is_empty());
        }
    }
}

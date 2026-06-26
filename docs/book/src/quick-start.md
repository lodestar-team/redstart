# Quick start

## A new project

```sh
redstart new my-subgraph
cd my-subgraph
```

This scaffolds a `redstart.toml` manifest and a `src/` directory with a starter
module.

## The project layout

```
my-subgraph/
├── redstart.toml        # project name, description, output dir
└── src/
    ├── main.red         # the root module (sources, handlers)
    └── abis/            # contract ABIs you import
```

A `.red` file can declare entities, ABIs, sources, templates, handlers, free
functions, and tests. Split them across modules with `mod name;` and refer across
modules with `name::Thing` — exactly like Rust.

## The workflow

```sh
redstart check      # type-check the whole project, across every module
redstart build      # emit schema.graphql + subgraph.yaml + src/mappings.ts
redstart test       # run native tests (no WASM, no Docker)
redstart fmt        # canonical formatting (--check to verify in CI)
redstart dev        # watch loop: check → build → test on every save
redstart deploy <slug>   # build → graph codegen → graph build → graph deploy
```

## Try the examples

The repository ships four worked examples, including a faithful port of a
real-world subgraph:

```sh
redstart test examples/erc20
redstart build examples/horizon-indexer
```

Read on for a tour of the language itself, starting with
[entities](./tour/entities.md).

# 0002. Sheets calculation engine in the browser (WebAssembly)

Status: accepted. Date: 2026-10-25.

## Context

The grid evaluated formulas with a TypeScript engine that knew about 14 functions, while the
vendored gridcraft engine (ADR 0001) has 500+ functions, dynamic arrays, LET/LAMBDA and
dependency-driven recalculation. Imported `.xlsx` files therefore showed errors for most formulas.
The server stores the workbook JSON and never evaluates it.

## Decision

- `spreadsheets-engine` (the ADR 0001 adapter) gains a `Session`: workbook JSON in, calculated
  values out, incremental recalculation after edits. It is pure and native-testable. The `xlsx`
  feature (default) gates import/export so the browser build leaves out the file-format code.
- `spreadsheets-engine-wasm` (`modules/spreadsheets/engine/wasm`, also an adapter by path) wraps
  `Session` with `wasm-bindgen`. `just wasm` builds it (`[profile.wasm]`, size-optimised) into the
  generated, git-ignored `packages/formula-engine/pkg`; `lint`, `typecheck`, `test` and `dev-web`
  depend on it. The module loads only with the spreadsheet route (about 0.7 MB gzip).
- The grid keeps one engine per open workbook. `WorkbookCalc.sync` diffs the immutable workbook
  against the last synced one and patches only changed inputs; structural changes reload.
- The stored formula syntax (Turkish names, `;` separators, decimal commas) is unchanged and is
  converted to canonical syntax by the adapter, so stored files and `.xlsx` stay interoperable.

## Exception: `unsafe_code`

The `wasm-bindgen` macros expand to `unsafe` glue, which `unsafe_code = "forbid"` rejects.
`spreadsheets-engine-wasm` is the isolated crate CLAUDE.md §5.1 allows: it sets `unsafe_code = "deny"`
(no hand-written unsafe, no `allow`) and repeats the workspace clippy table, because a crate cannot
inherit the workspace lints and change one entry.

## Alternatives

- Server-side evaluation: a round trip per edit, and CPU-bound work on the API.
- Keep extending the TypeScript engine: re-implements 500 functions with different semantics
  from the engine that writes `.xlsx` files.
- Web Worker: the grid reads values synchronously while rendering. Moving the engine to a worker
  needs an asynchronous value cache; deferred until recalculation time justifies it.

## Consequences

- Behaviour now follows the gridcraft engine, e.g. `2^3^2` is `64` (left to right, as in Excel),
  where the old TypeScript engine returned `512`.
- Developers need the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` at the version of the
  `wasm-bindgen` crate (`cargo install wasm-bindgen-cli --version 0.2.129 --locked`).
- Formats other than number, percent and currency are still produced by the TypeScript formatter;
  the engine's number-format crate is the follow-up.

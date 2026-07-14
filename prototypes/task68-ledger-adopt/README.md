# task68 ledger x adopt_v1 isolated prototype

This is a standalone acceptance prototype for task #68. It deliberately has no
dependency on, and makes no edits to, `rust/src`.

It models the finalized append-only ledger reducer, the add-only `adopt_v1`
overlay, atomic `ADOPT_ORPHAN` assignment, supersede-chain guards, tombstones,
canonical terminal-state encoding, and SHA-256 hashing. The frozen task #67
fixture is replayed as `52 -> 33`, `+19`, with `7` Completed hosts.

Run from this directory:

```bash
cargo test
cargo run --release
```

The five domain property tests are in `tests/properties.rs`. Generated replay
artifacts are kept under `artifacts/`.

# task72 gap-connector (`adopt_v2`) isolated prototype

This standalone crate extends the task #68 append-only ledger and `adopt_v1`
fixture with the task #72 gap connector. It does not depend on or modify the
production `rust/src` crate.

Implemented gates:

- R1 gap exemption plus R2/R3/R4 legal-host selection;
- separately switched R2' small-to-large evaluation over the same R2/R3
  connector channels;
- `ADOPT_ORPHAN`-only extension events, no release/degrade/tombstone path;
- `judge_at <= as_of`, strict event-prefix append-only behavior, monotonic host
  preservation, and `33 = adopted_v2 + residual_v2` conservation;
- frozen 33-object decisions and the preserved `L0#37199` R1+R2 witness;
- canonical terminal-state SHA-256 and deterministic release replay.

Run from this directory:

```bash
cargo fmt --check
cargo test
cargo run --release --quiet
```

The nine property tests are in `tests/properties.rs`; generated acceptance
artifacts are under `artifacts/`.

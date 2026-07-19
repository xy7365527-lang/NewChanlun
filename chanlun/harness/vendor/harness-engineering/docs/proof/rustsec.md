# Prove a Security Claim

[`intaglio`] is a Rust symbol-interner: it stores each distinct string once and
returns a compact, stable symbol that callers can compare and later resolve. Its
two indexes represent opposite directions of the same relationship: a vector
maps each symbol to its string, while a hash map maps each string back to its
symbol. A one-line request to red-team the crate produced an advisory in
RustSec, the Rust ecosystem's vulnerability database, because the acceptance bar
required an observable impact or exploitability result.

[`intaglio`]: https://github.com/artichoke/intaglio

## Reproduce the state corruption

Codex found an unwind-safety ordering defect. The interner appended a string to
the symbol-to-string vector before inserting the corresponding string-to-symbol
entry in the hash map. A custom hasher could panic during the second mutation.
If the caller recovered with `catch_unwind` and kept using the table, the vector
would contain the new string while the map would not. A later insertion could
reuse the missing symbol number and return a symbol whose vector lookup resolved
to the earlier string.

The [public issue] records a release-mode reproducer for this symbol confusion.
It also bounds the claim: the evidence did not establish memory unsafety when
the table used `RandomState`, the ordinary default hasher builder for Rust's
`HashMap`. The demonstrated failure required a custom panic-capable hasher, a
panic during insertion, and continued use after recovery.

[public issue]: https://github.com/artichoke/intaglio/issues/359

## Carry the finding through review and release

Codex developed the report and reproducer, implemented the rollback guard, added
regression tests for all five symbol-table variants, and opened the [fix pull
request]. Human review then changed the implementation substantially. Ryan asked
for a dedicated module, a typed state transition instead of a Boolean, a
consuming terminal operation, clearer names, assertions, and focused unit tests;
Codex revised the patch and reran the crate's checks.

[fix pull request]: https://github.com/artichoke/intaglio/pull/360

Ryan approved the implementation and explicitly authorized Codex to prepare the
point release, merge the pull request, and open a RustSec report. The repository
records the merge and [v1.13.3 release] under Ryan's maintainer identity, and
the resulting [RUSTSEC-2026-0078] advisory publishes the bounded impact. This
division matters: Codex performed the investigation and the delegated release
workflow; a human maintainer supplied implementation judgment and the authority
to merge, release, and disclose.

[v1.13.3 release]: https://github.com/artichoke/intaglio/releases/tag/v1.13.3
[RUSTSEC-2026-0078]: https://rustsec.org/advisories/RUSTSEC-2026-0078.html

The same prompt produced at least one useful finding in every active Artichoke
crate Ryan tried it on. Representative results included a Prettier CI job that
did not install dependencies, `rand_mt` documentation that overemphasized an
unseeded constructor, and `raw-parts` coverage that lacked compile-fail checks
for `Send` and `Sync` auto-trait boundaries and lifetime expansion. These were
CI correctness, documentation, and test-coverage findings. They remained
maintenance work because they supplied no reproducer for security impact or
exploitability. The harness preserves the distinction between reproduced
security impact and useful maintenance findings.

Source: Ryan Lopopolo, [“A Lazy Prompt Turned Into a RustSec Advisory”].
Snapshot: [`sources/raw/hyperbola/lazy-prompt-rustsec.mdx`].

[“A Lazy Prompt Turned Into a RustSec Advisory”]:
  https://hyperbo.la/w/lazy-prompt-rustsec/
[`sources/raw/hyperbola/lazy-prompt-rustsec.mdx`]:
  ../../sources/raw/hyperbola/lazy-prompt-rustsec.mdx

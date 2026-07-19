# Source Guide

`sources/sources.json` is the canonical manifest. Raw source text lives under
`sources/raw/`; the captured public-post corpus lives under `sources/twitter/`.
The manifest records evidence handling separately from source kind: `linked`,
`snapshot`, `repository_note`, and `private_review` describe provenance
boundaries while readable indexes and owning arguments provide thesis routing.

## Adding or refreshing a source

1. Record the canonical URL, title, author or speakers, publisher, date, and
   source kind when those fields apply. Route a source from the readable index
   or owning argument; do not force every source into one thesis.
2. Record `reviewed_at` when the source is read for this corpus. A `snapshot`
   also records when its bytes were retrieved. Record an archive URL when one is
   available.
3. Preserve full text locally only when Ryan explicitly authorizes its
   redistribution or the upstream source has a license that permits it.
4. For a local snapshot, add each artifact path and content hash to its
   `snapshot` evidence, record the retrieval date and rights basis, and pin an
   upstream revision when applicable. Use `repository_note` for living,
   repository-authored observations whose versioned provenance comes from Git
   history.
5. Keep third-party license text and required notices adjacent to the snapshot.
6. Give every individually catalogued X post a `collection_reference` to the
   captured corpus and select it by status ID. The corpus row owns creation,
   capture, engagement, availability, and screenshot provenance; do not copy
   those fields into the source record. Keep the source date and canonical link
   in agreement with the selected row.
7. When rights are unclear, store a canonical link, Wayback link, factual
   metadata, bounded quotation, and synthesis—not a full copy.
8. Update the manifest and the readable source map in the same change, then run
   `uv run --script sources/scripts/validate_manifest.py` and
   `uv run --script sources/scripts/test_manifest.py` from the repository root.

The canonical OpenAI harness-engineering essay may return an empty response or a
Cloudflare challenge to non-browser clients. Run
`uv run --locked --script sources/scripts/fetch_openai.py` from the repository
root to retrieve readable article text; the helper tries the canonical URL
first, then the verified Internet Archive capture recorded in `sources.json`.

## Boundaries

- A Wayback capture preserves access; it does not grant redistribution rights.
- CC BY covers only material Ryan has authority to license. Follow `COPYING.md`
  for exclusions.
- Do not alter raw snapshots to improve prose or formatting. Put analysis in the
  owning thesis document.
- Do not claim the public X capture is a complete account export. Extend it from
  an official export when available and preserve deleted-post provenance.
- Private sources must use `private_review` evidence with an access label. The
  other evidence variants are intended for public inspection and cannot carry an
  access field; that declaration does not prove an upstream link is live.
- Do not expose private repository source, paths, identifiers, or inaccessible
  links.

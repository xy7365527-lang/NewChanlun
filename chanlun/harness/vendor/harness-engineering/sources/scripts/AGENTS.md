# Python Source Tooling

These scripts parse and validate the source manifest and Twitter corpus and
retrieve the OpenAI harness-engineering essay. Keep them small, typed, and
independently runnable with `uv`.

## Model the domain before writing the workflow

- Parse JSON and other unknown input at the boundary. Business logic receives
  domain objects, never partially checked mappings.
- Represent domain state with standard-library dataclasses declared
  `@dataclass(frozen=True, slots=True, kw_only=True)`.
- Establish semantic invariants in `__post_init__`. Boundary decoders establish
  transport shape and primitive types, construct the domain object, and attach
  the input location to construction errors.
- Use closed tagged variants when records have different required fields. Avoid
  one dataclass with a collection of booleans and nullable fields that describe
  several implicit states.
- Prefer tuples, immutable values, and derived properties. Keep one owner for
  each fact instead of synchronizing parallel representations.
- Raw `dict[str, object]` values belong only at an input boundary or in JSON
  test fixtures. Do not return them as domain results or pass them through the
  workflow.
- Once construction proves an invariant, downstream code should rely on the
  type. Do not repeat shotgun validation in every caller.
- Build structured output from models and serializers. Repeated string joining
  and interpolation of structured records indicates a missing domain model.

## Keep units intact to the boundary

- Represent durations as `datetime.timedelta`. Convert with `total_seconds()`
  only when calling an API that requires a numeric value.
- Construct an explicit `httpx.Timeout` at the HTTP client boundary. Keep
  connect and overall request policy visible as named durations.
- Accept clients or other consequential collaborators as arguments where that
  makes behavior testable. Keep network access out of model constructors.
- Retry only failures that can plausibly succeed unchanged. Keep retry budgets
  bounded and honor an actionable `Retry-After` response.

## Preserve the script boundary

- Keep executable workflows as PEP 723 scripts with `uv` shebangs.
- Prefer the standard library. When a script needs an outside package, declare
  and pin it in that script's inline metadata and commit its `uv` lockfile.
- Keep shared models in `source_manifest.py` or `twitter_corpus.py`; keep the
  executable modules as thin orchestration layers.
- Do not edit immutable source snapshots while changing tooling.

## Tests and proof

Keep parser and model tests hermetic. Cover successful construction, missing and
unknown fields, tagged-variant boundaries, cross-record relationships, overflow
and path safety, and failures that previously escaped the model.

Run the focused proof loop from the repository root:

```sh
uvx ty check sources/scripts
uvx ruff check sources/scripts
uvx ruff format --check sources/scripts
uv run --script sources/scripts/test_manifest.py
uv run --script sources/scripts/validate_manifest.py
```

Use `uvx ruff format sources/scripts` to apply formatting. Run the locked fetch
as an integration check when changing retrieval behavior:

```sh
uv run --locked --script sources/scripts/fetch_openai.py
```

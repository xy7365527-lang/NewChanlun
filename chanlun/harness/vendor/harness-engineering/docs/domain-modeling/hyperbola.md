# Harness Engineering the hyperbo.la Build

Adding an RSS feed became a restructuring of Ryan’s static blog around one
toolchain, one workspace graph, explicit package ownership, and executable
repository contracts. Each mechanism gives one domain concept an explicit owner.

## Policy has a package

Repository-local tooling under `hld/` enforces source structure and dependency
direction. Custom ESLint rules require justified suppressions and constrain
filesystem imports. Structural checks ban legacy file forms, keep content in the
canonical location, and promote repeated dependencies into the workspace
catalog.

The diagnostics are part of the agent interface. They say which invariant was
violated and guide the repair. A check that merely rejects output would preserve
the need for a human to explain the architecture.

## Domain types replace permissive primitives

`HyperUrl` represents immutable URL intent rather than passing mutable `URL`
objects or raw strings between packages. The frontmatter package accepts the
uncertain Markdown/YAML boundary, applies the blog’s constrained schema, and
returns typed values with path-aware errors.

These types reduce repeated validation elsewhere. Application code does not need
to rediscover which URL shapes or frontmatter keys are legal.

## Package topology carries intent

The blog domain owns manifests, posts, route accessors, feed rendering, and
typed template contexts. The app shell owns shared composition and assets
without importing blog rules. A Vite integration package connects domain data to
development and build rendering.

Content and its assets are colocated. Development and prerendering use the same
Vite-native SSR path, so the repository does not maintain one implementation for
local work and another for production.

## Documentation is an architecture contract

The frontend architecture document names the north star, package
responsibilities, forbidden workarounds, and evidence expected from a change. It
directs the agent toward architectural repair. The document is durable context;
structural checks and tests make its stable parts executable.

Domain ownership, path structure, actionable diagnostics, and one execution path
make the system easier to continue. Each control's carrying cost must still
match the repository's risk.

Source: Ryan Lopopolo, [“Harness Engineering the Blog Build (Again)”]. Snapshot:
[`sources/raw/hyperbola/harness-engineering-the-blog-build.mdx`].

[“Harness Engineering the Blog Build (Again)”]:
  https://hyperbo.la/w/harness-engineering-the-blog-build/
[`sources/raw/hyperbola/harness-engineering-the-blog-build.mdx`]:
  ../../sources/raw/hyperbola/harness-engineering-the-blog-build.mdx

#!/usr/bin/env bash
# Minimal reproducible entrypoint for the #1111 self-managed DevSkim scanner.
#
# Mirrors the upstream DevSkim-Action invocation exactly under the default
# inputs used by the #1111 baseline:
#   /tools/devskim analyze --source-code <src> --output-file <out> \
#     --ignore-globs <globs>
# should-scan-archives / exclude-rules / options-json / extra-options are unused,
# matching run 32305956877.
set -euo pipefail

source_dir="$1"
output_file="$2"
ignore_globs="$3"

# Prevent the shell from glob-expanding the comma-separated ignore patterns,
# exactly as the upstream entrypoint does before calling the CLI.
set -o noglob
/tools/devskim analyze \
  --source-code "$source_dir" \
  --output-file "$output_file" \
  --ignore-globs "$ignore_globs"

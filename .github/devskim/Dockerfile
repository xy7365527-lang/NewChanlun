# syntax=docker/dockerfile:1
# Self-managed, byte-reproducible DevSkim scanner runtime for #1111.
#
# Rationale: microsoft/DevSkim-Action@4b5047945a44163b94642a1cecc0d93a3f428cc6
# is commit-pinned, but its own Dockerfile pins neither axis of its runtime:
#   * base image is the mutable tag `mcr.microsoft.com/dotnet/sdk:8.0`, and
#   * `dotnet tool install Microsoft.CST.DevSkim.Cli` carries no --version.
# So "action commit pinned" does NOT imply "scanner runtime pinned". This
# Dockerfile pins both, so the runtime that produces the SARIF is reproducible.
#
# Base image digest matches the #1111 baseline run 32305956877 (run log line 52).
# The digest is a multi-arch manifest list; ubuntu-latest (amd64) resolves to the
# amd64 variant deterministically.
FROM mcr.microsoft.com/dotnet/sdk:8.0@sha256:306301580fcaa5b445180e759db59309979002d1000669cb4cf58a567d0014bc

# DevSkim CLI pinned to the exact package behind SARIF driver `1.0.90+fb2d676ce4`.
# NuGet package versions are immutable once published, so --version plus the
# official nuget.org feed fully determines the CLI content; the gate re-asserts
# the resulting driver version fail-loud.
RUN dotnet tool install \
      --tool-path /tools \
      --version 1.0.90 \
      --add-source https://api.nuget.org/v3/index.json \
      Microsoft.CST.DevSkim.Cli

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]

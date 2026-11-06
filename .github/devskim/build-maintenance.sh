#!/usr/bin/env bash
# Build only the three reviewed assemblies. Rules and packaged dependencies stay
# those of the pinned upstream CLI; locks constrain all source-build restores.
set -euo pipefail
ai=/src/ai
sk=/src/devskim
m=/maintenance
git -C "$ai" apply --check "$m/patches/engine-cache.patch"
git -C "$ai" apply "$m/patches/engine-cache.patch"
git -C "$sk" apply --check "$m/patches/devskim-performance.patch"
git -C "$sk" apply "$m/patches/devskim-performance.patch"
cp "$m/locks/ai-common.json" "$ai/AppInspector.Common/packages.lock.json"
cp "$m/locks/ai-engine.json" "$ai/AppInspector.RulesEngine/packages.lock.json"
cp "$m/locks/devskim.json" "$sk/DevSkim-DotNet/Microsoft.DevSkim/packages.lock.json"
cp "$m/locks/cli.json" "$sk/DevSkim-DotNet/Microsoft.DevSkim.CLI/packages.lock.json"
for project in "$ai/AppInspector.RulesEngine/AppInspector.RulesEngine.csproj" \
               "$sk/DevSkim-DotNet/Microsoft.DevSkim.CLI/Microsoft.DevSkim.CLI.csproj"; do
  dotnet restore "$project" --locked-mode -p:TargetFrameworks=net8.0 \
    --source https://api.nuget.org/v3/index.json
  # This is a local maintenance build, not a Microsoft-signed package.
  dotnet build "$project" --no-restore -c Release -f net8.0 \
    -p:TargetFrameworks=net8.0 -p:SignAssembly=false
 done
cp -a /upstream-tools /tools
lib=/tools/.store/microsoft.cst.devskim.cli/1.0.90/microsoft.cst.devskim.cli/1.0.90/tools/net8.0/any
cp "$ai/AppInspector.RulesEngine/bin/Release/net8.0/ApplicationInspector.RulesEngine.dll" "$lib/"
cp "$sk/DevSkim-DotNet/Microsoft.DevSkim/bin/Release/net8.0/Microsoft.DevSkim.dll" "$lib/"
cp "$sk/DevSkim-DotNet/Microsoft.DevSkim.CLI/bin/Release/net8.0/devskim.dll" "$lib/"
# Fail if packaging accidentally leaves the original assembly in the runtime.
cmp "$ai/AppInspector.RulesEngine/bin/Release/net8.0/ApplicationInspector.RulesEngine.dll" "$lib/ApplicationInspector.RulesEngine.dll"
cmp "$sk/DevSkim-DotNet/Microsoft.DevSkim/bin/Release/net8.0/Microsoft.DevSkim.dll" "$lib/Microsoft.DevSkim.dll"
cmp "$sk/DevSkim-DotNet/Microsoft.DevSkim.CLI/bin/Release/net8.0/devskim.dll" "$lib/devskim.dll"
cat "$m/maintenance.json"

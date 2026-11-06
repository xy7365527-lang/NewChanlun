# syntax=docker/dockerfile:1
# #1111 pinned upstream runtime + #1419 reviewed performance maintenance.
# The upstream SARIF identity stays 1.0.90+fb2d676ce4; OCI labels and
# /usr/local/share/devskim/maintenance.json identify our unsigned rebuild.
FROM mcr.microsoft.com/dotnet/sdk:8.0@sha256:306301580fcaa5b445180e759db59309979002d1000669cb4cf58a567d0014bc AS build

RUN dotnet tool install --tool-path /upstream-tools --version 1.0.90 \
      --add-source https://api.nuget.org/v3/index.json Microsoft.CST.DevSkim.Cli

# Full history lets upstream Nerdbank.GitVersioning reproduce its version.
# Checkout objects are fixed; no branch or tag is trusted as build input.
RUN git clone --no-checkout https://github.com/microsoft/ApplicationInspector.git /src/ai \
 && git -C /src/ai checkout --detach 8ba537cf659a50ccc557b6ccc63e13d96af2dad0 \
 && git clone --no-checkout https://github.com/microsoft/DevSkim.git /src/devskim \
 && git -C /src/devskim checkout --detach fb2d676ce475a47c0338966bebd97a47ae566572

COPY patches/ /maintenance/patches/
COPY locks/ /maintenance/locks/
COPY build-maintenance.sh maintenance.json /maintenance/
RUN bash /maintenance/build-maintenance.sh

COPY regression/ /maintenance/regression/
# The exact installed candidate, not an intermediate DLL, must match upstream.
RUN dotnet run --project /maintenance/regression/Regression.csproj -c Release \
      -- /upstream-tools /tools

FROM mcr.microsoft.com/dotnet/sdk:8.0@sha256:306301580fcaa5b445180e759db59309979002d1000669cb4cf58a567d0014bc
LABEL org.newchanlun.devskim.maintenance="1.0.90-nc-perf1" \
      org.newchanlun.devskim.source="fb2d676ce475a47c0338966bebd97a47ae566572" \
      org.newchanlun.applicationinspector.source="8ba537cf659a50ccc557b6ccc63e13d96af2dad0" \
      org.newchanlun.devskim.signing="local-unsigned-rules-engine"
COPY --from=build /tools /tools
COPY --from=build /maintenance/maintenance.json /usr/local/share/devskim/maintenance.json
COPY --from=build /src/ai/LICENSE.txt /usr/local/share/devskim/ApplicationInspector-LICENSE.txt
COPY --from=build /src/devskim/LICENSE.txt /usr/local/share/devskim/DevSkim-LICENSE.txt
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]

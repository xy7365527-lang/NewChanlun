#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
HOME1742=/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027
VENV=${VENV:-$HOME1742/venv}
mkdir -p "$HOME1742/studio/server-state"
cd "$HOME1742/studio/server-state"
export PYTHONPATH="$ROOT${PYTHONPATH:+:$PYTHONPATH}"
export LANGSMITH_TRACING=false LANGCHAIN_TRACING_V2=false
export OTEL_ENABLED=false LANGGRAPH_CLI_NO_ANALYTICS=1
unset LANGSMITH_API_KEY LANGCHAIN_API_KEY LANGSMITH_CONTROL_PLANE_API_KEY
exec "$VENV/bin/langgraph" dev --config "$ROOT/langgraph.json" \
  --host 127.0.0.1 --port 2024 --no-browser --no-reload

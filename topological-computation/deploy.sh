#!/usr/bin/env bash
# Deploy script for Topological Computation Daemon (Linux/Mac)
#
# Usage:
#   chmod +x deploy.sh
#   ./deploy.sh                  # Full deploy, 5000 steps
#   ./deploy.sh --steps 200      # Short run
#   ./deploy.sh --check-only     # Checks only, no daemon launch

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SWARM_DIR="$HOME/.swarm"
PYTHON="${PYTHON:-python3}"

echo "============================================================"
echo "  Topological Computation Daemon — Deploy (Linux/Mac)"
echo "============================================================"
echo "Script dir: $SCRIPT_DIR"

# Check Python
if ! command -v "$PYTHON" &>/dev/null; then
    echo "[FAIL] Python not found. Set PYTHON env var or install python3."
    exit 1
fi
echo "[OK] Python: $($PYTHON --version 2>&1)"

# Step 1: Install dependencies
echo ""
echo "--- Step 1: Install dependencies ---"
$PYTHON -m pip install spacy pymupdf python-dotenv --quiet 2>&1 || {
    echo "[FAIL] pip install failed"
    exit 1
}
echo "[OK] Required packages installed"

# Optional
$PYTHON -m pip install web3 ipfshttpclient --quiet 2>/dev/null && \
    echo "[OK] Optional packages installed" || \
    echo "[OK] Optional packages skipped (not critical)"

# Step 2: Create directories
echo ""
echo "--- Step 2: Create directories ---"
mkdir -p "$SWARM_DIR"/{blocks,relations,index,output}
echo "[OK] $SWARM_DIR/ created"

# Step 3: Write .env
echo ""
echo "--- Step 3: Write .env ---"
ENV_FILE="$SCRIPT_DIR/.env"
if [ ! -f "$ENV_FILE" ]; then
    cat > "$ENV_FILE" << 'ENVEOF'
SEMANTIC_SCHOLAR_API=https://api.semanticscholar.org/graph/v1
ARXIV_API=https://export.arxiv.org/api
UNPAYWALL_EMAIL=hanjunyu2003@proton.me
BRAVE_ANSWER_API_KEY=
BRAVE_SEARCH_API_KEY=
IPFS_API=http://localhost:5001
ENVEOF
    echo "[OK] .env created"
else
    echo "[OK] .env already exists"
fi

# Step 4: Run tests
echo ""
echo "--- Step 4: Run tests ---"
cd "$SCRIPT_DIR"
$PYTHON -m pytest test_engine.py -v || {
    echo "[FAIL] Tests failed"
    exit 1
}
echo "[OK] All tests passed"

# Step 5: Generate seed + Launch daemon via deploy.py
echo ""
echo "--- Step 5: Launch via deploy.py ---"
cd "$SCRIPT_DIR"
$PYTHON deploy.py "$@"

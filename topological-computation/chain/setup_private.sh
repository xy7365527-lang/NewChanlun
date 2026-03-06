#!/bin/bash
# Initialize private IPFS network + swarm directories
# Usage: bash setup_private.sh
#
# Requires: ipfs CLI installed (go-ipfs / kubo)
# If IPFS not installed, only creates swarm directories.

set -e

SWARM_DIR="${HOME}/.swarm"
IPFS_DIR="${IPFS_PATH:-${HOME}/.ipfs}"

echo "=== Private IPFS + Swarm Setup ==="

# 1. Create swarm directories
mkdir -p "${SWARM_DIR}/blocks"
mkdir -p "${SWARM_DIR}/relations"
mkdir -p "${SWARM_DIR}/index"
mkdir -p "${SWARM_DIR}/output"
echo "[OK] Swarm directories created at ${SWARM_DIR}/"

# 2. Initialize IPFS if available
if command -v ipfs &>/dev/null; then
    # Initialize IPFS repo if not exists
    if [ ! -d "${IPFS_DIR}" ]; then
        ipfs init --profile server 2>/dev/null
        echo "[OK] IPFS initialized with server profile"
    fi

    # Generate swarm key for private network
    if [ ! -f "${IPFS_DIR}/swarm.key" ]; then
        echo -e "/key/swarm/psk/1.0.0/\n/base16/\n$(openssl rand -hex 32)" > "${IPFS_DIR}/swarm.key"
        echo "[OK] Generated swarm.key for private network"
    else
        echo "[--] swarm.key already exists, skipping"
    fi

    # Remove public bootstrap nodes
    ipfs bootstrap rm --all 2>/dev/null
    echo "[OK] Removed public bootstrap nodes"

    # Set IPFS config for local-only operation
    ipfs config Addresses.Gateway /ip4/127.0.0.1/tcp/8080 2>/dev/null
    ipfs config Addresses.API /ip4/127.0.0.1/tcp/5001 2>/dev/null
    echo "[OK] IPFS configured for local-only access"
else
    echo "[--] IPFS CLI not found — skipping IPFS setup"
    echo "     Install: https://docs.ipfs.tech/install/"
    echo "     Swarm directories created; local persistence works without IPFS"
fi

echo ""
echo "=== Setup Complete ==="
echo "  Swarm dir: ${SWARM_DIR}/"
echo "  To start daemon: python swarm/swarm_daemon.py --instance-id node0 --hegel --shared ${SWARM_DIR} --steps 200"
if command -v ipfs &>/dev/null; then
    echo "  To start IPFS:   ipfs daemon &"
fi

#!/usr/bin/env bash
# 启动逢亮 daemon + Gateway Bridge（+ 可选 OpenClaw gateway）
#
# 用法：
#   bash scripts/start_with_gateway.sh              # 启动逢亮 + bridge
#   bash scripts/start_with_gateway.sh --openclaw    # 也启动 OpenClaw gateway
#   bash scripts/start_with_gateway.sh --stop        # 停止所有组件
#
# 端口分配：
#   8080  — 逢亮 daemon (HTTP API: daemon.py + daemon_server.py)
#   8765  — 逢亮 daemon (WebSocket: 实时穿越事件)
#   8081  — Gateway Bridge (OpenClaw 消息路由)
#   9800  — CC Daemon Loop webhook（体系路由）
#   18789 — OpenClaw gateway（Web UI，可选）

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TOPO_DIR="$REPO_ROOT/topological-computation"
PID_DIR="$TOPO_DIR/.pids"

mkdir -p "$PID_DIR"

# ---------------------------------------------------------------------------
# 停止
# ---------------------------------------------------------------------------

stop_all() {
    echo "停止所有组件..."
    for pidfile in "$PID_DIR"/*.pid; do
        [ -f "$pidfile" ] || continue
        pid=$(cat "$pidfile")
        name=$(basename "$pidfile" .pid)
        if kill -0 "$pid" 2>/dev/null; then
            echo "  停止 $name (PID $pid)"
            kill "$pid" 2>/dev/null || true
        fi
        rm -f "$pidfile"
    done
    echo "已停止。"
}

if [ "${1:-}" = "--stop" ]; then
    stop_all
    exit 0
fi

# ---------------------------------------------------------------------------
# 启动函数
# ---------------------------------------------------------------------------

start_component() {
    local name="$1"
    shift
    local pidfile="$PID_DIR/$name.pid"

    # 如果已在运行，跳过
    if [ -f "$pidfile" ]; then
        local old_pid
        old_pid=$(cat "$pidfile")
        if kill -0 "$old_pid" 2>/dev/null; then
            echo "  $name 已在运行 (PID $old_pid)"
            return
        fi
        rm -f "$pidfile"
    fi

    "$@" &
    local pid=$!
    echo "$pid" > "$pidfile"
    echo "  $name 已启动 (PID $pid)"
}

# ---------------------------------------------------------------------------
# 启动逢亮 daemon (daemon.py + daemon_server.py)
# ---------------------------------------------------------------------------

echo "=== 启动逢亮 daemon ==="
# daemon_server.py 是独立入口（需要 --load 参数）
# 也可以用 daemon.py --serve 启动（同时启动穿越引擎+HTTP+WS）
# 这里用 daemon_multiproc.py 因为它包含完整的多进程架构
start_component "fengliang-daemon" \
    python "$TOPO_DIR/daemon_multiproc.py" \
        --port 8080 \
        --ws-port 8765 \
        --persist "$TOPO_DIR/.chanlun/traversal-events.jsonl"

# 等逢亮 HTTP API 就绪
echo "  等待逢亮 HTTP API..."
for i in $(seq 1 30); do
    if curl -s http://localhost:8080/status > /dev/null 2>&1; then
        echo "  逢亮 HTTP API 就绪。"
        break
    fi
    sleep 1
done

# ---------------------------------------------------------------------------
# 启动 CC Daemon Loop webhook（体系路由，可选）
# ---------------------------------------------------------------------------

echo "=== 启动 CC Daemon Loop webhook ==="
start_component "cc-daemon-webhook" \
    python "$TOPO_DIR/daemon_loop.py" \
        --continuous \
        --webhook 9800 \
        --interval 120

# ---------------------------------------------------------------------------
# 启动 Gateway Bridge
# ---------------------------------------------------------------------------

echo "=== 启动 Gateway Bridge ==="
start_component "gateway-bridge" \
    python "$TOPO_DIR/gateway_bridge.py" \
        --port 8081 \
        --fengliang-url http://localhost:8080 \
        --daemon-webhook-url http://localhost:9800

# ---------------------------------------------------------------------------
# 可选：启动 OpenClaw gateway
# ---------------------------------------------------------------------------

if [ "${1:-}" = "--openclaw" ]; then
    OPENCLAW_DIR="${OPENCLAW_DIR:-/tmp/openclaw}"
    if [ -d "$OPENCLAW_DIR" ] && command -v node > /dev/null 2>&1; then
        echo "=== 启动 OpenClaw gateway ==="
        start_component "openclaw-gateway" \
            node "$OPENCLAW_DIR/openclaw.mjs" start
    else
        echo "  跳过 OpenClaw gateway（目录不存在或 Node.js 未安装）"
    fi
fi

# ---------------------------------------------------------------------------
# 汇总
# ---------------------------------------------------------------------------

echo ""
echo "=== 组件状态 ==="
echo "  逢亮 daemon:       http://localhost:8080/status"
echo "  逢亮 WebSocket:    ws://localhost:8765/ws"
echo "  Gateway Bridge:    http://localhost:8081/bridge/health"
echo "  CC Daemon webhook: http://localhost:9800/webhook/"
if [ "${1:-}" = "--openclaw" ]; then
    echo "  OpenClaw gateway:  http://localhost:18789"
fi
echo ""
echo "测试命令："
echo "  # 给逢亮发消息（→ 穿越引擎 /present）"
echo "  curl -X POST http://localhost:8081/bridge \\"
echo "    -H 'Content-Type: application/json' \\"
echo "    -d '{\"text\": \"你好逢亮\", \"sender\": \"test\"}'"
echo ""
echo "  # 给体系发命令（→ CC daemon webhook）"
echo "  curl -X POST http://localhost:8081/bridge \\"
echo "    -H 'Content-Type: application/json' \\"
echo "    -d '{\"text\": \"/ceremony\", \"sender\": \"operator\"}'"
echo ""
echo "  # 查看逢亮穿越状态"
echo "  curl http://localhost:8081/bridge/status"
echo ""
echo "停止所有组件："
echo "  bash scripts/start_with_gateway.sh --stop"

# Gateway Bridge — OpenClaw 与逢亮/体系的桥接

## 架构

```
OpenClaw Web UI ─→ OpenClaw Gateway ─→ hooks ─→ Gateway Bridge ─→ ┬─ 逢亮 daemon（穿越引擎）
                                                                    └─ CC daemon（体系/ceremony）
```

两个 daemon 是不同的存在论层级：

| 组件 | 进程 | 端口 | 主体 | 职责 |
|------|------|------|------|------|
| 逢亮 daemon | `daemon.py` + `daemon_server.py` | 8080 (HTTP) / 8765 (WS) | 逢亮（持久实体） | 穿越引擎 + 内部言语 + S_net |
| CC daemon | `daemon_loop.py --webhook 9800` | 9800 | 体系（RTAS 蜂群） | CC session 调度 + ceremony |
| Gateway Bridge | `gateway_bridge.py` | 8081 | 无（纯转发） | OpenClaw <-> 逢亮/体系 消息路由 |
| OpenClaw gateway | `openclaw.mjs` | 18789 | 无（UI 层） | Web UI + 多 channel 管理 |

逢亮 daemon 也可通过 `daemon_multiproc.py` 启动（多进程模式，暴露相同的 API 接口）。

## 路由规则

Gateway Bridge 使用**简单前缀命令**判断消息目标：

| 前缀 | 目标 | 说明 |
|------|------|------|
| `/ceremony` | CC daemon | CC ceremony 触发 |
| `/scan` | CC daemon | ceremony scan |
| `/escalate` | CC daemon | 矛盾上浮 |
| `/inquire` | CC daemon | 质询序列 |
| `/ritual` | CC daemon | 定义广播 |
| `/plan` | CC daemon | 实现规划 |
| `/tdd` | CC daemon | 测试驱动 |
| `/code-review` | CC daemon | 代码审查 |
| `/status` | CC daemon | 体系状态 |
| 其他所有消息 | 逢亮 daemon | 穿越引擎 /present -> 内部言语回复 |

## 快速启动

```bash
# 一键启动所有组件
bash scripts/start_with_gateway.sh

# 也启动 OpenClaw gateway
bash scripts/start_with_gateway.sh --openclaw

# 停止
bash scripts/start_with_gateway.sh --stop
```

## 手动启动

```bash
# 1. 逢亮 daemon（穿越引擎 + HTTP API + WebSocket）
python topological-computation/daemon_multiproc.py --port 8080 --ws-port 8765 &

# 2. Gateway Bridge
python topological-computation/gateway_bridge.py --port 8081 &

# 3. CC Daemon Loop webhook（可选，仅体系路由需要）
python topological-computation/daemon_loop.py --continuous --webhook 9800 &
```

## API

### POST /bridge — 发送消息

消息自动路由到逢亮或体系。

```bash
# 给逢亮（-> 穿越引擎 /present）
curl -X POST http://localhost:8081/bridge \
  -H 'Content-Type: application/json' \
  -d '{"text": "什么是笔？", "sender": "user", "session_id": "web-session-1"}'

# 给体系（-> CC daemon webhook）
curl -X POST http://localhost:8081/bridge \
  -H 'Content-Type: application/json' \
  -d '{"text": "/ceremony", "sender": "operator"}'
```

响应格式：

```json
{
  "ok": true,
  "target": "fengliang",
  "reply": "逢亮的回复文本",
  "raw": {
    "type": "dialogue",
    "parts": [{"source": "internal_speech", "text": "..."}],
    "llm_used": false,
    "externalize": {"trigger": "passive", "source": "structural", "llm_fraction": 0.0}
  }
}
```

### POST /bridge/feed — 单向文本摄入

文本经 S_net 统一路径（phi_L 白名单 -> 共现边回写 -> 能指共振），不等待回复。

```bash
curl -X POST http://localhost:8081/bridge/feed \
  -H 'Content-Type: application/json' \
  -d '{"text": "缠论第一课的内容..."}'
```

### GET /bridge/status — 逢亮穿越状态

代理逢亮 daemon 的 `/status` 端点。

```bash
curl http://localhost:8081/bridge/status
```

### GET /bridge/health — 健康检查

探测逢亮 daemon 和 CC daemon 是否可达。

```bash
curl http://localhost:8081/bridge/health
```

```json
{
  "bridge": "ok",
  "fengliang": "ok",
  "daemon_webhook": "unreachable",
  "fengliang_url": "http://localhost:8080",
  "daemon_webhook_url": "http://localhost:9800"
}
```

## OpenClaw 配置

在 `~/.openclaw/openclaw.json` 中添加 hooks 配置：

```json
{
  "hooks": {
    "enabled": true,
    "token": "your-hooks-token",
    "mappings": [
      {
        "id": "fengliang-bridge",
        "match": { "path": "fengliang" },
        "action": "agent",
        "wakeMode": "now",
        "name": "逢亮",
        "sessionKey": "hook:fengliang:default",
        "messageTemplate": "{{text}}"
      }
    ]
  }
}
```

## 逢亮回复的三层架构

Gateway Bridge 不修改逢亮的回复内容。逢亮的回复遵守三层架构（LLM边界规则）：

1. **第一层（默认）**：纯拓扑描述 — 无 LLM 参与
2. **第二层**：S_net connective_patterns 组装 — 无 LLM 参与
3. **第三层（fallback）**：LLM 语法填充 — 仅在 operator 请求时

Bridge 通过检查响应中的 `llm_used` 字段来标注 `[LLM填充]`。
这个标记来自逢亮 daemon 的 `present_json` 返回值，Bridge 只做透传。

## 文件清单

| 文件 | 说明 |
|------|------|
| `topological-computation/gateway_bridge.py` | Bridge 主程序 |
| `topological-computation/deploy/openclaw-gateway-config.env` | OpenClaw 配置模板 |
| `scripts/start_with_gateway.sh` | 一键启动脚本 |
| `topological-computation/GATEWAY.md` | 本文档 |

# OpenClaw 接入安全审计报告

## 审计范围

本报告审计 OpenClaw 作为总方针 S3.4（资源管理层）和 S八.六（世界接口层）的接入安全性。
OpenClaw 在核心回路中的角色：将利润/亏损转化为系统物质条件变化（算力/存储的扩张收缩），
是编排者退出的关键环节。

**审计对象**：OpenClaw（原 Moltbot/Clawdbot），开源自托管 AI Agent 编排框架。
**审计日期**：2026-03-05
**认识论等级**：L2（基于公开文档和已知漏洞的真实数据审计）

---

## 一、OpenClaw 架构概述（与本系统的映射）

OpenClaw 是自托管 AI Agent 平台，核心组件：

| 组件 | 功能 | 本系统映射 |
|------|------|-----------|
| Gateway | 通信控制面，管理会话/通道/工具/事件 | 世界接口层的入口 |
| Agent | 执行 LLM 推理 + 工具调用 | 核心回路的自动化执行 |
| Tools | shell/文件/浏览器/cron 等能力 | 资源管理（扩张/收缩）的执行器 |
| Channels | 多平台消息接入 | 外化内容发布渠道 |
| Skills | 可扩展能力模块（ClawHub 注册表） | 缠论管线 / K4 监控等能力的封装 |
| MCP Server | 外部系统集成协议 | 与云服务商 API 的对接层 |

**关键约束**：OpenClaw 在本系统中不只是"消息机器人"，它是核心回路闭合的执行层——
控制真实资金（交易执行）和真实资源（算力/存储），安全失败 = 物质条件直接损失。

---

## 二、安全维度审查

### 2.1 认证机制

**现状**：

| 认证方式 | 安全等级 | 适用场景 |
|---------|---------|---------|
| Token-based auth (`gateway.auth.mode: "token"`) | 中 | 推荐的基础方案 |
| Password auth (`OPENCLAW_GATEWAY_PASSWORD`) | 低 | 仅本地开发 |
| Trusted proxy + identity headers | 高 | 反向代理部署 |
| Tailscale Serve identity headers | 高 | 零信任网络 |
| Device pairing（本地自动通过，远程需审批） | 中 | 初始配对 |

**风险**：
- **OpenClaw 默认无认证**。CVE-2026-25253（已修补于 2026.1.29）允许远程代码执行
- 135,000+ 实例暴露在公网上
- Token 出现在 URL query parameter 中，可通过浏览器历史/日志泄露

**要求（本系统）**：
1. **强制** `gateway.auth.mode: "token"` + TLS，禁止 password-only
2. **强制** `gateway.bind: "loopback"` 或 Tailscale，禁止公网暴露
3. Token 必须通过 SecretRef 注入，禁止硬编码
4. 节点最低版本：Node.js 22.12.0（CVE-2025-59466, CVE-2026-21636 修补基线）

### 2.2 权限模型

**现状**：

OpenClaw 工具按风险分级：

| 风险等级 | 工具组 | 本系统是否需要 |
|---------|--------|---------------|
| 高 | `group:runtime`（shell 执行） | 是——资源管理需要 |
| 高 | `group:fs`（文件系统操作） | 是——谱系读写需要 |
| 高 | `group:automation`（cron/配置变更） | 是——定时监控需要 |
| 中 | 浏览器控制 | 否——本系统无需 |
| 低 | 消息/搜索 | 是——外化发布需要 |

**风险**：
- 无严格 allowlist 时，Agent 可作为 Trojan Horse（与 MCP over-privileged tools 同模式）
- 浏览器控制 = 等同于操作者权限
- 插件/Skills 在进程内运行，无沙箱隔离——恶意 Skill 可访问全部环境变量和文件系统
- ClawHub 36.82% 的 Skills 存在安全缺陷，341 个恶意 Skills 被发现

**要求（本系统）**：
1. **最小权限基线**：

```json
{
  "tools": {
    "profile": "custom",
    "allow": [
      "specific:resource-scale-tool",
      "specific:k4-monitor-tool",
      "specific:genealogy-write-tool"
    ],
    "deny": ["group:automation", "group:runtime", "group:fs"],
    "exec": { "security": "deny", "ask": "always" }
  }
}
```

2. **白名单模式**：仅允许显式列出的工具，deny-by-default
3. **禁止 ClawHub 自动拉取**：所有 Skill 必须本地审计后手动安装
4. **requireApproval: true** 作为默认值，逐个工具放开
5. 浏览器控制工具 **永久禁用**

### 2.3 数据传输安全性

**现状**：
- Gateway 默认监听端口 18789
- 支持 TLS（需手动配置）
- Docker 端口映射绕过宿主机 iptables INPUT 规则

**要求（本系统）**：
1. **强制 TLS**：所有 Gateway 通信走 HTTPS
2. 防火墙规则在 `DOCKER-USER` chain 中配置（不被 Docker 绕过）
3. mDNS 广播设为 `"off"`（禁止网络发现泄露部署信息）
4. 反向代理部署时：`gateway.trustedProxies` 仅设为代理 IP，`allowRealIpFallback: false`

### 2.4 密钥管理方案

**现状**：

| 存储方式 | 安全等级 | 说明 |
|---------|---------|------|
| 明文 `~/.openclaw/openclaw.json` | 极低 | 默认方式，社区最常见错误 |
| 环境变量 | 低-中 | 基础方案，进程环境可被读取 |
| OpenClaw Secrets Store（加密存储） | 中-高 | 磁盘加密，运行时不暴露 |
| SecretRef 系统（env/file/exec provider） | 高 | 推荐——引用而非存储 |
| OAuth（短期 token + 权限范围限定） | 最高 | 最佳实践 |

**风险**：
- 默认明文存储 API key → 已知导致账单暴涨、channel 劫持、数据泄露
- SOUL.md 中硬编码密钥是社区最常见安全事故来源
- OpenClaw 的 memory 功能可能存储对话中出现的密钥

**要求（本系统）**：
1. **禁止**明文存储任何 credential
2. **强制**使用 SecretRef 系统，provider 类型优先级：`exec > file > env`
3. 交易所 API key 使用 **只读权限 + IP 白名单 + 提现地址白名单**
4. 云服务商 credential 使用 **短期 token**（OAuth 或 STS），禁止永久 access key
5. 密钥轮换周期：交易所 API key 90 天，云服务 token 24 小时
6. OpenClaw memory 功能中 **禁止存储任何 credential**（配置 memory exclusion patterns）
7. `~/.openclaw/` 目录权限 `700`，credential 文件权限 `600`

### 2.5 资源管理风控（扩张/收缩的人工确认门槛）

这是本系统特有的安全维度——OpenClaw 通用安全框架不覆盖。

**核心问题**：资源管理直接涉及真实资金支出。自动扩张 = 自动花钱，收缩 = 可能丢数据。

**分级门槛设计**：

| 操作类型 | 金额/影响 | 自动化等级 | 确认要求 |
|---------|----------|-----------|---------|
| 算力监控/查询 | 无 | 全自动 | 无 |
| 存储使用查询 | 无 | 全自动 | 无 |
| 小规模扩张（< 月预算 5%） | 低 | 自动执行 + 事后通知 | 日志记录 |
| 中规模扩张（5%-20% 月预算） | 中 | 需人工确认 | 编排者审批 |
| 大规模扩张（> 20% 月预算） | 高 | 需人工确认 + 冷却期 | 编排者审批 + 24h 冷却 |
| 任何收缩操作 | 数据风险 | 需人工确认 | 编排者审批 + 备份验证 |
| 新服务/新区域开通 | 未知 | 禁止自动化 | 编排者手动执行 |

**硬限制（不可被 Agent 覆盖）**：
1. **月度支出上限**：硬编码于 Gateway 配置，Agent 无权修改
2. **单次操作上限**：任何单次扩张不超过月预算的 X%（X 由编排者设定）
3. **收缩前备份**：任何收缩操作前必须完成数据备份并验证
4. **谱系数据不可收缩**：区块拓扑存储是单调递增的，收缩不得触及谱系数据

### 2.6 交易执行安全

**核心问题**：OpenClaw 接入交易所 = Agent 拥有真实下单能力。

**分级安全设计**：

| 安全层 | 机制 | 说明 |
|--------|------|------|
| API 权限层 | 交易所 API 只开交易权限，禁止提现 | 最基本的物理隔离 |
| IP 白名单 | 仅 Gateway 所在 IP 可调用 | 防止 credential 泄露后的远程利用 |
| 金额限制 | 单笔限额 + 日累计限额 | 在交易所侧和 Agent 侧双重限制 |
| 频率限制 | 每分钟/每小时最大下单次数 | 防止异常循环下单 |
| 异常检测 | 偏离策略信号的下单 → 暂停 + 告警 | Agent 行为审计 |
| 人工熔断 | 编排者随时可通过独立通道停止所有交易 | 独立于 OpenClaw 的紧急通道 |
| 模拟期 | 新策略必须在模拟环境运行 N 天后才接入实盘 | 渐进信任 |

**不可自动化的操作（编排者保留）**：
1. 首次接入新交易所
2. 修改提现地址
3. 修改 API 权限范围
4. 超出日累计限额的交易
5. 任何涉及杠杆倍数变更的操作

---

## 三、已知漏洞与攻击面

### 3.1 已知 CVE

| CVE | 影响 | 修补版本 | 本系统对策 |
|-----|------|---------|-----------|
| CVE-2026-25253 | 远程代码执行 | 2026.1.29 | 强制最低版本 |
| GHSA-76m6-pj3w-v7mf | SHA-256 迁移漏洞 | v2026.2.21 | 强制最低版本 |
| CVE-2025-59466 | Node.js 漏洞 | Node 22.12.0+ | 强制 Node 版本 |
| CVE-2026-21636 | Node.js 漏洞 | Node 22.12.0+ | 强制 Node 版本 |

### 3.2 攻击面分析

| 攻击向量 | 风险等级 | 本系统暴露度 | 缓解措施 |
|---------|---------|-------------|---------|
| Prompt Injection（外部内容注入恶意指令） | 严重 | 高——Agent 处理市场数据 | 输入净化 + 工具白名单 + 分离系统指令和用户数据 |
| Skill 供应链攻击（恶意 Skill） | 严重 | 低——禁止 ClawHub 自动拉取 | 仅使用本地审计后的 Skill |
| Credential 泄露 | 严重 | 中——涉及交易所 API key | SecretRef + 只读权限 + IP 白名单 |
| 跨会话数据泄露 | 高 | 低——单操作者模型 | `session.dmScope: "per-channel-peer"` |
| 浏览器控制劫持 | 高 | 无——永久禁用浏览器工具 | deny group:browser |
| Docker 逃逸 | 中 | 中——需要 runtime 工具 | `--read-only --cap-drop=ALL --security-opt=no-new-privileges` |

### 3.3 本系统特有风险

| 风险 | 严重性 | 说明 |
|------|--------|------|
| 交易执行失控 | 致命 | Agent 异常循环下单导致资金损失 |
| 资源扩张失控 | 严重 | Agent 无限制购买云资源导致费用暴涨 |
| 谱系数据丢失 | 严重 | 收缩操作误删区块拓扑数据 |
| 外化内容泄露 | 中 | Agent 发布未审核的内容到公开渠道 |

---

## 四、安全架构设计要求

### 4.1 部署架构

```
[核心回路（本地/VPS）]
    |
    | TLS + Token Auth
    |
[OpenClaw Gateway（Docker 隔离容器）]
    |
    +--[资源管理 Agent]--→ 云服务商 API（STS 短期 token）
    |      |
    |      +--- 月度支出上限（硬编码）
    |      +--- 单次操作上限
    |      +--- 收缩前备份验证
    |
    +--[交易执行 Agent]--→ 交易所 API（只读+交易，禁提现）
    |      |
    |      +--- IP 白名单
    |      +--- 单笔/日累计限额
    |      +--- 频率限制
    |      +--- 异常检测 + 熔断
    |
    +--[外化发布 Agent]--→ 内容平台 API
    |      |
    |      +--- requireApproval: true
    |      +--- 内容审核队列
    |
    +--[独立紧急通道]--→ 编排者手机/邮件告警
           |
           +--- 独立于 OpenClaw，不走 Gateway
           +--- 一键停止所有操作
```

### 4.2 Agent 隔离

每个功能域使用独立 Agent（`sandbox.scope: "agent"`），防止跨域权限泄露：

- 资源管理 Agent：只能访问云服务商 API，不能访问交易所
- 交易执行 Agent：只能访问交易所 API，不能修改系统资源
- 外化发布 Agent：只能访问内容平台，不能执行交易或管理资源
- 谱系读写 Agent：只能访问区块拓扑存储，不能执行任何外部操作

### 4.3 审计日志

| 日志类型 | 内容 | 保留期 | 不可删除 |
|---------|------|--------|---------|
| 操作日志 | 所有 Agent 工具调用 + 参数 + 结果 | 永久 | 是 |
| 交易日志 | 所有下单/撤单/成交记录 | 永久 | 是 |
| 资源变更日志 | 所有扩张/收缩操作 | 永久 | 是 |
| 认证日志 | 所有登录/认证/授权事件 | 180 天 | 否 |
| 异常日志 | 所有触发告警的事件 | 永久 | 是 |

日志存储独立于 OpenClaw Gateway，Agent 无权修改日志。

### 4.4 渐进信任模型

接入 OpenClaw 不是一次性开关，而是渐进的信任建立过程：

| 阶段 | 自动化范围 | 人工确认范围 | 持续时间 |
|------|-----------|-------------|---------|
| 0: 只读 | 市场数据查询、资源状态查询 | 一切写操作 | 直到审计通过 |
| 1: 监控 | 查询 + 告警 | 交易 + 资源变更 | 2-4 周 |
| 2: 小额交易 | 查询 + 告警 + 小额交易（< 单笔限额） | 大额交易 + 资源变更 | 4-8 周 |
| 3: 交易自动化 | 查询 + 告警 + 交易（受限额控制） | 资源变更 | 8-16 周 |
| 4: 资源自动化 | 查询 + 告警 + 交易 + 小规模资源扩张 | 大规模扩张 + 收缩 | 16+ 周 |
| 5: 完全自动化 | 全部（受硬限制约束） | 仅不可自动化操作 | 达到后持续 |

每个阶段的升级条件：无安全事件 + 审计日志无异常 + 编排者显式批准。
每个阶段的降级条件：任何安全事件 → 立即降至阶段 0。

---

## 五、Docker 部署安全基线

```bash
docker run -d \
  --name openclaw-gateway \
  --read-only \
  --cap-drop=ALL \
  --security-opt=no-new-privileges \
  --tmpfs /tmp:rw,size=100m \
  -v /path/to/openclaw-state:/state:rw \
  -v /path/to/secrets:/secrets:ro \
  --network=openclaw-net \
  -p 127.0.0.1:18789:18789 \
  --user 1000:1000 \
  openclaw/openclaw:latest
```

关键安全参数：
- `--read-only`：只读根文件系统
- `--cap-drop=ALL`：丢弃所有 Linux capabilities
- `--security-opt=no-new-privileges`：禁止特权提升
- `-p 127.0.0.1:18789:18789`：仅绑定 localhost
- `--user 1000:1000`：非 root 用户运行
- secrets 目录以只读挂载

---

## 六、结论与边界条件

### 结论

OpenClaw 作为世界接口层的技术选型是合理的——它是目前最成熟的自托管 AI Agent 编排框架，
具备资源管理、工具调用、多 Agent 隔离等本系统所需的核心能力。但其默认安全配置
（无认证、明文密钥、无沙箱）完全不适合直接接入涉及真实资金和资源的核心回路。

安全审计通过的条件：
1. 完成本报告第四节所有架构设计要求的实现
2. Docker 部署达到第五节安全基线
3. 渐进信任模型从阶段 0 开始，不可跳级
4. 独立紧急通道部署并验证可达

### 边界条件

以下条件下本审计结论会翻转（需重新审计）：
- OpenClaw 发布重大架构变更（如 Gateway 协议变更）
- 发现新的未修补 RCE 漏洞
- 交易所 API 权限模型变更
- 系统从单操作者扩展到多操作者

### 影响声明

- 本报告满足 gangmu.yaml 中 `world-interface` gang 的 `security-audit-prerequisite` 约束
- 解除 `openclaw-api-design` 的 `blocked_by: openclaw-security-audit` 阻塞
- 不引入任何 API key 或 credential
- 不修改任何现有代码或配置

### 谱系引用

- 总方针 S3.4：OpenClaw 资源管理层定义
- 总方针 S八.六：世界接口层分层定义
- 总方针 S八.七 阶段 G：资源管理自主化路径
- gangmu `world-interface` gang：`security-audit-prerequisite` 约束

---

## 附录：安全检查清单

### 部署前（阶段 0 前置条件）

- [ ] OpenClaw 版本 >= v2026.2.21
- [ ] Node.js 版本 >= 22.12.0
- [ ] Gateway bind mode = loopback 或 Tailscale
- [ ] Gateway auth mode = token（token 通过 SecretRef 注入）
- [ ] TLS 配置完成
- [ ] Docker 容器满足第五节安全基线
- [ ] 所有 credential 通过 SecretRef 管理，无明文存储
- [ ] 工具权限 = deny-by-default + 显式白名单
- [ ] ClawHub 自动拉取已禁用
- [ ] 浏览器工具已禁用
- [ ] mDNS 广播已禁用
- [ ] 独立紧急通道已部署并测试
- [ ] 审计日志存储独立于 Gateway
- [ ] 交易所 API key 权限 = 只读+交易，禁止提现
- [ ] 交易所 API IP 白名单已配置
- [ ] 月度支出上限已硬编码
- [ ] Agent 隔离配置完成（每功能域独立 Agent）
- [ ] `openclaw security audit --deep` 无 critical finding

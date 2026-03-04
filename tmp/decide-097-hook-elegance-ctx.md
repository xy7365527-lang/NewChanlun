# Gemini Decide 上下文：097号——hook 在三层存在形式中的角色边界

## 决断类型：选择

## 背景

096号确立蜂群规则的三层分布式存在形式：
1. **CLAUDE.md**（基因组——声明层，自动加载）
2. **hooks**（免疫系统——强制层，自动触发）
3. **skills**（可执行知识结晶——知识层，按需/事件驱动加载）

当前 `team-structural-inject.sh` 是 PostToolUse hook（TeamCreate 后触发），功能是输出一段文字告诉 agent "你必须读取 sub-swarm-ceremony skill"。

编排者 INTERRUPT："如果是严格地这样存在，那么 hook 是不是就不够优雅和干净了"

## 矛盾分析

如果三层各司其职：

| 层 | 职责 | 机制 | 动作语义 |
|---|---|---|---|
| CLAUDE.md | 声明原则（"什么"） | 平台自动加载 | 声明 |
| hooks | 强制语法（"不许"） | PreToolUse block | 阻断 |
| skills | 可执行流程（"怎么做"） | Skill 工具按需加载 | 执行 |

那么 `team-structural-inject.sh` 作为 PostToolUse hook 输出"请读取 skill"——这个"请"不属于任何一层：
- 不是"阻断"（它 allow 了 TeamCreate）
- 不是"声明"（CLAUDE.md 已声明原则15）
- 不是"执行"（它不含可执行流程）

它是一个夹在三层之间的"提示/索引"功能。这打破了三层的纯粹边界。

## 四个选项

### A：删除 team-structural-inject.sh——三层已自足
- CLAUDE.md 原则15 已声明子蜂群规则 → agent 读了就应该知道
- agent-team-enforce.sh 已强制 Task 必须有 team_name → 语法守卫已到位
- sub-swarm-ceremony skill 已存在 → agent 可通过 Skill 工具加载
- **三层之间不需要第四个"索引层"**
- 优点：架构最纯粹，三层边界清晰
- 风险：agent 可能不会主动发现 sub-swarm-ceremony skill（CLAUDE.md 只声明原则，不列举 skill 路径）

### B：保留但改为纯 structural skill 列表输出
- hook 只输出 dispatch-dag 中注册的 structural skill 列表（名称+路径）
- 不含任何规则内容、不含"你必须"等指令
- 定位为"知识索引"——类似图书馆目录，不是规则传递
- 优点：保留发现性，不越界
- 缺点：仍然是"提示"，与"hook = 强制层"的定位不完全一致

### C：改为 PreToolUse 验证——如果 teammate 创建 team 时未加载 sub-swarm-ceremony skill，阻断
- 将 hook 从 PostToolUse（事后提示）改为 PreToolUse（事前验证）
- 检查 agent 是否已读取 sub-swarm-ceremony skill → 未读取则 block
- 优点：纯粹的"强制层"语义——hook 只做阻断，不做提示
- 缺点：技术上无法检测"agent 是否已读取 skill"（Claude Code 没有提供这种 API）

### D：删除 hook，将 skill 索引写入 CLAUDE.md
- 在 CLAUDE.md 原则15 中明确写出 sub-swarm-ceremony skill 路径
- agent 读 CLAUDE.md 时就同时获得原则声明和 skill 路径
- hook 层完全不参与"索引/提示"
- 优点：消除 hook 的越界，信息在声明层完成
- 缺点：CLAUDE.md 变长（但 skill 路径只是一行）

## 编排者约束

"如果是严格地这样存在，那么 hook 是不是就不够优雅和干净了"——暗示当前 hook 的索引/提示功能与三层纯粹边界不一致。

## 请 Gemini 决断

在 A/B/C/D 中选择（或提出更优方案）。关键决策维度：
1. 三层存在形式的边界纯粹性
2. 实际可行性（agent 能否自主发现 skill）
3. 优雅性和干净性

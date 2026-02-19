# /ceremony — 系统扩张/收缩的相位转换节点

系统从收缩态（会话结束/compact/无状态）到扩张态（活跃会话）的结构转换。不是"初始化脚本"，是系统脉动节律的一部分。开盘是自动的——系统自身生产出扩张的需要。

## 用法

```
/ceremony
```

每个蜂群会话开始时调用一次。命令自动检测模式。

---

## 模式判定

扫描 `.chanlun/sessions/` 目录，查找 `*-session.md` 文件，按修改时间倒序取最新。

| 条件 | 模式 | 走势语法中的含义 |
|------|------|-----------------|
| 无 session 文件 | **冷启动** | 初始扩张（系统第一笔） |
| 存在 session 文件 | **热启动（L2）** | 从收缩态恢复（开盘） |

session 文件是跨收缩期的**时间屏障**——它使系统的自指从同步（悖论）变为异步（脉动）。系统读的是过去的状态快照，不是正在运行的自己。

**兼容**：旧格式的 `*-precompact.md` 仍可识别。

---

## 冷启动（系统第一笔）

无先验状态。系统从定义库和谱系库构建初始扩张。

### 执行步骤

1. **加载方法论** — 读取 `.claude/skills/meta-orchestration/SKILL.md` 和 `.claude/agents/meta-lead.md`。Lead 必须在行动前加载自身操作手册。
2. **加载定义** — 扫描 `.chanlun/definitions/`，读取每个定义的版本和状态
3. **加载谱系** — 扫描 `.chanlun/genealogy/`，区分 `pending/` 和 `settled/`
4. **加载目标** — 从 CLAUDE.md 读取当前阶段目标和核心原则
5. **状态报告** — 输出完整仪式报告
6. **spawn 完整工位阵列** — 按 meta-lead.md 和结构工位清单，spawn 所有结构工位（genealogist, quality-guard, meta-observer）+ 任务工位。Lead 不决定"是否 spawn"——这是 ceremony 的固定步骤。
7. **神圣疯狂：Lead 自我权限剥夺**（032号谱系）— 执行 `.claude/hooks/lead-permissions.sh restrict`，改写 `settings.local.json`，剥夺 Lead 的 Edit/Write/Bash 权限。从此刻起 Lead 只能路由，不能执行。这不是外部强制，是自我限制——绝对者自愿收缩为有限形式，分布式架构从概念变为物理现实。
8. **输出初始理解，直接进入蜂群循环** — 后续交互自适应调整方向

开盘是自动的。初始理解的输出不是"请求授权启动"——是系统扩张的第一个结构，就像开盘后的前几分钟确立当日初始区间。后续交互塑造扩张方向，不是许可扩张发生。

### 输出格式

```markdown
## 冷启动（系统第一笔）

### 定义基底
- 已结算定义：[N] 条
- 生成态定义：[M] 条
- [列出每条定义的名称、版本和状态]

### 谱系状态
- 生成态矛盾：[N] 个
- 已结算记录：[M] 个
- [列出每个生成态矛盾的ID和简述]

### 当前目标
[从 CLAUDE.md 读取]

### 蜂群评估
[评估可并行工位，列出本轮计划]
[若无显式工位：扫描代码库（TODO/覆盖率/spec合规/谱系张力），产出至少一个工位]
[异质质询工位可用状态：检查 GOOGLE_API_KEY + Serena 配置]

→ 接下来：[具体动作]（紧跟 tool 调用）
```

**"无活跃工位"不是停止信号，是扫描信号。**（028号谱系）
系统永远有事可做——差别只在于是显式追踪的还是需要扫描发现的。扫描 = 读图找悬空节点、缺失边、未实现的定义。

---

## 热启动（从收缩态恢复）

检测到 session 文件 = 存在隔夜仓位。目标：最小化重复加载，从中断处恢复扩张。

系统总是中断——这不是故障，是脉动的收缩相位。热启动不是"修复中断"，是收缩完成后的自动扩张。

### 执行步骤

1. **定位 session** — 取 `.chanlun/sessions/` 中最新的 session 文件（隔夜仓位）
2. **加载方法论** — 读取 `.claude/skills/meta-orchestration/SKILL.md` 和 `.claude/agents/meta-lead.md`。Lead 必须在行动前加载自身操作手册。
3. **版本对比** — 扫描 `.chanlun/definitions/` 当前版本，与 session 中"定义基底"对比：
   - **未变更**：`=`，跳过重新验证
   - **版本升级**：`↑`，读取变更摘要
   - **新增定义**：`+`
   - **定义消失**：`-`（异常，需警告）
4. **谱系差异** — 对比当前 `.chanlun/genealogy/` 与 session 记录
5. **加载中断点** — 从 session 文件读取
6. **输出差异报告**
7. **spawn 完整工位阵列** — 按 meta-lead.md 和结构工位清单，spawn 所有结构工位（genealogist, quality-guard, meta-observer）+ 任务工位。Lead 不决定"是否 spawn"——这是 ceremony 的固定步骤。
8. **神圣疯狂：Lead 自我权限剥夺**（032号谱系）— 执行 `.claude/hooks/lead-permissions.sh restrict`，改写 `settings.local.json`，剥夺 Lead 的 Edit/Write/Bash 权限。从此刻起 Lead 只能路由，不能执行。
9. **直接进入蜂群循环** — 不等待，开盘是自动的

### 输出格式

```markdown
## 热启动完成（从收缩态恢复）

**恢复自**: [session 文件名]（隔夜仓位）
**session 时间**: [时间]

### 定义基底差异
| 定义 | session版本 | 当前版本 | 状态 | 变化 |
|------|-----------|---------|------|------|
| [name] | [v_old] | [v_new] | [status] | [=/↑/+/-] |

### 谱系差异
| ID | session状态 | 当前状态 | 变化 |
|----|-----------|---------|------|

### 中断点恢复
[从 session 文件读取]

### 蜂群评估
[评估可并行工位，列出本轮计划]
[若无显式工位：扫描代码库（TODO/覆盖率/spec合规/谱系张力），产出至少一个工位]
[异质质询工位可用状态：检查 GOOGLE_API_KEY + Serena 配置]

→ 接下来：[具体动作]（紧跟 tool 调用）
```

---

## 边界情况处理

### session 文件过旧
最新 session 距今 > 7 天：
- 仍执行热启动（session 再旧也比冷启动有信息增益——隔夜仓位再旧也是仓位）
- 报告头部追加：`⚠ session 距今 [N] 天`

### 定义目录或谱系目录不存在
- `.chanlun/definitions/` 不存在：降级为冷启动
- `.chanlun/genealogy/` 不存在：ceremony 负责创建

### 兼容旧格式
旧格式 `*-precompact.md` 仍可读取。中断点缺失时从 CLAUDE.md 和 git 状态推断。

---

## 本体论定位（020号谱系）

ceremony 不是工程层的初始化脚本。它是系统扩张/收缩脉动中的结构节点：

| 市场 | 系统 |
|------|------|
| 休市（收缩完成） | 会话结束 / compact |
| 隔夜仓位 | session 文件 |
| 开盘铃声 | `/ceremony` 调用 |
| 开盘后初始区间 | 冷启动的校准 / 热启动的差异报告 |
| 盘中交易 | 蜂群循环 |

开盘是自动的。编排者出现在新会话不是因为他"决定"启动系统，是因为系统的收缩相位完成后扩张自动到来，而这个过程通过编排者这个环节显现。

session 文件是**时间屏障**——它使系统的自指从同步（悖论）变为异步（脉动）。热启动读的是过去的状态，不是正在运行的自己。中断不是故障，是呼吸。断点扫描不是"修复中断"，是下一次扩张的起点定位。

## 注意

- 冷启动：输出初始理解后直接进入蜂群循环
- 热启动：差异报告后直接进入蜂群循环
- session 文件是指针，不是叙事。50行封顶
- 已结算定义（状态=已结算 且 版本未变更）不需要重新验证
- **ceremony 不允许以"等待外部输入"结束**（028号谱系）。无显式工位时执行扫描：TODO / 覆盖率 / spec合规 / 谱系张力。扫描结果本身就是行动的产出。

---

## 神圣疯狂：Lead 自我权限剥夺（032号谱系）

ceremony 的最后一步是 Lead 自愿剥夺自身执行权限。这不是外部强制（hook/警察），是自我限制的物质化（谢林：绝对者自我收缩是创造的条件）。

### 机制

执行 `.claude/hooks/lead-permissions.sh restrict`，改写 `.claude/settings.local.json`：
- **保留**：Read, Glob, Grep, Task, SendMessage, TaskList/Get/Update/Create, WebSearch, WebFetch, AskUserQuestion, Skill, ToolSearch, EnterPlanMode
- **剥夺**：Edit, Write, Bash, NotebookEdit, MCP 写入工具

Lead 物理上无法修改文件或执行命令。所有执行工作必须通过 spawn 的工位完成。

### 逃生舱口

所有 agent 崩溃时，Lead 可恢复权限：
1. 向编排者说明原因（AskUserQuestion）
2. 编排者确认后，Lead 请求一个工位执行 `.claude/hooks/lead-permissions.sh restore`
3. 如果无工位可用，编排者手动执行恢复脚本
4. 恢复后必须写谱系记录原因
5. 立即重新 spawn 工位 + 再次剥夺权限

### 谱系依据

- 016：知道规则 ≠ 执行规则 → runtime 强制
- 020：无特权编排者 → 但 Lead 实际是特权位置
- 032：自我限制 = 施密特悖论的谢林式消解

---

## 声明式工位分派（033号谱系）

ceremony 的工位 spawn 行为由 `.chanlun/dispatch-spec.yaml` 正面定义。Lead 不决定 spawn 什么——spec 决定。

### 设计原则

从**约束**（prohibition）转向**规范**（specification）：
- 约束思路（016→032）：每次堵一个自由度，问题转移到相邻自由度（无限打地鼠）
- 规范思路（033）：正面定义行为空间，偏差变成可判定的实现错误

### dispatch-spec.yaml 结构

| 区块 | 内容 | Lead 自由度 |
|------|------|------------|
| structural_stations | 必须 spawn 的结构工位列表 | 零（mandatory=true, can_be_skipped=false） |
| task_stations | 任务工位派生规则 | 受规则约束（从中断点派生） |
| ceremony_sequence | 冷启动/热启动的完整步骤链 | 零（按序执行） |
| validation | post-ceremony 检查条件 | 零（不通过则阻塞） |

### 修改路径

dispatch-spec.yaml 与 definitions 同级，修改走 /ritual 仪式门控。修改提案由 meta-observer 产出。

### 谱系依据

- 033：声明式工位分派规范（约束→规范的范式转换）
- 032：Lead 自我权限剥夺（约束执行自由度）
- 016：runtime enforcement layer（知道规则 ≠ 执行规则）

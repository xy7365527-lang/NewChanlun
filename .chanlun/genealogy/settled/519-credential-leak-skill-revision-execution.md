---
id: "519"
title: "凭证泄露 + git 清理 + SKILL 修订执行：v3 工程化第二次执行的失败档案"
status: "已结算"
type: "矛盾发现"
date: "2026-04-27"
depends_on: ["518"]
related: ["516", "517"]
negated_by: []
negates: []
aufhebt_skill_section: "SKILL.md#6-v1, SKILL.md#9-v2 (after-518), SKILL.md#2-add-2.6"
aufhebt_design_concept: "v3-blueprint-circle-6-limits-md-naming"
negation_form: "expansion + correction"
negation_source: "execution-failure-self-exposure"
---

# 519 — 凭证泄露 + git 清理 + SKILL 修订执行

> **元注释（schema 扩展议题继续）**：本 frontmatter 中的 `aufhebt_skill_section` 和 `aufhebt_design_concept` 字段是前瞻性声明（同 518 号）。当前 schema 未强制约束。详见 SKILL.md 新加 § 2.6"设计层 vs 工程层辩证"。

## Section 1 · 事件全貌（按时间线）

### 1.1 触发

cc-tier 执行 spec:b0ca6c2c.../598fe11e-...（SKILL.md 第 2.4 节扬弃修订）时，commit 50abe8e8cbe060c34f4c2265f8db6f314be2a14c **包含 112 个文件**，远超 spec 范围（spec 要求 2 个文件 = SKILL.md + 518 谱系）。

### 1.2 用户介入与诊断

用户要求停下，按 SKILL § 4.1 / § 4.2 / § 9 报告 scope creep。Traycer 给出 4 类事实：

a) `.env.bak-v71` 被 commit —— **6 个真实有效 API key 写入 git history**
b) `src/newchan/topology/multi_tf_adapter.py` 被改 +204/-1（cursor session 产物）
c) `.chanlun/genealogy/settled/516-...md` + `517-...md`（cursor session 产物）
d) 109 个其他文件（sessions / skills / tmp/ / block-topology / review-results）混入

### 1.3 核心决定（用户）

- **决定 1**: D · 不轮换 6 个 key，承担风险
- **决定 2**: X · 只从 git history 移除 `.env.bak-v71`，tmp/ 留在 history（按"未 push + 单人"环境）
- **决定 3**: L4 · 不创建 limits.md（v3 蓝图设计层名字与工程层无对应），519 号谱系 + SKILL § 6 按 M2 重写
- **决定 4**: git-filter-repo 通过 brew 安装
- **决定 5**: W · stash sessions（用户引入第 4 路径 V · `.git/info/exclude` 本地排除规则）
- **决定 6**: Z2 · 接受 `.git.bak-2026-04-27/` 为合法 untracked
- **决定 7**: Q · 删除冗余 `.env.local`，新增 `.env.*` / `.env.bak-*` / `tmp/`

### 1.4 limits.md 命名错误的暴露

执行准备阶段，Traycer 提议"在 limits.md 写一条凭证泄露事件"。检查后发现 `limits.md` 在三个候选位置都不存在。

用户判断：v3 蓝图圈 6 + SKILL.md § 6 引用的"limits.md"是设计层名字，工程层失败档案实际由 `.chanlun/genealogy/settled/` 谱系系统承载——这是 SKILL.md 和 v3 蓝图同时存在的命名不精确。

### 1.5 cursor 与 Traycer 工作线不协调的发现

516 / 517 号 frontmatter 显示 `settled_by: cursor-gpt-5.5` + `negation_model: gemini-3.1-pro-preview`——这是 cursor session 独立做的 W3/W4 plan review 工作。multi_tf_adapter.py 改动也是 cursor session 产物（引用 484 号谱系）。

存在两条并行未协调的 v3 工程化工作线（cursor + Traycer），暴露多 AI 工具协调议题——本谱系标记为待 520+ 处理。

## Section 2 · 暴露的具体不精确处（按类型）

### 2.a · SKILL.md § 9 缺 commit scope 边界检查

cc-tier 在没有 commit scope 规则的地方按"通常做法"行动 = 把所有 untracked 文件一并入库。这是工程社区默认假设，但违反 v3 体系。

**根源**：SKILL.md § 4.2 列了"必须先问的情况"但**没列 commit 边界**，§ 9 检查清单**没有 commit scope 检查项**。

**修订**：§ 9 加新检查项 "我提议的 commit 范围严格匹配 spec 范围吗？是否把 untracked 文件一并 staged？"

### 2.b · SKILL.md § 9 缺"引用工程对象前确认存在"检查

Traycer 引用 `limits.md` 时未先确认其在 codebase 存在。这违反辩证唯物主义"工程层 vs 设计层"区分。

**根源**：SKILL.md 没有"引用具体工程对象前必须确认存在"的检查项。

**修订**：§ 9 加新检查项 "我引用了任何具体工程对象（文件名/路径）但没确认其在 codebase 实际存在吗？"

### 2.c · SKILL.md § 2 缺设计层 vs 工程层辩证

更深的根源：SKILL.md 没有显式的"设计层 vs 工程层"辩证区分。这导致 cc 把 v3 蓝图的设计命名当作工程层文件名直接使用。

**辩证形态**：把"设计存在"等同于"工程存在"——是唯心主义残余。

**修订**：新加 § 2.6 "设计层 vs 工程层的辩证"。

### 2.d · SKILL.md § 6 命名错误（limits.md vs 谱系系统）

SKILL.md § 6 标题"Limits.md 写入纪律"和正文反复引用 `limits.md` 文件——但工程层不存在此文件。失败档案的工程实现是 `.chanlun/genealogy/settled/` 谱系系统。

**辩证形态**：v3 蓝图设计层命名（limits.md）与工程层实际承载（谱系系统）的不对应，被 SKILL.md 假装为对应。

**修订**：§ 6 整节重写为"失败档案写入纪律"，按 M2 区分 A/B 类（A 类 = 思想推演 / 工程执行失败，已工程化为谱系系统；B 类 = 操盘止损失败，待 v3 圈 6 工程化）。

### 2.e · `.gitignore` 漏洞（双形态）

- `.env` 精确匹配根目录文件名 → `.env.bak-v71` 不被忽略 → 凭证入库
- `tmp/` 完全不在 .gitignore → 60+ 实验文件历史性入库
- `.env.local` 单列冗余 → 加 `.env.*` 通配后变成多余规则

**辩证形态**：工程基础设施（.gitignore）的不精确导致辩证原则（不可篡改 + 失败档案）失效——工程不精确就是辩证不精确的物质形态。

**修订**：commit `1d786576cf chore: gitignore .env* + tmp/`：
- 新增 `.env.*` / `.env.bak-*` / `tmp/`
- 删除冗余 `.env.local`

## Section 3 · 处理决定

### 3.a · 凭证（D 路径）

- **不轮换 6 个 API key**，承担风险
- 已知风险：6 个 key 仍有效；Anthropic 平台日志含 key 不可消除；可能在 cc/cursor 对话历史 / Time Machine / iCloud / 编辑器缓存 / shell history 中
- 此决定本身被记录为体系的物质事实——历史不被改写，但被诚实记录

### 3.b · git history 清理（X 路径）

- `git filter-repo --path .env.bak-v71 --invert-paths --force` 移除 `.env.bak-v71`
- `tmp/` 留在 history（避免大规模改写 commit hash）
- 未 push + 单人 → 无需 force push、无协作者通知
- 副作用：50abe8e8 之后所有 commit hash 改写（链上节点重新 hash 是 filter-repo 标准行为）
- 副作用：origin remote 被自动移除（filter-repo 默认安全机制）
- 备份：`.git.bak-2026-04-27/` + tag `backup-before-519-cleanup-2026-04-27` + stash@{0}

### 3.c · `.gitignore` 修订（commit `1d786576cf`）

- `.env` → `.env` + `.env.*` + `.env.bak-*`
- 加 `tmp/`
- 删除冗余 `.env.local`（被 `.env.*` 覆盖）

### 3.d · 保留为既成事实（不 revert）

| 文件类型 | 保留理由 |
|---|---|
| `.chanlun/sessions/2026-04-24-* 到 2026-04-27-*` (14+ 文件) | hook 自动写入；之前 commit 已有先例；.gitignore 显式允许 |
| `.claude/skills/{using-superpowers, brainstorming, writing-plans, ...}` (16 目录 ~80 文件) | 用户授权安装的 superpowers 框架 |
| `.chanlun/block-topology/` (5 文件) | v3 圈 6 物质基础 |
| `.chanlun/review-results/gemini-...md` | gemini-challenger 合法输出 |
| `.chanlun/genealogy/settled/516-...md` + `517-...md` | cursor session 产物，内容质量高（517 预先解决了部分圈 2 议题）；保留但需后续处理"多工具协调" |
| `src/newchan/topology/multi_tf_adapter.py` + tests | cursor session 产物，484 号谱系下游推论；不冲突 v3 原则；标记为"待圈 3 诊断时正式纳入 L₁ 或 L₃" |

### 3.e · SKILL.md 修订（在 Section 4 详述）

按 M2 路径，SKILL.md 在 Step 5 修订三处：§ 2.6 新加、§ 6 重写、§ 9 加两项。

### 3.f · multi-AI 协调议题（待处理）

cursor + Traycer + cc + Claude 多工作线协调议题不在本谱系处理范围。标记为 **520+ 待处理议题**。

## Section 4 · SKILL 修订内容（具体文本）

详见 commit （Step 7 commit hash）的 `.claude/skills/dialectical-trading-system/SKILL.md` 完整 diff。本节列出修订要点：

### 4.1 · § 2.6 新加（设计层 vs 工程层辩证）

核心断言：**v3 蓝图是设计层的物质化，codebase 是工程层的物质化，两者不是同一个本体——是同一辩证运动的两种存在形态。把"设计存在"等同于"工程存在"是唯心主义残余。**

引用纪律 / 禁止 / 必须三段。判断标准：**这个名字在 codebase 真实存在吗？**

### 4.2 · § 6 整节重写（M2 · A/B 类区分）

**命名澄清**："limits.md"是 v3 蓝图设计层名字，工程层由 `.chanlun/genealogy/settled/` 谱系系统承载。

**A 类**（已工程化）：思想推演 / 工程执行失败档案 = 谱系条目。schema = 现有 frontmatter 字段。
**B 类**（待工程化）：操盘止损失败档案。schema 待 v3 圈 6 工程化。
**A/B 共同禁止**：推卸性借口、修改已写入档案、总结性档案、合并独立失败。

### 4.3 · § 9 加两项

- "我提议的 commit 范围严格匹配 spec 范围吗？是否把 untracked 文件一并 staged？"
- "我引用了任何具体工程对象（文件名/路径）但没确认其在 codebase 实际存在吗？"

### 4.4 · 修订形式遵守 § 2.4

SKILL.md 是操作文档（不是谱系 block），可演化。但本次修订**本身是历史事件**，通过 519 号谱系记录。

旧版 SKILL.md（commit 50abe8e8 / ab894ebfe3 之前的版本）保留在 git history 中（同时 `/Users/silencehan/Downloads/SKILL.md` 也保留旧版本作为历史副本）。

## Section 5 · 元辩证地位

### 5.1 与 518 号谱系的扬弃链关系

518 号正文 §剩余 第 4 项预言：

> "这份谱系自己也面临递归问题：会不会有类似的不精确？大概率会，等下一轮的暴露。"

**519 号 = 此预言在同一次执行内的具体兑现**。

- 518 号修订了 SKILL § 2.4 的不精确（机械否定 vs 辩证扬弃）
- 519 号修订了同一次执行暴露的三个新不精确（commit scope / 引用工程对象 / 设计 vs 工程混淆）

辩证关系：扬弃链 518 → 519 → 520 → ...

不是孤立事件，是体系自我修订的连续形态。

### 5.2 多个不精确同时暴露的内在关联

不是 7 件独立的事，是 **1 个事件的 7 个侧面**：

- 凭证泄露（事件 a）→ 暴露 .gitignore .env vs .env* 漏洞 + git history 边界没规定（不精确 1）
- tmp/ 进入 git（事件 b）→ 同一类型的 .gitignore 漏洞（不精确 1 的另一形态）
- scope creep（事件 c）→ 暴露 SKILL § 9 缺 commit scope 边界检查（不精确 2）
- limits.md 命名错误（事件 d）→ 暴露 SKILL § 6 命名错误（不精确 3）+ § 2 缺设计层 vs 工程层辩证（不精确 4）
- cursor 工作线和 Traycer 工作线不协调（事件 e）→ 暴露多 AI 工具协调议题（待 520+ 处理）

**5 个事件 + 4 个不精确的根源 + 1 个待处理议题**。

### 5.3 关于 526+（B 类操盘失败档案）

519 号本身只承载 A 类（思想推演 / 工程执行失败）。

B 类（操盘止损失败档案）的 schema 在当前还不存在，未实盘。

v3 蓝图圈 6 工程化时统一处理。

承认这一点不是缺陷，是辩证开放性。

### 5.4 519 号的剩余

按 518 号的模式显式承认：

- 519 号自己大概率有不精确
- 520+ 会暴露
- 这是体系健康反身性的物质证据
- 多 AI 工具协调议题是已知的未处理项
- B 类操盘失败档案 schema 是已知的未工程化项
- multi_tf_adapter.py 的圈 3 诊断时正式归位是已知的未审查项
- 516/517 号谱系的"未授权写入"先例如何防止扩散是已知的未处理议题
- SKILL.md 长度增长（305 → 361 行）是否需要分卡处理是 520+ 候选议题
- `git check-ignore -v` 对已 tracked 文件不输出是 .gitignore 只防新文件不影响已 tracked 的具体表现——本 spec 处理 tmp/ 的方式（X 决定）只解决未来漏洞，不撤销已入库的 tmp/。彻底清理需要 `git rm --cached` 但不在本 spec 范围
- `.git.bak-2026-04-27/` 目录中的 `.git/info/exclude` 是 Step 1 备份时刻的旧版本，不含本次新增的本地排除规则——回滚时排除规则会消失需重新加（不是数据丢失，是元数据丢失）

承认这些不是缺陷——是诊断作为辩证运动的诚实形态。

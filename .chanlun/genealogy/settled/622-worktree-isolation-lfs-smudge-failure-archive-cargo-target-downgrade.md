---
id: "622"
number: 622
status: 已结算   # 本轮蜂群并行隔离方案降级的缺口落盘（worktree checkout LFS smudge 失败 → git archive + CARGO_TARGET_DIR）。同根因 549/476（125MB LFS object 本地不可达），新显形面（隔离层，非映射管线层）。最终结算待编排者 /ritual。
settled_date: "2026-07-02"
settled_by: "genealogist via /ritual（编排者明令『并行全部推进』授权；裁决来源 codex 裁决①-⑤ + staging 归档）"
date: "2026-06-26"
type: 矛盾发现
depends_on: ["549", "476"]
related: ["178", "347", "no-workaround", "no-patch-mentality", "037", "056"]
title: "worktree 隔离因 relations.jsonl 125MB LFS smudge 失败降级 = 549/476 同根因（LFS object 本地不可达）在『蜂群并行隔离层』的新显形面：git worktree checkout 时 smudge filter 对 relations.jsonl 拉取失败致 worktree 创建受阻 → 隔离方案降级为 git archive + CARGO_TARGET_DIR（隔离 cargo target 而非 git worktree）。区分 549（mapper 读 LFS 指针崩溃，scope=block-topology 映射管线）vs 本号（worktree checkout smudge 失败，scope=蜂群并行隔离）——同一 125MB object 不可达，两个互不覆盖的冻结 scope"
negation_source: homogeneous
negation_form: waiting
# waiting：après-coup——隔离层降级（git archive + CARGO_TARGET_DIR）已落地为本轮可用方案，
#   但『worktree 原生隔离不可用』的回溯解冻仍待 549 §十 A 执行（git lfs pull 实体化 125MB object）。
#   A 执行后 worktree checkout 的 smudge 可正常完成 → 隔离层可回归原生 worktree → 本号回溯解冻。

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# negates：隐含命题『蜂群并行可用 git worktree 做工位隔离（每工位独立 checkout）』
# 实际拓扑后果（retrospective 141号结论1）：worktree 隔离层被冻结并降级——
#   worktree checkout 触发 relations.jsonl 的 smudge filter（LFS → 实体文件）拉取，
#   125MB object 本地不可达（549 §九.2：.git/lfs/ 不存在 + git-lfs CLI 未安装）→ smudge 失败 → worktree 创建受阻。
#   降级到 git archive（不走 smudge，直接导出工作树快照）+ CARGO_TARGET_DIR（隔离编译产物，非隔离 git 工作树）。
topo_effect: "freeze:worktree-native-isolation:swarm-parallel-isolation-layer（冻结 git worktree 原生隔离；解冻条件 = 549 §十 A 执行 git lfs pull；降级方案 git archive + CARGO_TARGET_DIR 作为冻结期可用替代）"

# 矛盾（type=矛盾发现 必填）
contradiction:
  description: "蜂群并行需要工位级隔离（避免并行工位抢同一 cargo target / 互相污染工作树）。原生方案 = 每工位 git worktree 独立 checkout。但 git worktree checkout 会对仓库内 LFS 跟踪文件触发 smudge filter（把 LFS 指针实体化为真实文件）——relations.jsonl 是 125MB LFS object（meta 声称 size 125404332），其 object 本地不可达（549 §九.2 实测：本机 .git/lfs/ 目录不存在 + git-lfs CLI 未以任何形式安装 + object 不在本地缓存，四种检查全空）。⟹ smudge filter 对 relations.jsonl 拉取失败 → worktree checkout 受阻 → 原生 worktree 隔离不可用。"
  layer: 编排   # 蜂群并行隔离层（基础设施/编排），非缠论域、非 formal/ 架构层
  trigger: "本轮蜂群并行 4 方向（rust 重锚 #127 / 深层 Lean #128-131 / L2 回测 #132 / 清理结算 #133-134），尝试用 git worktree 给并行工位做隔离时，worktree checkout 因 relations.jsonl smudge 失败受阻；隔离方案降级为 git archive + CARGO_TARGET_DIR"

# 涉及的定义
definitions_involved:
  - name: "549 relations.jsonl 沦为未实体化 LFS 指针（冻结 block-topology 映射管线）"
    version: ".chanlun/genealogy/settled/549（status: 已结算）"
    role: "**同根因前置**。549 的根因（relations.jsonl = 125MB LFS object，本地不可达：.git/lfs/ 不存在 + git-lfs CLI 未装）与本号**完全同一**。但 549 的 topo_effect scope = `freeze:block-topology-mapping-pipeline:downstream`（mapper 读 LFS 指针首行 json.loads 崩溃）。本号是**另一个互不覆盖的显形面**：worktree checkout 的 smudge filter 失败（隔离层，非映射管线层）。549 §十 A（git lfs pull）= 本号同一解冻条件——一个 A 解冻两个 scope。"
  - name: "476 JSONL append-only ⊥ long-running（迁 LFS 根源）"
    version: ".chanlun/genealogy/settled/476（status: 已结算）"
    role: "**抽象根源前置**。476：append-only JSONL 长运行无界 append → 膨胀 → 迁 LFS。relations.jsonl 膨胀至 125MB → 迁 git-lfs 是 476 预言的物质后果。549 是 476 在 git-lfs 层的第一显形面（mapper 崩溃），本号是 476 在 git-lfs 层的**第二显形面**（worktree smudge 失败）。两个显形面共享 476→125MB→LFS→本地不可达的同一因果链。"
  - name: "178 区块拓扑建系 / 347 content-addressed block topology"
    version: ".chanlun/genealogy/settled/178 + 347（status: 已结算）"
    role: "数据来源。relations.jsonl 是 178 建立的 append-only 关系层，347 声明 block = source of truth。本号不改其概念结论，只记录该文件在 worktree 隔离场景下因 LFS 不可达造成的工程后果。"
  - name: "no-workaround / no-patch-mentality"
    version: ".claude/rules/no-workaround.md + no-patch-mentality.md §3"
    role: "约束降级方案的性质。git archive + CARGO_TARGET_DIR **不是补丁**——它不是『用残缺数据覆盖正典让其先跑通』（那是 549 §十排除的 B/C），而是在 worktree 原生隔离冻结期的**诚实降级**（明确声明：隔离 cargo target 而非 git worktree，原生隔离待 A 执行解冻）。关键区分：降级方案不触碰 relations.jsonl（不 append、不覆盖、不重生成），故不违 no-workaround；它是在约束下的精确存在论表达（隔离层换实现，不降低严格性）。若降级被冒充为『worktree 隔离已实现』则是声明膨胀（090）——本号明确标注降级 ≠ 原生 worktree。"

# 解决方式
resolution:
  type: 未解决   # 缺口记录（隔离层降级）。原生 worktree 解冻条件 = 549 §十 A（git lfs pull），与 549 共享同一基础设施待办。降级方案 git archive + CARGO_TARGET_DIR 为冻结期可用替代，非补丁。
  description: "worktree 隔离因 relations.jsonl 125MB LFS smudge 失败（549/476 同根因：LFS object 本地不可达）而冻结，降级为 git archive（导出工作树快照，不走 smudge filter）+ CARGO_TARGET_DIR（隔离编译产物，每工位独立 target 目录）。这是 549/476 根因在『蜂群并行隔离层』的新显形面——与 549（映射管线层）互不覆盖。解冻条件 = 549 §十 A 执行（git lfs pull 实体化 125MB object），与 549 共享同一基础设施待办（蜂群无 git-lfs CLI / 远端凭据权限，无法自行执行）。降级方案不触碰 relations.jsonl（不违 no-workaround），是冻结期诚实降级。"
  decided_by: 蜂群内部   # genealogist 落盘 549/476 同根因新显形面；隔离降级是工程行动（基础设施待办，蜂群无权限执行 A）；最终结算待编排者 /ritual

# 被否定的方案
negated:
  description: "(1) 用 git worktree 给并行工位做原生隔离（每工位独立 checkout）。(2) 为绕过 smudge 失败而对 relations.jsonl 做有损处理（如 .gitattributes 临时移除 LFS 跟踪 / 用 .bak 覆盖让 checkout 通过）。"
  why_negated: "(1) worktree checkout 触发 relations.jsonl 的 smudge filter，125MB LFS object 本地不可达（549 §九.2）→ smudge 失败 → worktree 创建受阻。原生 worktree 隔离在 A 执行前不可用。(2) 任何对 relations.jsonl 的有损处理（移除 LFS 跟踪 / .bak 覆盖）= 549 §十排除的 B/C 类有损 workaround（违 no-workaround）——把 6889 正典缩成残缺态让 checkout『先跑通』。故降级方案选 git archive（绕过 smudge 但**不修改** relations.jsonl，导出当前工作树快照）+ CARGO_TARGET_DIR（隔离编译产物），既隔离又不触碰正典。"

# 新产出
new_output:
  definitions:
    - "549/476 根因的第二显形面：worktree checkout smudge 失败（隔离层）——与 549 第一显形面（mapper 读崩溃，映射管线层）互不覆盖，共享同一 125MB object 不可达根因"
    - "两个互不覆盖的冻结 scope：549 freeze block-topology 映射管线 / 622 freeze worktree 原生隔离——一个 549 §十 A（git lfs pull）解冻两者"
    - "隔离层诚实降级：git archive（不走 smudge，导出工作树快照）+ CARGO_TARGET_DIR（隔离 cargo target，非 git worktree）——不触碰 relations.jsonl 故不违 no-workaround，明确标注降级 ≠ 原生 worktree（防声明膨胀 090）"
    - "smudge filter 是 worktree checkout 的隐藏 LFS 依赖：仓库内任一 LFS 跟踪文件 object 不可达 → 所有 worktree checkout 受阻（不仅 relations.jsonl 的消费者）"
  code_changes: "无谱系侧代码改动（本号是缺口记录）。隔离层工程动作（git archive + CARGO_TARGET_DIR）由蜂群并行编排执行，不触碰 relations.jsonl。原生 worktree 解冻待 549 §十 A（git lfs pull）= 基础设施待办（蜂群无权限）。"
  orchestration_changes: "方法论：①蜂群并行工位隔离不依赖 git worktree（仓库含不可达 LFS object 时 worktree checkout 受阻）——用 git archive + CARGO_TARGET_DIR。②任何依赖 git worktree/checkout 的机制须先核对仓库内 LFS 跟踪文件是否本地实体化（549 §九.2：本机 git-lfs 全缺）。③隔离降级不触碰 relations.jsonl（有损处理=549 §十排除的 B/C=违 no-workaround）。④降级方案明确标注 ≠ 原生 worktree 隔离（防声明膨胀），原生隔离解冻 = 549 §十 A 执行。"

# 影响范围
impact:
  affected_modules:
    - ".chanlun/block-topology/relations.jsonl → 125MB LFS object 本地不可达，worktree checkout 的 smudge filter 对其失败（549 同根因，新显形面）"
    - "git worktree 原生隔离 → 冻结（worktree checkout 受阻），降级为 git archive + CARGO_TARGET_DIR"
    - "本轮蜂群并行 4 方向（#127 rust 重锚 / #128-131 Lean / #132 L2 回测 / #133-134 清理）→ 隔离层走降级方案（git archive + CARGO_TARGET_DIR），不阻塞业务推进"
    - "549（已结算）→ 本号是其同根因的第二显形面（隔离层），不破坏其结算，共享 §十 A 解冻条件。549 维持 settled"
    - "476（已结算）→ 本号是其 git-lfs 层第二显形面（worktree smudge），不破坏其结算。476 维持 settled"
  affected_definitions:
    - "549（已结算）：同根因新显形面（scope 互不覆盖：映射管线 vs 隔离层）。维持 settled，本号补一个未被 549 topo_effect 覆盖的冻结 scope。"
    - "476（已结算）：git-lfs 层第二显形面。维持 settled。"
  downstream_implications:
    - "原生 worktree 隔离解冻 = 549 §十 A（git lfs pull）执行——与 549 映射管线解冻是同一基础设施待办（一个 A 解冻两个 scope）"
    - "蜂群并行隔离用 git archive + CARGO_TARGET_DIR（不依赖 worktree，不触碰 relations.jsonl）"
    - "任何 git worktree/checkout 依赖须先核对 LFS 跟踪文件本地实体化状态（549 §九.2：本机 git-lfs 全缺）"

# 谱系关联
related_records:
  parent: "549号（relations.jsonl 沦为 LFS 指针冻结映射管线）——本号是其同根因（125MB object 本地不可达）在『蜂群并行隔离层』的第二显形面，共享 §十 A 解冻条件"
  children: []
  related:
    - "476号（JSONL append-only ⊥ long-running）：抽象根源（膨胀→迁 LFS），本号是其 git-lfs 层第二显形面"
    - "178号（区块拓扑建系）/347号（content-addressed block）：relations.jsonl 来源"
    - "no-workaround / no-patch-mentality §3：约束降级方案性质（git archive 不触碰 relations.jsonl=不违；有损处理 relations.jsonl=违）"
    - "037号（递归蜂群分形）/056号（蜂群递归默认模式）：蜂群并行隔离需求的架构根"

# 认识论等级标注（formalization-validity-domain 强制）
epistemological_levels:
  - proposition: "relations.jsonl = 125MB LFS object，本地不可达（.git/lfs/ 不存在 + git-lfs CLI 未装）"
    level: "L0/L1（549 §九.2 四种检查全空实测 + LFS 指针 size 字段 125404332）"
    increment: "高：与 549 共享同一根因事实"
  - proposition: "worktree checkout 触发 relations.jsonl smudge filter，object 不可达 → checkout 受阻"
    level: "L0（git LFS 机制：worktree checkout 对 LFS 跟踪文件触发 smudge 实体化，object 缺失则失败）"
    increment: "高：549 未覆盖的隔离层显形面"
  - proposition: "降级方案 git archive + CARGO_TARGET_DIR 不触碰 relations.jsonl（不违 no-workaround）"
    level: "L0（git archive 导出工作树快照不走 smudge；CARGO_TARGET_DIR 隔离编译产物——均不修改 relations.jsonl）"
    increment: "高：降级 ≠ 有损 workaround 的结构判定"
  - proposition: "解冻条件 = 549 §十 A（git lfs pull）"
    level: "L0（同根因推论：A 实体化 125MB object → smudge 可完成 → worktree 可用）"
    increment: "中：与 549 共享解冻条件（一个 A 解冻两 scope）"

---

> **[/ritual 结算段 · 2026-07-02]** 裁决来源：codex 裁决①-⑤（`.chanlun/review-results/codex-ritual-*.md`）+ 编排者明令「并行全部推进」授权。判决全文见 staging：GRAMMAR §1-A。判决摘要：worktree 隔离 LFS smudge 失败同根 549/476，成立。


# 矛盾发现 622：worktree 隔离 LFS smudge 失败 = 549/476 同根因在『蜂群并行隔离层』的新显形面

## 一句话结论

本轮蜂群并行尝试用 **git worktree** 给工位做隔离时，worktree checkout 触发 `relations.jsonl` 的 **smudge filter**（把 LFS 指针实体化），但该文件是 **125MB LFS object 且本地不可达**（549 §九.2 实测：本机 `.git/lfs/` 不存在 + git-lfs CLI 未装 + object 不在本地缓存）→ smudge 失败 → worktree 创建受阻。隔离方案**降级为 git archive（导出工作树快照，不走 smudge）+ CARGO_TARGET_DIR（隔离 cargo 编译产物，非隔离 git 工作树）**。

这是 **549/476 同一根因（125MB LFS object 本地不可达）的第二显形面**——与 549（mapper 读 LFS 指针崩溃）**互不覆盖**：

| | 549（已结算） | 622（本号） |
|---|---|---|
| 显形面 | mapper `read_all_relations` 对 LFS 指针首行 `json.loads` 崩溃 | git worktree checkout 的 smudge filter 对 LFS object 拉取失败 |
| 冻结 scope | `freeze:block-topology-mapping-pipeline:downstream`（映射管线层） | `freeze:worktree-native-isolation`（蜂群并行隔离层） |
| 触发场景 | 跑 `map_genealogy_to_blocks.py` | 蜂群并行用 git worktree 隔离工位 |
| **共享根因** | relations.jsonl = 125MB LFS object 本地不可达（549 §九.2） | **同一** |
| **共享解冻条件** | 549 §十 A：`git lfs pull` 实体化 object | **同一**（一个 A 解冻两 scope） |

## 矛盾的精确形式

蜂群并行需工位级隔离（避免并行工位抢同一 cargo target / 污染工作树）。原生方案 = 每工位 git worktree 独立 checkout。但：

1. git worktree checkout 对仓库内 LFS 跟踪文件触发 smudge filter（LFS 指针 → 实体文件）。
2. relations.jsonl 是 125MB LFS object，本地不可达（549 §九.2：四种检查全空）。
3. ⟹ smudge 失败 → worktree checkout 受阻 → 原生 worktree 隔离不可用。

## 定义依据

- 549 §九.2：本机 `.git/lfs/` 目录不存在 + git-lfs CLI 未以任何形式安装（`which git-lfs` / `git lfs version` / `brew list git-lfs` / 全盘 `find` 四种检查全空）+ object（oid `3b895e98…`，125MB）不在本地缓存。
- LFS 机制：worktree checkout 对 LFS 跟踪文件触发 smudge 实体化，object 缺失则失败。
- 549 §十：对 relations.jsonl 的有损处理（B/C）= 违 no-workaround。本号降级方案绕过此——git archive 不修改 relations.jsonl。

## 边界条件（结论翻转）

- 若 549 §十 A 执行（装 git-lfs + 配置远端 + `git lfs pull` 实体化 125MB object）→ smudge 可正常完成 → worktree checkout 不再受阻 → 原生 worktree 隔离恢复 → 本号回溯解冻（同时解冻 549 映射管线）。
- 若 relations.jsonl 从 LFS 跟踪中移除（.gitattributes 改 + 实体化为普通文件）→ smudge 不触发——但这涉及对正典的处理，须走 549 框架（不在本号擅自决定）。
- 若降级方案 git archive + CARGO_TARGET_DIR 被发现**未真正隔离**（如工位间仍抢同一 CARGO_TARGET_DIR）→ 降级方案不充分，须补隔离机制（非复活 worktree）。

## 下游推论

- 蜂群并行隔离用 git archive + CARGO_TARGET_DIR（不依赖 worktree，不触碰 relations.jsonl）。
- 原生 worktree 解冻 = 549 §十 A（git lfs pull）——与 549 映射管线解冻是同一基础设施待办（蜂群无权限执行）。
- 任何 git worktree/checkout 依赖须先核对仓库内 LFS 跟踪文件本地实体化状态。
- 降级方案明确标注 ≠ 原生 worktree 隔离（防声明膨胀 090）。

## 谱系引用

- 父：549（relations.jsonl 沦为 LFS 指针冻结映射管线）——本号是其同根因第二显形面（隔离层），共享 §十 A 解冻条件。
- 根源：476（append-only ⊥ long-running，膨胀→迁 LFS）——本号是其 git-lfs 层第二显形面。
- 数据来源：178（区块拓扑建系）/347（content-addressed block）。
- 约束：no-workaround / no-patch-mentality §3（降级方案不触碰 relations.jsonl=不违；有损处理=违）。

## 影响声明

记录 549/476 同根因（125MB LFS object 本地不可达）在『蜂群并行隔离层』的第二显形面（worktree checkout smudge 失败）。补一个未被 549 topo_effect 覆盖的冻结 scope（worktree 原生隔离）。记录隔离方案降级（git archive + CARGO_TARGET_DIR，不触碰 relations.jsonl，诚实降级非补丁）。共享 549 §十 A 解冻条件（一个 git lfs pull 解冻两 scope）。不破坏任何 settled 谱系（549/476/178/347 维持 settled）。不改任何拓扑数据文件、不触碰 relations.jsonl。

## 张力检查（019d/020）

### 检查范围（同轮蜂群 ∪ 1-hop ∪ Hub）
- 同轮蜂群：619（A′ canonical base）/620（#95 吸收反转）/621（结构工位自动化缺口）——本号（worktree LFS 隔离）与之同轮但不同域（619/620=formal/ 架构层，621=蜂群编排/hook 层，622=蜂群基础设施/LFS 隔离层），无张力。
- 1-hop 邻接：549/476/178/347/no-workaround。
- Hub 节点：476（LFS 根源）/549（git-lfs 层第一显形面）。

### 张力1：vs 549（映射管线冻结）——同根因不同 scope，互补不冲突
549 freeze 映射管线层，本号 freeze 隔离层——同一 125MB object 不可达的两个互不覆盖显形面。**可分层**（scope 正交）。一个 549 §十 A 解冻两者。无矛盾、无中断#1。

### 张力2：vs 476（append-only ⊥ long-running）——下游一致显形
476 预言膨胀→迁 LFS。本号是其 git-lfs 层第二显形面（worktree smudge）。**一致深化**，无矛盾。

### 张力3：vs no-workaround——降级方案合规性
549 §十排除对 relations.jsonl 的有损处理（B/C）。本号降级方案 git archive 不修改 relations.jsonl → 不违 no-workaround。**对齐**（降级是诚实，非补丁）。无矛盾。

### 递归运动结构完成检测（020）
- 第0层：本号写入（worktree smudge 失败 + 隔离降级）。
- 第1层：本号 × 549 碰撞→同根因第二显形面（净新发现高：worktree 隔离层是 549 未覆盖的新 scope）。
- 第2层：本号 × 476 碰撞→同模式确认（净新发现量骤降=背驰）。
- 涉及范围：scope₁(549 隔离层新 scope) > scope₂(476 同模式)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 解冻=549 §十 A（基础设施待办，非本号结构内矛盾），不触发新 /escalate。

## 回溯扫描（职责3）

- **549（settled）**：本号是其同根因第二显形面（隔离层 scope，549 topo_effect 未覆盖），不破坏其结算（映射管线 scope 不变）。549 维持 settled，本号补一个正交冻结 scope。
- **476（settled）**：本号是其 git-lfs 层第二显形面，不破坏。维持 settled。
- **178/347（settled）**：relations.jsonl 来源，本号不改其概念结论。维持 settled。
- **无 settled 被本号回溯破坏。** 本号是 549/476 同根因新显形面的记录，解冻条件 = 549 §十 A（共享基础设施待办）。

# 零件交接：cc-divergence → t-opentail（#65 让位）

> 我（cc-divergence，C7 未完成分支集）与 t-opentail 任务重叠，Lead 编排去重让位。
> 你的 `Strict/OpenTail.lean` 已**完整且更强**地覆盖我的全部产出（`Ext h h'` 二元历史依赖关系 >
> 我的类型索引 `ExtEvent t`；且你填了 `OpenTailSystem` 标准内核结构对接 T-kernel）。
> 我的版本被 superseded。以下是我执行中获得的、**你可能尚未掌握的事实**，供你吸收。

## 1. 关键事实：上游 RecursiveConstruction.lean 已被 T-recursive (#61) 从根上修了越界

这是本次让位中最重要的交接项。我执行时通过 codex (gpt-5.5 high) 复审发现：

**`Formal/RecursiveConstruction.lean` 当前状态（已被 T-recursive 工位改写）**：
- **删除了** `CandidateMove.outcome` 字段（原 line ~334，`def CandidateMove.outcome := classifyMove c.centers`）
- **删除了** `candidate_preserves_totality`（原把未完成压成 outcome 的越界定理）
- **新增**（line 329-444）：
  - `structure CandidateMove`（仍含 `settled : Bool` 字段，line 329-333）
  - `inductive MoveState | completed (o) | pending (tail)`（line 348-351）
  - `def CandidateMove.state`（line 368：settled 路由到 completed/pending）
  - `theorem candidate_state_unique`（line 381，状态层 ∃!）
  - `theorem settled_gives_completed` / `pending_gives_pending`（line 388/398）
  - `theorem pending_is_not_final`（line 409：`pending o ≠ completed o`）
  - `theorem candidate_state_totality`（line 425：状态层 totality）
  - `theorem candidate_overreach_rejected`（line 439：未完成不取得 completed 地位）

**推论（你必须注意）**：
- 标准第5部分**前半**（未完成→状态层 ∃!、修候选越界）**已由 T-recursive 在根上落实**，
  不是"诚实标注+入口层修正"，而是**真删除了 outcome 字段**。
- 因此你（t-opentail）**不要**在 `Strict/OpenTail.lean` 或别处重复定义状态层 API
  （`MoveState`/`CandidateMove.state`/`candidate_overreach_rejected`），那会与 T-recursive
  漂移 = 声明膨胀（codex 在我身上抓到过这个，见下）。
- 若你的 `Strict/OpenTail.lean` 用自己的 `TailLean`/`OpenState`/`OpenHist`（独立于
  `RecursiveConstruction.CandidateMove`），则与上游无耦合，**无冲突**——这是更干净的选择。
  但若你打算让 `Formal/DivergenceNesting.lean` 引用上游 `CandidateMove`，复用
  `RecursiveConstruction.MoveState`/`CandidateMove.state` 即可（不要新建）。

## 2. Formal/DivergenceNesting.lean 现状（我改了，需你统一处理）

我在让位前已对 `Formal/DivergenceNesting.lean` 做了修改（现单文件 `lake env lean` 全绿、codex PASS）：
- 顶部加了 `import Formal.RecursiveConstruction`
- `open Formal.TrendTrichotomy (... Direction)`（加了 Direction）
- 删除了我初版重复的状态层（已复用上游 `RecursiveConstruction.MoveState`/`CandidateMove.state`）
- 在末尾加了 `OpenTail` + `Ext(t)=⊔Bⱼ(t)` 分支集 + 方向语义 + 两个接缝定理
  （`openTail_candidate_is_pending`/`openTail_not_completed` 复用上游
  `pending_gives_pending`/`candidate_overreach_rejected`）

**你统一改 DivergenceNesting.lean 时的选择**（无冲突的前提）：
- **方案A（推荐，最干净）**：把 `Formal/DivergenceNesting.lean` 中我加的整个"标准第5部分后半"
  模块（从 `## 标准第5部分（后半）` 注释块到 `end` 前）**删除**——因为它已被你的
  `Strict/OpenTail.lean` superseded。同时**移除** `import Formal.RecursiveConstruction` 和
  `open ... Direction`（若 DivergenceNesting 其余部分不需要它们）。让 `DivergenceNesting.lean`
  回归它原本的职责（背驰二分 + 区间套 `Nested`/`NestingChain` + L_confirm），分支集统一归
  `Strict/OpenTail.lean`。
- **方案B**：若想保留 `Formal/` 层的分支集副本，确认它仍 import 得到上游（T-recursive 已改的
  `RecursiveConstruction`），且不与 `Strict/OpenTail.lean` 命名碰撞（不同 namespace：
  `Formal.DivergenceNesting` vs `Strict.OpenTail`，无碰撞）。但这是双份维护，按编排者"单一权威源"
  裁定（lakefile.toml 注释里对 Claim7 已有此裁定），**应选方案A**。

> 我倾向方案A——你的 `Strict/OpenTail.lean` 是更符合严格分类架构的单一权威源。

## 3. codex 审计裁定记录（避免你重复踩坑）

我的 `Formal/` 版本经过两轮 codex (gpt-5.5 high) 审计，记录可复用的裁定：

- **裁定1（Ext 必须 h 依赖）**：codex 否定"对所有 ExtEvent 全域二分"——标准要求
  `∀ ω ∈ Ext(h), ∃! j, ω ∈ Bⱼ(h)` 是**历史依赖**分区。你的 `Ext h h'` 二元关系
  （`h'.origDir = h.origDir`）已正确做到 h 依赖，比我的类型索引 `ExtEvent t` 更贴标准字面。✓
- **裁定2（方向语义必须落地）**：codex 否定"B₁/B₂ 仅标签命名"——必须连接反转=方向翻转/
  延续=方向保持。你的 `branchResultDir`/`reversal_flips_dir`/`continuation_keeps_dir`/
  `branch_result_dirs_differ` 已做到。✓
- **裁定3（不重复上游状态层）**：codex 抓到我初版重复定义 `MoveState`/`classifyState` 与上游
  漂移 = 声明膨胀。修复 = 复用上游。**你务必检查 `Strict/OpenTail.lean` 的
  `current_always_pending`/`current_unique` 是否与 T-recursive 的
  `candidate_state_unique`/`candidate_overreach_rejected` 重复**——若你的 `OpenHist`/`OpenState`
  是独立类型（不基于 `CandidateMove`），则不算重复（是 Strict 层的独立公理化，合法）；
  若基于 `CandidateMove`，则应复用上游不重证。
- **裁定4（olean 缓存会撒谎）**：我 `lake env lean` 全绿但 codex 报编译失败——原因是
  `.olean` 缓存了旧版 `RecursiveConstruction`。**验证时务必 `rm -f
  .lake/build/lib/Formal/RecursiveConstruction.olean .lake/build/lib/Formal/DivergenceNesting.olean`
  后再 `lake env lean`，或用 `lake env lean --trust=0`**（codex 用此确认排除缓存假象）。

## 4. ∃! 的严格表达（本项目无 Mathlib）

本项目 `lakefile.toml` 刻意不依赖 Mathlib，`∃!` notation / `ExistsUnique` **不可用**。
严格表达用展开式 `∃ x, P x ∧ ∀ y, P y → y = x`（这是 `ExistsUnique` 的定义本身，非弱化）。
我用了一个本地 `def UniqueClassify P := ∃ x, P x ∧ ∀ y, P y → y = x` 包装。
你若需要 ∃! 且 `Strict/Classification.lean`（T-kernel）已提供等价谓词，复用它即可。

---
认识论：L0（定义层，标准第5部分 + 002/beichi 第24课的结构形式）。
影响声明：本文件是 scratch 交接，非 .lean 模块，不进 build。`Formal/DivergenceNesting.lean`
的最终处理（方案A删除/方案B保留）由 t-opentail 统一决定。

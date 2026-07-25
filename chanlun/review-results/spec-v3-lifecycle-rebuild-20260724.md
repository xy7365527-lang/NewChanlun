# #231 Spec：V3 NestLifecycleBook 重建（活假设状态机，实装卡 + 勘误 + #78 修复合并规格）

- 日期：2026-07-24 ｜ 票据：issue #231（wayfinder:task；parent map #221「orphan 处置图」）
- 工作面：worktree `/tmp/kimi-nest-mainline`（分支 kimi-nest-mainline-20260717，**已全量提交的干净面**）；主仓只读；git 操作仅限本票完成后由编排者授权的一次提交
- 重建缘起：#229 定案【丢失】——原 nest_lifecycle.rs（1283 行 + #64 T11 续作 + #78 修复）从未提交、untracked 被删、全库无副本。本 spec 是重建的合并规格
- 合并来源（全部已入库，git 可查）：
  1. 实装卡 `chanlun/review-results/v3-lifecycle-statemachine-implementation-card-20260720.md`（§0–§9，下称「卡 §n」）
  2. 实装报告 `chanlun/review-results/v3-lifecycle-statemachine-impl-20260720.md`（结构锚点与 1770/0 基线）
  3. 勘误 `chanlun/review-results/erratum-v3-lifecycle-impl-20260721.md`（trend 反超真实不可达；真实通道 = pan 活窗 T11）
  4. #78 修复核验 `chanlun/review-results/code-review-issue78-20260721.md`（ForceCheck 三值化 / as_of 守卫 / cross_agree 剔自证 / 下溢守卫）
  5. #64 裁定 `chanlun/escalate/r43-lifecycle-ruling-20260721.md`（§2(d) seg_c_full、§5 授权边界）
- 纪律：v3 硬禁令（无概率推断、无回测验证、无 EMH；全部判据 = 确定性结构/力度谓词，测试全为确定性合成序列）；090（声明=能力，出切片项照卡 §9 登记）；TDD

---

## Problem Statement

map #59 Destination 组件「活假设状态机」（V3：Provisional→Confirmed/Invalidated + first_provable 钟）的实装曾落地（1770/0 绿 + #64 续作 + #78 修复核验通过），但从未提交，untracked 状态下被删、不可恢复（#229 定案）。#43 重审材料与 #64 裁定书（纯文档）仍在；实装卡/报告/勘误/修复核验四类规格全部在案。问题不是「设计没了」而是「代码没了」——按既有规格在干净面上重建，落进它自己的 commit，不再造第二个未提交孤儿。

语义缺口（不重建就一直在）：`judge_at` 一钟多职（首见/首证/确认挤一个字段）、反超被丢成 None 无审计、Provisional 无载体（E2E-D5 三态未落地）、pan 活窗真实反超无审计通道（勘误定案的唯一真实可达路径）。

## Solution

按卡 §2–§7 重建 `rust/src/theta_v0/classifier/nest_lifecycle.rs`（新文件 + mod.rs 注册一行），含 **#78 修复语义**与**勘误口径**，范围为原切片原样：**状态机核心 + 消费契约**，不含生产 bin 接线（卡 §9 出切片项全部维持出切片）。

1. **sidecar 注册表**：`NestLifecycleBook = BTreeMap<LifecycleKey, NestLifecycleEntry>`（范式复用 persistent.rs 注册表 / recursive_tower CpScanOwnership / p92 YieldBook first-write-wins）。事件流零改动（T5 bit-exact 护栏）；状态只活在新 entry。
2. **三态 + 五钟**：`NestEventState { Provisional, Confirmed, Invalidated }`（Unresolved 出切片，卡 §2.4 声明=能力）；`observed_at / first_provable_at / structure_end_at / confirmed_at / invalidated_at`，首次写入不后移、终态吸收禁复活、Invalidated 留档禁删除（E2E §1:83）。
3. **LifecycleKey = (level, side, kind, seg_a, seg_c_full, b_center_start)**；trend 域 seg_c_full 未暴露的现状下沿用**白名单工程桥**（收束/回扩/活窗延展记 `Supersedes` 不记 Invalidated；模块头诚实标注「工程桥，非 E2E WireV1 EventKey，并轨前不得进入证书真值路径」，卡 §6.3/§9）。
4. **反超判据（含 #78 修复 1 三值化）**：`ForceCheck { Verified(bool), Unavailable(UnavailReason) }`，原因码 `MissingForceSeries / CoordinateMapFailed`；**仅 `Verified(false)` 进 Invalidated(ForceOvertake)**，`Unavailable` 只记 `ForceUnavailable` 审计注记（含 `force_unavailable_at`）不判 Invalidated——「不可验」≠「不再弱」。trend 域复用 `confirm_times`（终假接住）、pan 域活窗复用 `segments_diverge_or`（禁第二力度引擎）；单调性保证翻假唯一且永久（卡 §4.2）。
5. **as_of 单调守卫（#78 修复 2）**：倒退 prefix **显式拒绝**（非静默吸收），同 as_of 幂等零 delta；倒退不制造 IdentityVanished。
6. **judge_at 一个 bit 不动**（卡 §5.2）：字段/写入点/回填/CERT 主键/D3 统计全部保持；新五钟只活在新 entry。
7. **钟不变量显式化**（review judgement call 4）：`assert_invariants` 公开可调用（钟序 observed ≤ first_provable ≤ structure_end ≤ confirmed、钟 ≤ as_of、反超 invalidated ≥ first_provable、终态互洽），不只靠 debug_assert。

## User Stories

- US-01：As a map #59 维护者，I want NestLifecycleBook 三态状态机重建落地且 mod.rs 注册恢复，so that Destination 组件「活假设状态机」从【丢失】回到【实装在库】。
- US-02：As a 消费侧（v0 基例 exit.rs:154-156 预留位），I want `first_provable_at` 字段按卡 §3 衔接语义可用，so that v1 升级点「改读最深处证书 first_provable」有消费对象。
- US-03：As a 审计者，I want 反超失效全程留档（Invalidated 禁删除 + ForceEvidence 载荷 + Unavailable 审计注记），so that 「反超被丢成 None」的现状有审计链且不假杀身份（#78 修复 1）。
- US-04：As a 审计者，I want 倒退 prefix 被显式拒绝（#78 修复 2），so that 乱序投喂不可静默构造「复活」。
- US-05：As a 复审者，I want T5 bit-exact 护栏（挂/不挂 book 重跑 provider 输出 PartialEq + Debug 双判相等），so that sidecar 反流改生产有结构性证据为否。
- US-06：As a 090 维护者，I want 模块头完整登记：白名单工程桥限制（禁入证书真值路径）、Unresolved 出切片、trend T1 合成保留不代表真实可达（勘误）、§5 验收线字面矛盾（见 Further Notes），so that 声明=能力。
- US-07：As a 裁定者（#43 重审线），I want 重建严格不越卡 §6.2/§8 边界（N^δ 装配仍只消费闭合 c_p、d_parent_interval 不读本 book、消费不放开），so that #64 裁定边界零侵蚀。

## Implementation Decisions

模块与接口级（具体行号由实装定；结构锚点以实装报告为参照）：

### ID-1 模块形状

- 新文件 `rust/src/theta_v0/classifier/nest_lifecycle.rs`（唯一实装落点）+ `classifier/mod.rs` 注册一行（`pub mod nest_lifecycle;`）。既有行零改动（level_view.rs / nest.rs / divergence.rs / bins / Cargo.toml 全不触碰）。
- 核心类型：`LifecycleKey`（六元组，键不含状态/钟/行进中区间——E2E §6.1:248）；`NestLifecycleEntry`（key/state/revision/五钟）；`NestLifecycleBook(BTreeMap)` + `advance(events, as_of)` 推进入口 + `assert_invariants` 公开不变量断言。
- 范式锚（重建时照搬结构，不照抄丢失代码——规格在案）：persistent.rs:88-107 注册表、recursive_tower.rs:434-457 生命周期挂 book 内对象 + 显式推进、p92:751/:759 `entry().or_insert()`。

### ID-2 转移表（卡 §2.3 子集 + #78 修复）

- ∅→Provisional：首次观察到比较对，observed_at = as_of（只写一次）。
- Provisional 自环：D1–D4 首次同真 ⟹ first_provable_at = as_of（只写一次；`Unavailable` 不写 first_provable——#78 核验 T14 锚定）。
- Provisional→Invalidated：`Verified(false)`（反超）或身份消失（上一 prefix 有、本 prefix 不再产出该 key）；原因码 `ForceOvertake / IdentityVanished` 入 revision 载荷；身份消失路径 invalidated_at 独立于 first_provable_at。
- Provisional→Confirmed：first_provable 已写 ∧ c 结构完成（structure_end_at = as_of）∧ **完成时复核仍弱**（024:24：同一谓词在完成窗上重算为真——禁「曾经弱过」冒充，E2E §4.1:151 禁项）。
- 终态吸收：Confirmed/Invalidated 后同 key 任何后续观察零输出；无删除 API（Supersedes 迁移仅 Provisional 可达）。
- **完成但不背驰**（实装报告 §5.5 登记）：pan 完成窗 force 假且从未可证 ⟹ 诚实滞留 Provisional，不伪造 Invalidated（反超定义要求「曾可证」）。

### ID-3 反超与 ForceCheck 三值化（#78 修复 1 全量）

- `enum ForceCheck { Verified(bool), Unavailable(UnavailReason) }`；`UnavailReason { MissingForceSeries, CoordinateMapFailed }`。
- 活窗力度现算：缺 hist/dif ⟹ MissingForceSeries；`map_src_to_close_idx` 失败 ⟹ CoordinateMapFailed；齐备 ⟹ `Verified(segments_diverge_or(...))`（divergence.rs 单一力度引擎，禁第二查法）。事件通道恒 `Verified(divergence_confirmed)`（与卡「Verified 路径逐 bit 不变」一致）。
- advance 判负分支：仅 `Verified(false)` ⟹ Invalidated(ForceOvertake)；`Unavailable` ⟹ `LifecycleRevisionKind::ForceUnavailable` 审计注记 + `force_unavailable_at`，**不计 Invalidated、不写 first_provable**。
- 等力亦失效（严格 `<` 口径，divergence.rs:331-333）按卡 §4.3 登记于模块头 + 测试子场景（工程口径，不冒充教义逐字）。

### ID-4 as_of 单调守卫（#78 修复 2 全量）

- 倒退 prefix（as_of 小于该 key 已见最大 as_of）⟹ **显式拒绝**（错误返回或显式拒绝注记，实装定形式），不静默吸收；同 as_of 重复 advance 幂等零新 revision；倒退不产生 IdentityVanished。
- observed_at 建仓写一次、无任何改写点；终态吸收含桥匹配（Supersedes 链上终态同吸收）。

### ID-5 白名单工程桥（卡 §6.3 原样）

- trend 域 seg_c_full 未暴露 ⟹ 身份迁移白名单：同 key 除 interval_b/turn_source 外全等且新 interval_b ⊆ 旧 ⟹ 记 `Supersedes`（迁移链 `superseded_from` 留痕、钟不动、无 Invalidated）；seg_a 改变 ⟹ 越界 ⟹ IdentityVanished（负面对照锁定）。
- 模块头标注：工程桥非 E2E WireV1 EventKey，**并轨前不得进入任何证书真值路径**（卡 §9；N^δ 装配/d_parent_interval/基例门均不读本 book）。
- 开放登记（不实装）：当前树 NestCandidateEventExt 已带（极值价, 组锚）锚（#110/#206 线已入库）——白名单桥与两元锚并轨属后续切片，本票不碰。

### ID-6 消费边界（#64 裁定）

- #64 §2(a)「构建放开、消费不放开」为执行口径：本 book 是观察记录，不进 `d_parent_interval_snapshot/terminal` 输入、不改 `divergence_confirmed` 布尔口径、N^δ 装配仍只消费已闭合完整 c_p。
- §5 验收线字面矛盾（review (a)2：「Invalidated 至少一个真实案例可触发、可消费、可查账」 vs §2(a) 消费不放开）——按 §2(a) 执行（Invalidated 可查账、**不开放消费**），矛盾登记 Further Notes 归编排者澄清，本票不替裁。

### ID-7 验收

- 全量 `cargo test --lib` 绿（既线 #110 `extract_signals_bit_exact_digest_guard` 失败除外、勿修勿归因）；`cargo check --lib` 零新增警告。
- T1–T8 + T11 + T14/T15 全绿（Testing Decisions）。
- mod.rs 注册恢复；模块头登记齐全（US-06）。

## Testing Decisions

**好测试 = 只测外部行为**：经 `advance` 公共入口喂确定性合成事件/结构序列，断言 entry 状态/五钟/revision 载荷的外部可观察行为；无随机、无统计断言（v3 硬禁令）。TDD：测试先于实装。接缝 = 新建的 `nest_lifecycle` 模块公共接口（`NestLifecycleBook::advance` + entry 读面）——本模块即接缝，不另立。

**测试清单（卡 §7 + 勘误 + #78 修复锚定）**：

- **T1 trend 反超可触发（合成保留）**：observed→first_provable→invalidated(ForceOvertake)→终态吸收零输出。模块头与测试注释标注「合成路径，真实不可达（勘误 20260721）」。
- **T2 pan 活窗反超 + 等力边界**：as_of1 三通道成立（first_provable）→ as_of2 全假 ⟹ Invalidated；等力子场景（严格 < 不成立）登记 §4.3 口径。
- **T3 单调不复活**：翻假后 c 窗再变弱仍零新 revision（谓词层证伪器 + 状态机层零增长）。
- **T4 钟不变量**：写入不后移、钟序、≤ as_of、幂等；`assert_invariants` 全列。
- **T5 bit-exact 护栏**：真实夹具，挂/不挂 book 重跑 provider 输出 PartialEq + Debug 序列化双判相等。
- **T6 身份消失**：pan 窄锚→A′ 回退切换 ⟹ Invalidated(IdentityVanished)，与 ForceOvertake 原因码可区分。
- **T7 完成时复核**：完成窗仍弱 ⟹ Confirmed（structure_end = confirmed）；完成窗已反超 ⟹ Invalidated 而非 Confirmed（024:24 机械表达，E2E §4.1:151 禁项防护）。
- **T8 trend 身份迁移豁免**：收束迁移记 Supersedes（链留痕、钟不动）；seg_a 改变 ⟹ IdentityVanished（白名单不越界）。
- **T11 pan 活窗真实反超（勘误真实通道）**：真实 provider 夹具（复刻 level_view extended_windows + R1 全合取 MACD + retest 块），pan 活窗力度现算反超 ⟹ Invalidated 可审计——勘误定案的唯一真实可达路径，必过。
- **T14 Unavailable 不判 Invalidated**（#78 核验锚定）：缺 hist/dif ⟹ ForceUnavailable 注记、无 Invalidated、不写 first_provable。
- **T15 倒退显式拒绝**（#78 核验锚定）：倒退 prefix 拒绝/注记、同 as_of 幂等、倒退不产生 IdentityVanished。

**prior art**：原测试族全部照卡 §7 重建（确定性合成序列，T5/T11 用真实 provider 夹具——实装报告 §4 的夹具构造记录在案）；全量 `cargo test --lib` 尾行留档。

## Out of Scope

- **生产 bin 接线**（卡 §6.2 伪码的 prefix 循环投产）——后续切片；本票交付状态机核心 + 消费契约（调用方按 `PanLiveWindow`/`structure_completed` 契约喂入）。
- **Unresolved 四态**（卡 §2.4）、WireV1 全量 EventKey/StateKey/修订链、谱系两钟 opened/closed、跨级证伪（043:30）、024:28 面积乘 2 外推、postcondition 诊断钟、DeferOrphan 重判、Lean 侧 ActiveTail↔OpenTailSystem 桥（卡 §9 原样维持出切片）。
- **白名单桥与两元锚并轨**（NestCandidateEventExt 锚已在库）——后续切片。
- **`divergence_confirmed` 布尔口径、judge_at 写入/回填改动**——卡 §9 明令不授权（触碰须重过 p92 兜底对账 + p95 翻转基线）。
- **#43 裁定本身**——#64 已裁定（构建放开、消费不放开），本票不请求变更。
- **既线失败 `extract_signals_bit_exact_digest_guard`**——#110 线在案，勿修勿归因。
- **#218/#214 任何口径改动**——本票与它们正交（已入库线，不回滚不触碰）。

## Further Notes

### 与来源文档的对账

| 来源 | 本 spec 落点 |
|---|---|
| 卡 §2–§6（状态对象/身份键/转移/反超/judge_at 拆分/bit-exact 分析） | Solution + ID-1~ID-5 |
| 勘误（trend 反超真实不可达） | ID-3 事件通道恒 Verified + T1 合成标注 + T11 真实通道 |
| #78 修复 1（ForceCheck 三值化） | ID-3 全量 + T14 |
| #78 修复 2（as_of 守卫） | ID-4 全量 + T15 |
| #78 修复 3（声明=能力）/修复 4（下溢守卫） | 模块头登记（US-06）+ 实装时注意 |
| #64 §2(a) 消费不放开 / §5 验收线字面矛盾 | ID-6 |
| review judgement call 4（钟不变量 debug_assert） | Solution 7（assert_invariants 公开） |

### 登记待澄清（归编排者，不替裁）

1. **#64 §5 验收线字面矛盾**：「Invalidated 可消费」 vs §2(a)「消费不放开」——本票按 §2(a) 执行（可查账、不开放消费），待澄清后如需调整出后续票。
2. **重建评审**：原线有 shadow 评审惯例（V1-V3 + #64 曾有 code-review-v1-v3-64）——本票完成后建议配两轴 code-review（Standards + 本 spec 逐条），评审票随实装票联动。

### 提交纪律

- 本票实装落进**它自己的 commit**（编排者授权的 git 操作）；不与其他线混版。提交前全量测试绿 + 两轴 code-review 结论在档。

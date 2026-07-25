# #78 状态机三项修复·code-review（Standards × Spec 双轴）

- 日期：2026-07-21
- 对象：issue #78（实装卡 `chanlun/review-results/issue78-implementation-card-20260721.md`；来源评审 `code-review-v1-v3-64-20260721.md`）
- 裁定边界：`chanlun/escalate/r43-lifecycle-ruling-20260721.md`（#64 SUPERSEDE）
- 方法：只读静态评审（禁 cargo build/test——另一工位独占 crate；禁改源码）。
- **改动面界定限制（重要）**：`nest_lifecycle.rs`（2326 行）与 `v1_level_calib_fix.rs`（705 行）均
  为 **git 未跟踪新文件**（`git status` = `??`，`git log` 无记录），V3 切片与 #78 修复同在一个
  未提交 blob 内——**#78 在 nest_lifecycle.rs 内的改动面无法用 git diff 与修复前基线隔离**。
  本评审对 nest_lifecycle.rs 的核验方式 = 现码结构 + 测试语义锚定（T2/T6/T11/T14/T15），
  凡涉「逐 bit 不变/不退」的声明均按此降级口径标注。`level_view.rs` 为已跟踪文件，
  但其 diff 同时含 #64(d) 实装与 #78 修复 3.2，同属未提交块，亦无法 diff 隔离。

## 一、Standards 轴：修复内容 vs 实装卡

### 修复 1：假反超 ForceCheck 三值化 / ForceUnavailable 注记 —— 已核验，无发现

- 三值枚举 `ForceCheck { Verified(bool), Unavailable(UnavailReason) }`：
  `rust/src/theta_v0/classifier/nest_lifecycle.rs:276-282`；原因码
  `MissingForceSeries`/`CoordinateMapFailed` :286-291。
- 活窗力度现算三值化（缺 hist/dif ⟹ MissingForceSeries；`map_src_to_close_idx` 失败 ⟹
  CoordinateMapFailed；齐备 ⟹ `Verified(segments_diverge_or(...))`，禁第二套力度引擎）：
  :952-972。事件通道恒 `Verified(divergence_confirmed)`（:929-931），与卡「Verified 路径
  逐 bit 不变」语义一致（diff 层面未核，见顶部限制）。
- `Unavailable` 不判 Invalidated：`advance` 第 6 步 match 仅 `Verified(false)` 进
  Invalidated(ForceOvertake)（:716-748）；`Unavailable` 分支只记
  `LifecycleRevisionKind::ForceUnavailable` 审计注记 + `force_unavailable_at` 同 as_of
  幂等去重（:749-770），entry 保持原态、`continue` 不进第 7 步 Confirm。
- 完成时复核仍用本 prefix 现算 `obs.force`，不沿用 first_provable 旧值（:774-779，
  E2E §4.1:151 禁项守住）。
- 数据补齐恢复推进：无残留状态阻塞（`force_unavailable_at` 只作去重基准，:754）。
- T14（:2124-2256）真实夹具核验：**真实 provider 活窗定位**（`pan_real_fixture` +
  `provide_pan_live_windows`，非纯 mock，满足验收线「真实夹具覆盖」）；断言覆盖卡测试
  ①缺数据非终态+注记在案（:2152-2168）②补齐恢复推进至 Confirmed 可消费（:2214-2255）
  ④Unavailable 零 Invalidated（:2182-2187）+ CoordinateMapFailed 子场景（:2189-2212）
  + 同 as_of 注记幂等（:2170-2172）。卡测试③（Verified(false) 仍判 Invalidated）由
  T11（:1924-1998）真实夹具锚定：三通道反超 ⟹ Invalidated(ForceOvertake) + ForceEvidence
  载荷可查账 + 终态留档不可消费。

### 修复 2：as_of 单调守卫 / RetrogradeRejection —— 已核验，无发现（含 1 项 Low 登记）

- entry 记 `last_as_of`（:364）；倒退喂入（`as_of < last_as_of`）显式拒绝：零 revision、
  entry 零改动 + `RetrogradeRejection{key, last_as_of, rejected_as_of}` 审计注记
  （:633-652，注记结构 :503-512，门户 `retrograde_rejections()` :550-554）——禁静默接受。
- 新 entry 本 prefix 创建（`last_as_of = as_of`）恒通过守卫（:616 → :640 不触发），
  语义无冲突。
- 倒退 prefix 的身份消失扫描不触及 `last_as_of > as_of` 的身份（:823-826
  `entry.last_as_of <= as_of` 过滤）——禁借倒退喂入制造 IdentityVanished。
- 同 as_of 幂等：同 as_of 同输入零新 revision（T15 :2315-2319 锚定；机制 = 各写入点
  的 is_none/去重守卫 + 终态吸收）。
- T15（:2265-2325）断言覆盖卡测试①拒绝+注记+entry 不动（:2290-2305）②同 as_of 幂等
  （:2315-2319）③合法前进照常（:2321-2324，T3 语义不退；T3 本体 :1210 存在）。
- `assert_invariants` 增补 `observed_at ≤ last_as_of`（:864-867）。

### 修复 3：声明越界两处 —— 已核验，无发现

- 3.1 模块头：nest_lifecycle.rs:4-8 已改如实措辞（实装面 = 本文件 + mod.rs 注册 +
  #64 授权面内 level_view.rs additive，附裁定书编号；原「唯一实装落点」仅以回顾注记
  形式出现，不再是能力声明）。全仓 grep「唯一实装落点」仅剩该回顾注记一处。
- 3.2 取 (b) 案：level_view.rs:698-704 注释已改能力相符措辞——「迁移前 golden dump
  不可得，不作『与迁移前逐 bit 相等』的无锁定承诺；能力声明 = legacy 为 ext 薄包装
  （`.event` 映射，构造即保证）+ 全量既有测试背书（nest_lifecycle T5 位精确护栏 +
  p92/p95 下游）」。代码侧 legacy 确为 `ext.into_iter().map(|e| e.event)` 薄包装
  （level_view.rs:716-720），且 level_view.rs 测试内含「ext.event 与 legacy 逐 bit 相等」
  断言（diff @@ -1504 块，"additive 铁律"）。声明=能力成立。全仓 grep 无残留旧承诺。

### 修复 4：v1_level_calib_fix.rs exec-1 下溢 —— 已核验，无发现

- `identity_pred_level(exec) -> Option<usize>` 用 `checked_sub(1)`（:128-132）；
  调用点 exec=0 显式跳过 + `V1_TAG ... skip=exec0_invalid` 照实登记（:477-483）+
  注释（exec 自 1 起为 p105 §3 口径）。
- 测试 `t_exec0_identity_pred_no_underflow`（:682-691）：exec=0 ⟹ None 不 panic，
  exec=1/4 正常映射。卡要求满足。

### 授权文件面 —— 已核验，无越面

- 卡授权 = nest_lifecycle.rs / level_view.rs（仅注释/测试）/ v1_level_calib_fix.rs。
  全 diff grep「#78」仅命中 level_view.rs:701（3.2 注释）与 scene-ledger 文档；
  nest.rs/runner.rs 等他处改动无 #78 痕迹（属 #64/#74/#75/#76 各票）。

## 二、Spec 轴：#64 裁定与 r43 裁定书边界

- **first_provable 仅 Verified(true) 分支写**（任务指定不变量）：:685
  `entry.first_provable_at.is_none() && obs.force == ForceCheck::Verified(true)` —— 已核验。
  Unavailable 不写 first_provable（T14 :2166 锚定）。
- **observed_at 不后移**：仅建仓时写一次（:608），无任何改写点 —— 已核验。
- **终态吸收禁复活**：:656-658，桥匹配到终态同吸收 —— 已核验（T3 :1210 锚定）。
- **Invalidated 留档禁删除、反超证据入载荷**：book 无删除 API（唯一 remove 在 :661
  Supersedes 迁移，且仅 Provisional 可达——终态已在 :656 被吸收）；ForceEvidence 同口径
  原语现算入 entry+revision 双载荷（:716-748；IdentityVanished 恒 None :844-845）——
  符合裁定 §2(b)/§4。
- **消费侧 Closed-only**：`consumable_closed` 只放 Confirmed（:570-574）；
  `lineage_nodes` 红线注释在案（:556-565）——符合裁定 §2(a)。
- **judge_at 一个 bit 不动**：level_view.rs diff 中 `judge_at: view.query.as_of` 仅随
  事件构造迁入 ext 函数，赋值表达式逐字不变 —— 已核验。
- **nest.rs:802-810/:987-989 装配红线区不动**：nest.rs diff hunk 止于旧行 ~790，
  之后直到测试块（旧 1693）零触碰，红线区内容未改 —— 已核验（行号随前置 hunk 漂移，
  内容不变）。
- **禁第二查法**：力度判定唯一来源 = `segments_diverge_or`/provider 确认字段；
  ForceEvidence 复用同函数逐通道现算（:308-333），仅审计载荷不进真值路径 —— 已核验。
- **完成但不背驰诚实滞留 Provisional**（从未「曾弱」不伪造 Invalidated）：:810-813 ——
  符合卡 §4.1 反超定义边界。

## 三、发现清单（按严重度）

- **Medium（可核验性，非代码缺陷）**：nest_lifecycle.rs / v1_level_calib_fix.rs 全文件
  git 未跟踪，#78 改动面无法 diff 隔离；「Verified 路径逐 bit 不变」（实装卡修复 1）与
  「T6/T11 逐 bit 不退」（验收线）两项声明 **diff 层面未核**，本评审仅以现码结构 +
  T2(:1141)/T6(:1447)/T11(:1924) 测试语义锚定核验为「语义层已核验」。建议：提交后
  以首个 commit 为基线可追平此项。
- **Low**：`retrograde_rejections` 无去重——同一倒退喂入重复执行将重复追加注记
  （:641 push 无幂等守卫），与 ForceUnavailable 的同 as_of 幂等（:754）不对称。
  实装卡未要求注记幂等（「显式拒绝 + 审计注记」语义上重复注记=重复违规事实，可接受），
  但向量无界增长值得登记。不阻塞。
- **信息项（非发现）**：倒退 prefix 中的**新身份**仍建仓（observed_at = 倒退 as_of，
  :604-619 先于守卫）——per-identity `last_as_of` 语义下新身份无基线可违，与卡
  「book 记录每身份 last_as_of」口径一致；不变量 `observed_at ≤ as_of` 不受损。
- **未核项（本工位禁 cargo，照实登记）**：①全量 `cargo test --release --lib`
  1807 passed / 0 failed 基线；②debug 模式（debug_assert 钟序）全绿；③T14/T15 实际
  运行结果。以上均未经本评审执行核验，以交付方报告为准。

## 四、逐项「已核验无发现」登记（无发现项）

- 修复 1 三值化/注记/恢复推进/不计 Invalidated 统计 —— 已核验无发现。
- 修复 2 倒退显式拒绝/注记/同 as_of 幂等/倒退不制造 IdentityVanished —— 已核验无发现
  （Low 登记见上）。
- 修复 3.1/3.2 声明=能力 —— 已核验无发现。
- 修复 4 下溢守卫+测试 —— 已核验无发现。
- T14/T15 锚定裁定语义（061:26 反超定义、024:24 完成时复核、禁第二查法、真实夹具）——
  已核验无发现。
- 授权文件面/红线区（judge_at、nest.rs 装配输入、consumable_closed）—— 已核验无发现。

## 五、判定

**带边界的通过**。边界两条：①Medium 可核验性限制——#78 改动面无法 diff 隔离，
「逐 bit 不变/不退」为语义锚核验而非 diff 核验；②cargo 测试基线与 debug 模式
本工位未核（禁跑），需由可执行工位以 1807/0 绿 + debug 全绿收口。代码本体四项修复
与全部裁定边界静态核验通过，无 Critical/High 发现。

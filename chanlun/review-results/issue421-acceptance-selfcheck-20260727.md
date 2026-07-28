# #421 活假设账本接生产：验收自查（2026-07-27 裁定续跑，2026-07-28 收口）

## 0. 结论与口径

**总判定：部分满足，不能按 090 宣称完整验收。**

- 生产接线、触发节拍、双通道、倒退拒绝、完成后停延展、力度跃变、release 不变量调用、
  #428 显式审计、#427 契约/断言/变异实测与既有输出逐字节护栏均已有证据。
- #426 的字面硬项「两类否证终局在真实数据上均可触发并被记录」只满足
  `NeverConstituted`；100k 生产回放中 `ForceOvertake=0`。F4 确定性测试可触发两类，
  但不能冒充真实数据验收。
- 「完成信号已到但力度不可验」已可显式查账；实测为零不构成规格豁免，其终局归属仍待
  另票裁定。

状态只使用：**满足 / 部分 / 未满足 / 未能判定**。`/tmp` 证据是本工位实测产物；源文件
行号以本次收口工作树为准。

## 1. 基线、TDD 与测试门

| 项目 | 状态 | 证据 |
|---|---|---|
| 开工 debug 基线 | 满足 | `/tmp/wt421-baseline-debug.log`：1988 passed / 1 failed / 135 ignored；唯一红为 #491 `extract_signals_bit_exact_digest_guard` |
| 开工 release 基线 | 满足 | `/tmp/wt421-baseline-release.log`：1988 passed / 1 failed / 135 ignored；唯一红为 #491 |
| #428 F9 红 | 满足 | `/tmp/wt421-f9-red.log`：缺 `CompletionForceUnavailableAudit` / accessor，先红后实现 |
| #428 F9 绿 | 满足 | `/tmp/wt421-f9-green.log`；测试 `f9_completion_force_unavailable_is_explicit_audit` |
| 生产接线黑盒红/绿 | 满足 | pre 二进制运行成功但不产生 lifecycle 文件：`/tmp/wt421-red-*`；post 产生 `/tmp/wt421-post-clean-lifecycle.dump` |
| 左端恒等变异红 | 满足 | 将确认臂左端临时篡改为 `pair.seg_c.0 + 1` 后，release 断言以 `left=121 / right=120` 失败：`/tmp/wt421-left-anchor-mutant-red.log` |
| 左端恒等恢复绿 | 满足 | `/tmp/wt421-left-anchor-restored-green.log`、`/tmp/wt421-left-anchor-postmerge-green.log` |
| 主接缝常规时限 | 满足 | `/tmp/wt421-nest-lifecycle-green.log`：24 passed，测试执行 0.01s |
| 实现提交前 debug 门 | 满足 | `/tmp/wt421-code-debug.log`：1990 passed / 1 failed / 135 ignored；唯一红 #491，零新增红 |
| 实现提交前 release 门 | 满足 | `/tmp/wt421-code-release.log`：1990 passed / 1 failed / 135 ignored；唯一红 #491，零新增红 |
| 收口提交前 debug / release 门 | 满足 | `/tmp/wt421-final-{debug,release}.log`：两档均 1990 passed / 1 failed / 135 ignored；唯一红 #491，零新增红 |

说明：票面允许的 #446 三条 lee 基线在本工位开工时已被并发工作线修绿，因此本工位实测
失败集合只有 #491；没有把过期的四红名单照抄成实测。

## 2. #426 转呈验收清单逐条自查

| #426 条款 | 状态 | 证据锚 |
|---|---|---|
| 回放循环新增账本喂数出口，只在重估 trigger 喂 | 满足 | `rust/src/bin/p123_fast_replay.rs:869-883`；出口 `rust/src/theta_v0/classifier/nest_lifecycle.rs:1097`；20k/100k 实跑 summary |
| 时点倒退显式拒绝、账本零改动、拒绝可查 | 满足 | `f5_retrograde_prefix_rejected_with_zero_book_change`（nest_lifecycle.rs:2732）；桥匹配分支专测 `t18_retrograde_rejected_on_bridge_match_branch`（:2147） |
| 通道切换置结构完成 | 满足 | `f2_channel_switch_sets_structure_completed_and_stops_extension`（:2491）；生产 feed 在 p123_fast_replay.rs:1237-1318 |
| 完成后不再追加延展记录，红绿可验 | 满足 | F2 的完成前/完成/完成后修订增量；`f7_terminal_skip_equals_feeding_terminal_identity`（:2820）证明跳过与照喂逐位等价 |
| 通道切换处力度跃变专属覆盖 | 满足 | `f3_channel_switch_force_jump_is_visible_both_ways`（:2589），正反两臂均走不同通道求值 |
| 两类否证终局在真实数据上都可触发 | **部分** | F4（:2668）确定性触发两码；但 `/tmp/wt421-post-100k-lifecycle.dump` 实测 `NeverConstituted=92`、`ForceOvertake=0`，故真实数据字面项未满足 |
| 订单流逐字节护栏 | 满足 | p123 stdout/P116 面 20k、100k `cmp=0`；m8 三窗 trades 对拍见 §5 |
| 挂/不挂账本时数据源事件流逐字节相等 | 满足 | `f6_feed_exit_is_sidecar_events_byte_identical`（:2779）；p123 replay dump 20k、100k `cmp=0`；m8 tower_events 对拍见 §5 |
| 主接缝不进 slow 档 | 满足 | 24 个 nest_lifecycle 测试 0.01s：`/tmp/wt421-nest-lifecycle-green.log` |
| 取数复制实现，不提升既有私有函数可见性 | 满足 | 私有 `leg_as_segment_copy`（nest_lifecycle.rs:1032）；p123 只持有自足 `LifecycleRunOwned`（p123_fast_replay.rs:440），未改上游可见性 |
| 活假设不进装配候选/证书真值路径 | 满足 | 模块消费边界 nest_lifecycle.rs:540-547；p123 sidecar 只写独立 dump（p123_fast_replay.rs:20、836），既有输出对拍为零差异 |
| 全库零新增失败 | 满足 | §1 debug/release 双档；唯一红均为基线 #491 |
| #428：完成且力度不可验须显式记录、可查账 | 满足 | `CompletionForceUnavailableAudit`（nest_lifecycle.rs:528-533）、写入点 :728-744、生产 dump :1306-1318、F9 |
| #428：显式报告缺口并提供发生率，不自裁终局 | 满足 | 20k `0/3715=0%`，100k `0/119988=0%`；p123_fast_replay.rs:521-537；本报告 §0/§6 明确保留终局裁定缺口 |
| `StructureCompleted` / `Invalidated` 公共文档订正 | 满足 | nest_lifecycle.rs:162-165、:227-246；三原因码完整，Unavailable 不再被描述成可达的 StructureCompleted 滞留 |
| release 核验措辞降级或显式调用 | 满足 | 生产 trigger 后无条件调用 `lifecycle_book.assert_invariants()`：p123_fast_replay.rs:882 |
| 喂入来自产窗侧，不绕过创新高预滤 | 满足 | p123_fast_replay.rs:1175-1223 复用 tower→projection→decompose→assemble_level_view→`provide_nest_candidate_events`；活窗走 `provide_pan_live_windows`（nest_lifecycle.rs:972、1111-1129） |
| `CONTEXT.md` 工程口径注记只登记、不执行 | **部分** | 本工位已在 §6 登记编排动作且未越权改 `CONTEXT.md`；主仓注记本身仍待编排者执行 |
| #450 结论后的数字复核 | **未能判定** | 本工位未执行 #450 数据解读复核；本票数字不得据此外推，见 §6 |

## 3. #427 缺口清单逐条自查

| #427 条款 / 最新留言缺口 | 状态 | 证据锚 |
|---|---|---|
| 模块头改为「时钟精度 = trigger 粒度」，说明不可早记与理论上限 | 满足 | nest_lifecycle.rs:48-58；非 trigger 首次可证只允许晚记到下一 trigger |
| 重建报告旧/新口径收为一个现行口径 | 满足 | `chanlun/review-results/v3-lifecycle-rebuild-impl-20260724.md:82-96,99-119`；历史发现显式标为已由 #421 补观测面 |
| 左端恒等断言接线 | 满足 | level_view.rs:790-799；非 debug 门控 |
| 左端恒等红绿双向变异实测 | 满足 | `/tmp/wt421-left-anchor-mutant-red.log` 与 `/tmp/wt421-left-anchor-postmerge-green.log`；测试 `trend_confirm_truncation_keeps_seg_c_left_anchor`（level_view.rs:1653-1785）覆盖确认/未确认两臂并证明右端确实变化 |
| 通道切换力度跃变注记 | 满足 | nest_lifecycle.rs:2579-2588 的 F3 注记与双臂测试 |
| #64 §2(d) 两字段授权核销 | 满足 | 重建报告 :110-116：`seg_c_full` 因左端等价不复活；`c_start_live` 从未新增，左端由产窗定位原语派生 |
| 订单流与账本产出逐字节不变 | 满足 | §5 的 p123 + m8 pre/post 对拍；新增 lifecycle dump 是独立新输出，未伪称与 pre 相等 |
| #426 已真正接生产调用方 | 满足 | p123_fast_replay.rs:869-883、1237-1318；不再只有模块内测试调用 |
| 全库零新增失败 | 满足 | §1 双档读数 |

## 4. SPEC #421 自身主接缝与边界

| SPEC 项 | 状态 | 证据锚 |
|---|---|---|
| 喂数正确性 | 满足 | F1/F5 + p123 生产 trigger 接线 |
| 通道切换与完成后停止迁移 | 满足 | F2/F7 |
| 两类原因码正确、终态留档不删 | 部分 | F4/T17/T6 在状态机层满足；真实数据仅观测到 NeverConstituted，未满足 #426 的加强字面项 |
| 力度跃变可见 | 满足 | F3 |
| 桥匹配分支倒退拒绝专测 | 满足 | `t18_retrograde_rejected_on_bridge_match_branch` |
| 交易行为零变化 | 满足 | §5 既有输出面逐字节对拍 |
| 不接同族另外两个 replay bin | 满足 | 依 SPEC Out of Scope，仅接 p123 |
| 不做趋势域真实反超复核 | 满足 | 依 SPEC Out of Scope，未夹带 |
| 不做桥匹配扫描性能量化 | 满足 | 依 SPEC Out of Scope，未夹带 |
| #454 文件拆分 | 满足 | 明确不在范围；本工位未为拆分移动任何一行 |
| #429/#430 两轴新上下文评审 | 未满足 | 属编排者另行派发；本工位只准备代码与本自查，不自评替代 |

## 5. pre/post 逐字节对拍

pre 固定在 `a12a1022d9ddd8d1cae867a107a3a33c359358cf`，post 固定在
`6e15ceffeeb8259c065bf7c0ec9ec7c65935737c`，均从独立 worktree/target 构建。

| 面 | 规模/窗口 | cmp | SHA-256（pre = post） | 产物 |
|---|---|---:|---|---|
| p123 replay/P116 dump | 20k | 0 | `d811e9e595f398c6c3b3729cee0daf621d67d14a81325bd597f822581a95d491` | `/tmp/wt421-{pre,post}-clean-replay.dump` |
| p123 stdout/订单可见面 | 20k | 0 | `234738595da9b48528dade4a40cb4a0132b61990687b819fee129089be8be6d4` | `/tmp/wt421-{pre,post}-clean-p123.stdout` |
| p123 replay/P116 dump | 100k | 0 | `dc5633105c2e0947067da50df574f4296bc5dacf029080a3c9a046c077c45de4` | `/tmp/wt421-{pre,post}-100k-replay.dump` |
| p123 stdout/订单可见面 | 100k | 0 | `db4edfa420f946cbd80883ca9fc4cefcae221e38b48e06f53c31801a2c55aefe` | `/tmp/wt421-{pre,post}-100k-p123.stdout` |
| m8 trades | p3fold | 0 | `e0573a72a797db0e6f5e70293058ad61880d537533bb4317647396ca9c013196` | `/tmp/wt421-{pre,post}-m8-p3fold/trades.jsonl` |
| m8 tower_events | p3fold | 0 | `83f45a36ab422a92d198d5d9289c40db94e95fe2cba7af2322e2d37aa8e718c2` | `/tmp/wt421-{pre,post}-m8-p3fold/tower_events.jsonl` |
| m8 trades | wf7 | 0 | `dd8459fd29dc11044f70541bfd47125fed7cb33e93091b2e48ac549712b16cfe` | `/tmp/wt421-{pre,post}-m8-wf7/trades.jsonl` |
| m8 tower_events | wf7 | 0 | `aa96b3e836be5a43514c425d572c5312d15d8c6950d991d094416e4d61af1cbb` | `/tmp/wt421-{pre,post}-m8-wf7/tower_events.jsonl` |
| m8 trades | wf8 | 0 | `0a744e228ce37640f3681c3f4dc494618f0c475c8bd3f1f0092c91d5d6455b87` | `/tmp/wt421-{pre,post}-m8-wf8/trades.jsonl` |
| m8 tower_events | wf8 | 0 | `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` | `/tmp/wt421-{pre,post}-m8-wf8/tower_events.jsonl` |

不能逐字节相同、且必须逐面归因的新输出：

1. `P421_LIFECYCLE_DUMP`：pre 不存在，post 为本票新增的独立审计面。20k 文件
   `/tmp/wt421-post-clean-lifecycle.dump`，SHA
   `15da4bb66d4b9b2c6408c6b4899b0c09b33abacb2b7b4f1fbb9893cb556fa478`；
   100k 文件 `/tmp/wt421-post-100k-lifecycle.dump`，SHA
   `721efec80cf3de0b79ae06c0a3b5d7c94a9927eb5ff779f31d7af117d4f113cc`。
2. stderr 新增 `P421_LIFECYCLE_SUMMARY`。它是侧车审计读面，不是订单流或既有账本产物；
   20k 为 `520/2479/3715/2479/0`，100k 为 `1135/77825/119988/77825/0`
   （依次为 triggers/live/completion/switch/unavailable）。

## 6. 未完成与编排者执行项

1. **#426 真实数据双终局：部分。** 100k 生产回放中 `NeverConstituted=92`、
   `ForceOvertake=0`、Confirmed=121；且 `live_windows=channel_switches=77825`，
   `extension_suppressed=0`，样本里没有跨 trigger 延展后再反超的生产身份。F4 只证明状态机
   和喂数出口可表达两码，不证明生产数据上两码都已发生。#409 的 92.6% 来自
   `feed=every_prefix / structure_completed=false / pan_live_only` 探针，不能代替当前生产
   双通道验收；须由后续票核定完成事件匹配与活窗生命周期，不能在本票擅造完成判据。
2. **#428 终局归属待裁。** 完成且力度不可验当前显式留
   `CompletionForceUnavailableAudit` 并保持 Provisional；补第四原因码、显式挂起桶、接受
   永久滞留三选一仍须另票裁定。本轮 20k/100k 发生率均为 0，不构成豁免。
3. **`CONTEXT.md` 只登记未执行。** 请编排者在主仓「背驰否证 / 从未构成」处补
   「本模块工程判据是力度谓词从未可证，创新高预滤在产窗侧」注记。
4. **#450 未复核。** 本报告的按级别数字尚未按 #450 结论回看，不得外推。
5. **#429/#430 待编排。** 两轴 `/code-review` 必须由新上下文执行，本自查不替代。
6. **#454 未触碰。** `nest_lifecycle.rs` 拆分另票处理。

## 7. 返工节（2026-07-28）

本节是 #429/#430 双影子 FAIL 后的返工增量；上文保留为首轮自查历史。凡读数或结论冲突，
以本节为准。pre 为本工位开工 HEAD `ce074711d5` 的二进制；重验期间主线仅并发前进了
#514 的两份 `scripts/` 文件，未触碰本票 Rust 文件。

### 7.1 P-H1 / P-H2 / P-H3 / T8 逐条判定

| 项目 | 状态 | 证据锚 |
|---|---|---|
| P-H1 身份跨 prefix 存活 | **满足** | `f9a_same_trigger_completion_does_not_collapse_lifetime_to_zero` 先红 `/tmp/wt421r-ph1-red.log`、后绿 `/tmp/wt421r-ph1-green.log`；`feed_replay_prefix` 只有命中 book 中先前 Provisional 身份才切通道，同 trigger 首见碰撞只建活身份 |
| P-H1 100k 生产寿命 | **满足** | `/tmp/wt421r-post-100k-lifecycle.dump`：Observed=248；有 StructureCompleted 的 229/229 全为正寿命，0 寿命=0；min/median/max=5/120/411 trigger，分桶 `1..99=100`、`100..499=129` |
| P-H1 终局与关系行 | **满足** | 100k：Confirmed=140、NeverConstituted=89、ForceOvertake=1、IdentityVanished=18；summary 为 `live_windows=89576 / completion_signals=230 / channel_switches=230 / extension_suppressed=89098`，不再是 `live_windows==channel_switches` |
| P-H2 生产接缝可达 | **满足** | F9 已改为只经 `feed_replay_prefix` 构造「先建活身份→完成信号+ForceMaterial::unavailable」；红 `/tmp/wt421r-ph2-red.log`，最终 25/25 lifecycle 绿 `/tmp/wt421r-nest-after-reapply.log`；audit 可查且仍不擅裁新终态 |
| P-H2 唯一分母 | **满足** | `ReplayFeedStats::completion_signals` 按桥身份唯一；跨 trigger 重发的分子/分母均不重复。100k 生产发生率为 `completion_force_unavailable=0 / completion_signals=230 = 0%`，原始重复观察 `completion_events=125453` 仅保留诊断，不再作分母 |
| P-H3 共享 dirty/cache | **满足** | sidecar 直接借用原 `LevelDerived`/`RunEntry` 的 centers/kinds/events；无第二个 map/cache。P-H3 编译红 `/tmp/wt421r-ph3-red.log`，debug/release 绿 `/tmp/wt421r-ph3-after-reapply.log`、`/tmp/wt421r-ph3-release.log` |
| P-H3 100k 复用读数 | **满足** | `provider_requests=2099 / provider_reevals=87 / provider_reuses=2012`，复用率 95.855%，相对旧侧车每 request 全算的实际补算下降 95.855%；20k shadow 为 179 checks / 0 mismatch |
| T8 契约矛盾 | **满足** | T8 注释已与模块头/feed 契约统一：非 trigger prefix 只会晚记；`observed_at` 与 `first_provable_at` 同次首次写入，不能构造 `first_provable_at < observed_at` |
| S-H1 账本不可变 | **不在修复面** | 编排者已以 `5022e31df5` 在 `.claude/rules/common/coding-style.md` 明文裁定例外；本返工未把它伪装成代码修复 |

谱系依据补录：生命周期构建/消费边界依
`chanlun/escalate/r43-lifecycle-ruling-20260721.md`；共享驻留面及
`LevelDerived`/`RunEntry` 归属依
`chanlun/escalate/r1-r3-ruling-confirmation-checklist-20260720.md`。本返工没有另造第二套
ConfirmCursor/PanMemo 缓存定义。

### 7.2 红绿与测试门

- P-H1：`/tmp/wt421r-ph1-{red,green}.log`；红为首次碰撞错误计 switch，绿后观察钟
  `99 < 109` 完成钟。
- P-H2：`/tmp/wt421r-ph2-{red,green}.log`；红为生产 feed 无 audit，绿后 F9 经主接缝可达；
  最终测试另锁跨 trigger 重发不放大发生率。
- P-H3：`/tmp/wt421r-ph3-red.log` 缺共享 payload/helper 编译红；
  `/tmp/wt421r-ph3-after-reapply.log` 与 `/tmp/wt421r-ph3-release.log` 绿。
- `cargo test --lib`：`1992 passed / 1 failed / 135 ignored`；
  `cargo test --release --lib` 同数。唯一失败均为在册 #491
  `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`，见
  `/tmp/wt421r-cargo-test-{lib,release-lib}.log`；零新增失败。

### 7.3 字节护栏与 lifecycle characterization

| 面 | cmp | SHA-256（pre = post） |
|---|---:|---|
| p123 20k stdout | 0 | `bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6c` |
| p123 20k P116 dump | 0 | `fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a40` |
| p123 100k stdout | 0 | `d8b69c180c23c5e393bf3c330825d88ae9c989ff38f2b8865e049e1b5eb56da8` |
| p123 100k P116 dump | 0 | `8a7327feb3b9ba29f69fa824af1f1ecc137d9303637b47ad8465b6d638705f84` |
| m8 p3fold trades / tower_events | 0 / 0 | `65621e3505253a7fcde046a9eaa78bbc8c19798fe84ed626c88b56d6cb0cc21b` / `83f45a36ab422a92d198d5d9289c40db94e95fe2cba7af2322e2d37aa8e718c2` |
| m8 wf7 trades / tower_events | 0 / 0 | `db6e14fab9f2e5767f38c832aead1efcf6c06b0831181e6c7c7cdaa3deaffbfb` / `aa96b3e836be5a43514c425d572c5312d15d8c6950d991d094416e4d61af1cbb` |
| m8 wf8 trades / tower_events | 0 / 0 | `abf8245ffd880c0f4488cd02f8696e950cd8349de15c705b52a2add3d679eaf8` / `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` |

lifecycle 是修复本体，不作 `cmp=0`：100k pre 为 Observed/StructureCompleted
`248/248`、正寿命 `0/248`、Confirmed/NeverConstituted `151/97`；
post 为 Observed/StructureCompleted `248/229`、正寿命 `229/229`、
Confirmed/NeverConstituted/ForceOvertake/IdentityVanished `140/89/1/18`。
post lifecycle SHA-256：20k
`6a8877ee6549d50204407cf06dd466bd68874e4bf26b46d35468f699b4b80a52`，
100k `d45201143c3d2171216849d5acdcfefbb2195c6a889d9f853282dcbde94c6c0e`。

### 7.4 各 MED / LOW 去向（逐项）

- #429 S-M1（文件/函数上限）：仍归 #454 拆分债；本票不触碰 #454。
- #429 S-M2（自查缺谱系引用）：本节已补 `r43-lifecycle-ruling` 与
  `r1-r3-ruling-confirmation-checklist`，本项满足。
- #429 S-L1（F6 名称强于单测断言）：保留 LOW；本节只把 F6 当字段/Debug 对照，
  真实字节结论只取 §7.3 的生产 pre/post `cmp=0`。仓内 integration 字节门由编排者另派。
- #429 S-L2（`leg_as_segment_copy` 重复）：按 #421/#426 明示授权保留，不提升私有函数可见性。
- #430 Spec-M1（Unavailable 生产不可达）：已由 P-H2 主接缝 F9 修复并满足。
- #430 Spec-L1（T8 反向文字）：已订正并满足。
- #430 Standards-M1（超 50 行/Data Clumps）：仍归 #454；本返工只移除旧的
  `collect_lifecycle_runs` 全量重算函数，不借返工扩大拆分范围。
- #430 Standards-M2（声明/实现未收口）：T8 已与模块头/feed 契约统一，本项满足。
- #430 Standards-M3（缺仓内 p123 integration/E2E 字节门）：**部分**；新增 P-H3 bin
  单测与原 shadow 对拍，但完整字节证明仍是外部产物，留待编排者派专门回归门票。
- #430 Standards-L1（授权型 Duplicated Code）：同 #429 S-L2，保留登记、不越权改可见性。
- #430 未能判定项（#450 后按级别复核）：仍未能判定；本返工不据总量外推按级别结论。

#429/#430 保持 OPEN；本节只提交修复与证据，复审由编排者派新上下文执行。

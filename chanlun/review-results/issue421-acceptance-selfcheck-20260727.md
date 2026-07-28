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

## 8. 裁定执行节（2026-07-28）

本节执行 #421 最末条「裁定：完成定义与结算时序」。上文 §7 是裁定前返工历史；凡与本节
冲突，以本节为准。**总判定仍为部分满足，不能按 090 宣称通过验收神谕。**

### 8.1 Q1 / Q2 / Q3 逐条判定

| 裁定项 | 状态 | 执行结果与证据 |
|---|---|---|
| Q1 身份开窗即入账、每 trigger 两路喂 | **满足** | `feed_replay_prefix` 先组装全部可见 `PanLive`，再追加完成相；同 trigger 不再用完成事件替换活窗。真实首完成另存 `CompletionSignal`，不再用终态冒充完成事实，也不再要求「先前仍为 Provisional」。BTC 100k：`Observed=248`、`COMPLETION_SIGNAL=248`、`completion_signals=248`，首完成零丢弃；`IdentityVanished=0`、`retrograde_rejected=0`。后到提前终局身份的完成信号只入独立分母，终态吸收禁止回填 `StructureCompleted`。F9a/F9b/F9c 锁定同 trigger、提前终局、completion-only 三条路径。 |
| Q2 闪现窗零寿命照实记、统计分层 | **满足（记录语义）；神谕未满足** | 同 trigger 链照实为 `Observed@t → (FirstProvable@t) → StructureCompleted@t → 终局@t`；`LifecycleSettlementStats` 与 p123 的 `LIFETIME` / `P421_LIFETIME_SUMMARY` 分开报告 `flash_terminal` 和非闪现寿命。20k 为 `46/46` 闪现，100k 为 `248/248` 闪现，非闪现 `count=0`。没有为制造正寿命而延迟完成或回填历史。闪现率 100% 已高于裁定所述“三四成”逃生门，须带本节数据另行决定是否启用独立逐 bar 喂数循环；本票不擅自越过 US15/稀疏 trigger 裁定。 |
| Q3 Provisional 可直接 ForceOvertake | **部分** | 模块头、`NestEventState::Invalidated`、`LifecycleRevisionKind::StructureCompleted`、`advance` 第 6/7 步及 T7/T8 均登记「曾可证后反超可直接终局、无需 StructureCompleted」。F3、T7、F9b 均证明该链可执行，`assert_invariants` 也明确允许 `structure_end_at=None`。但 BTC 100k 双通道生产账本因 248 个身份均在首次可见 trigger 已收到真实完成信号而 `ForceOvertake=0`，没有可交付的生产 dump 实例链；故只判部分。 |

### 8.2 TDD 与测试门

- RED：`/tmp/wt421q-red.log`，新增测试先因缺
  `NestLifecycleBook::completion_signals` / `settlement_stats` 编译失败。
- GREEN：`/tmp/wt421q-nest-lifecycle-green.log`，27 passed / 0 failed。
- `cargo test --lib`：`/tmp/wt421q-cargo-test-lib.log`，
  1994 passed / 1 failed / 135 ignored。
- `cargo test --release --lib`：`/tmp/wt421q-cargo-test-release-lib.log`，
  1994 passed / 1 failed / 135 ignored。
- 两档唯一失败均为在册 #491
  `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`；本次零新增失败。

### 8.3 五项验收读数

| 验收项 | 状态 | 实测 |
|---|---|---|
| 1. debug / release 测试门 | **满足** | 两档均 1994/1/135，失败集合仅 #491；新增 lifecycle 测试全绿。 |
| 2. 字节护栏 | **满足** | p123 20k/100k stdout 与 P116 dump 均 `cmp=0`；m8 的 p3fold/wf7/wf8 两面（trades、tower_events）六对均 `cmp=0`。P-H3 100k 仍为 provider `2099/87/2012`（request/reeval/reuse），复用率 95.855169%。 |
| 3. 同 BTC 100k 探针神谕 | **未满足** | p409 同码同前缀：248 身份、124 曾可证、113 ForceOvertake，反超率 91.129032%，Force 寿命 min/median/max=`9/378/20966` 根，Unavailable=0。双通道账本：248 身份、151 曾可证、ForceOvertake=0、Confirmed/Never=`151/97`、闪现 248、非闪现 0、Unavailable=0。身份总数与 Unavailable 一致，但终局分布和非闪现寿命不一致。 |
| 4. 结算时序证明 | **部分** | 100k dump 中 `IdentityVanished=0`；独立 `COMPLETION_SIGNAL=248` 等于全部真实首完成；全局 `as_of` 单调、无倒退拒绝。因生产 `ForceOvertake=0`，未能给出要求的生产 `Observed→(FirstProvable)→Invalidated{ForceOvertake}` 实例链；仅测试与 p409 有该链。 |
| 5. 自查档裁定执行节 | **满足** | 本节逐条登记 Q1/Q2/Q3、五项门、探针对照与未满足项。 |

### 8.4 同前缀探针对照

| BTC 100k 指标 | p409（pan-live-only，逐 prefix） | p123 双通道账本（稀疏 trigger） |
|---|---:|---:|
| 身份 / entries | 248 | 248 |
| 曾可证 | 124 | 151 |
| ForceOvertake | 113 | 0 |
| 反超率（Force / 曾可证） | 91.129032% | 0% |
| Confirmed / NeverConstituted | 0 / 0 | 151 / 97 |
| IdentityVanished | 20 | 0 |
| 闪现终局 | 不适用（无完成通道） | 248（100%） |
| 非闪现寿命 | Force `9/378/20966` | `count=0` |
| Unavailable | 0 | 0 |

证据：

- p409：`/tmp/wt421q-p409-btc100k.log`、
  `/tmp/wt421q-p409-btc100k-entries.jsonl`、
  `/tmp/wt421q-p409-btc100k-force-lifetimes.txt`。
- p123：`/tmp/wt421q-post-{20k,100k}-p123.{stdout,stderr}`、
  `/tmp/wt421q-post-{20k,100k}-{replay,lifecycle}.dump`。

两侧身份数同为 248，排除「双通道漏窗」解释。差异来自完成定义：p409 按既有探针契约
`structure_completed=false` 继续延展；p123 按 ADR-0003 与本次 Q1/Q2，在完成事件首次可见
trigger 当场结算。若忽略首完成、等下一 trigger 重发，可人为制造正寿命，但这正是
#429 复审判定的返工病，本节禁止复活。

### 8.5 lifecycle 修正本体前后

BTC 100k 裁定前（f663 返工）：

- `Observed/StructureCompleted=248/229`；
- `Confirmed/Never/Force/IdentityVanished=140/89/1/18`；
- `completion_signals=230`，漏 18 个真实首完成。

裁定后：

- `Observed/StructureCompleted/COMPLETION_SIGNAL=248/248/248`；
- `Confirmed/Never/Force/IdentityVanished=151/97/0/0`；
- `flash_terminal/nonflash_count=248/0`；
- lifecycle SHA-256：20k
  `7a927e9b23d6ed2e0ba06ba7eed8cc96fbda4574c77f56836a06394458bfdda8`，
  100k `7ce0b1b39c4837657525696f695d1acbe1aec31414be8dc8e1392c6f31b8b5d9`。

### 8.6 非 lifecycle 字节护栏

| 面 | cmp | SHA-256（pre = post） |
|---|---:|---|
| p123 20k stdout | 0 | `bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6c` |
| p123 20k P116 dump | 0 | `fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a40` |
| p123 100k stdout | 0 | `d8b69c180c23c5e393bf3c330825d88ae9c989ff38f2b8865e049e1b5eb56da8` |
| p123 100k P116 dump | 0 | `8a7327feb3b9ba29f69fa824af1f1ecc137d9303637b47ad8465b6d638705f84` |
| m8 p3fold trades / tower_events | 0 / 0 | `2da686833581d5358a1806aa41ad7ba16371bc3db190fe2a8ccfe62b9cb46627` / `83f45a36ab422a92d198d5d9289c40db94e95fe2cba7af2322e2d37aa8e718c2` |
| m8 wf7 trades / tower_events | 0 / 0 | `3371f1e62153e8aa216ae6f28530bcaa95423a1e3f9b5341e647ec1a78a3fca8` / `aa96b3e836be5a43514c425d572c5312d15d8c6950d991d094416e4d61af1cbb` |
| m8 wf8 trades / tower_events | 0 / 0 | `006c31f54cd72d8ec9c9461122068faf58377cad2d176f839f6a6e0c5ff601b7` / `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` |

### 8.7 遗留

1. 验收神谕与当前完成定义在 BTC 100k 上不同时成立：按真实首完成当场结算得到 100% 闪现；
   忽略首完成则违反 Q1/Q2。须由编排者按裁定中的逃生门决定是否另开「独立逐 bar 喂数循环」
   带数据重裁；在此之前本票只能部分满足。
2. 生产双通道没有 ForceOvertake 实例，Q3 仅在公共契约、状态机与测试层闭合。
3. #454/#497 标准债、本票外的 integration 字节门与 map #59 Decisions 均未触碰。

## 9. 逃生门执行节（2026-07-28）

本节执行 #421 comment-5103071677 的「独立逐 bar 喂数循环」授权。实现严格保持：

- 生产 `trigger=(forest_epoch, signal_signature)`、候选事件流、YieldBook、装配、证书与订单流
  不动；
- sidecar 活窗发现复制 p409 的逐级逐 run 路：`forest_epoch` 变化时重算结构窗，其他 bar
  保持身份分量、只把 `seg_c_live.1` 延到当前 bar；
- 每根 bar 经独立出口先喂活窗、后喂当 bar 首次可见完成事件；无回填、无延迟完成；
- 完成事件仍按 #415 / ADR-0003 的「数据源由 PanLive 切入 Event 通道即结构完成」口径
  当场结算，不用下一 trigger 重发制造寿命。

### 9.1 RED / GREEN 与测试门

- RED：`/tmp/wt421-escape-red.log`。F10a/F10b/F10c 先因
  `ReplayBarFeed` / `feed_replay_bar` 尚不存在而编译失败。
- GREEN（新增行为）：`/tmp/wt421-escape-f10-green-final.log`，3 passed / 0 failed：
  - F10a：逐 bar 两相顺序 + 同 bar 闪现零寿命；
  - F10b：跨 bar `Observed→FirstProvable→ForceOvertake`，修订 `as_of` 单调；
  - F10c：completion-only 窗先合成当 bar 观察、首完成零丢弃。
- lifecycle 模块：`/tmp/wt421-escape-nest-lifecycle-green-final.log`，
  30 passed / 0 failed。
- p123 bin 接缝：`/tmp/wt421-escape-p123-bin-green-final.log`，
  1 passed / 0 failed。
- 全库 debug：`/tmp/wt421-escape-cargo-test-lib-final.log`，
  1997 passed / 1 failed / 135 ignored。
- 全库 release：`/tmp/wt421-escape-cargo-test-release-lib-final.log`，
  1997 passed / 1 failed / 135 ignored。
- 两档唯一失败均为在册 #491
  `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`；
  本轮零新增失败。

### 9.2 五项验收逐条判定

**认识论等级**：项 1 的确定性测试为 **L1**；项 2–4 的 BTC/p123/m8 真实回放实测为
**L2**；项 5 的文档登记为 **L0**。本节没有 L3 交叉验证，现行 E 改判见 §10。

| 验收项 | 状态 | 本轮实测 |
|---|---|---|
| 1. 测试门 | **满足** | F10 三条新增行为、lifecycle 30/30、p123 bin 1/1 全绿；全库 debug/release 均为 1997/1/135，唯一失败 #491。 |
| 2. 字节护栏 | **满足** | p123 20k/100k stdout + P116 dump 四对 `cmp=0`；m8 三窗 trades+tower_events 六对 `cmp=0`；P-H3=`2099/87/2012`、复用率 95.855169%。lifecycle dump 按逐 bar feed 本体发生变化，见 §9.4。 |
| 3. p409 正面神谕 | **未满足** | p409 BTC 100k 仍为 248 身份、124 曾可证、113 Force、反超率 91.129032%、Force 寿命 `9/378/20966`；逐 bar 双通道账本仍为 248/248 闪现、Force=0、非闪现 count=0。Unavailable 两侧均 0。禁止靠忽略/延迟首完成凑数。 |
| 4. 结算时序 | **部分满足** | 100k dump 有 100000 条连续 `FEED`（0..99999、gap=0）；1143 条修订 `as_of` 回退=0、跨 bar 错配=0；`COMPLETION_SIGNAL=248` 全部与当 bar 对齐，IdentityVanished=0、retrograde_rejected=0。但生产 Force=0，故不存在可交付的生产 `Observed→FirstProvable→Invalidated{ForceOvertake}` + ForceEvidence 实例。 |
| 5. 自查档 | **满足** | 本节登记实现、五项门、字节面、探针对照、直接反证与失败归因。 |

### 9.3 trigger 粒度 → 独立逐 bar 前后

**认识论等级：L2**（BTC 单标的、固定 20k/100k 前缀真实回放）；只可否证本票在该
provider/前缀上的神谕，不构成 L3 跨标的、跨时段外推。

BTC 100k，同一数据、同一完成定义：

| 指标 | trigger 粒度（26771ee0d8） | 独立逐 bar |
|---|---:|---:|
| 生命周期时钟 / provider trigger | 1206 trigger | 100000 bar / 1206 trigger |
| 活窗物理观察 | 89576 | 11026776 |
| entries / 首完成信号 | 248 / 248 | 248 / 248 |
| 曾可证 | 151 | 151 |
| Confirmed / NeverConstituted | 151 / 97 | 151 / 97 |
| ForceOvertake / IdentityVanished | 0 / 0 | 0 / 0 |
| 闪现终局 / 闪现率 | 248 / 100% | 248 / 100% |
| 非闪现寿命 | count=0 | count=0 |
| Unavailable / retrograde | 0 / 0 | 0 / 0 |

20k 同判：逐 bar读数为 `bars=20000`、`provider_triggers=565`、
`live_windows=316914`、`completion_signals=46`；终局 `Confirmed/Never=35/11`、
`Force/IdentityVanished=0/0`、闪现 `46/46`、非闪现 count=0。

逐 bar 循环确实执行了：100k 活窗调用量从 89576 增为 11026776，dump 每 bar 有
`FEED cadence=bar`。终局分布不变不是循环没有运行，而是每只身份第一次可见时完成信号也
在同一 bar 到达，按 Q1/Q2 必须当场结算。

### 9.4 非 lifecycle 字节面与 lifecycle 修正本体

**认识论等级：L2**（固定真实回放集上的精确 `cmp` 与离散事件计数）；证明范围限于表列
p123 前缀及 m8 三窗，不冒充全定义域不变性。

| 面 | cmp | SHA-256（pre = post） |
|---|---:|---|
| p123 20k stdout | 0 | `bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6c` |
| p123 20k P116 dump | 0 | `fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a40` |
| p123 100k stdout | 0 | `d8b69c180c23c5e393bf3c330825d88ae9c989ff38f2b8865e049e1b5eb56da8` |
| p123 100k P116 dump | 0 | `8a7327feb3b9ba29f69fa824af1f1ecc137d9303637b47ad8465b6d638705f84` |
| m8 p3fold trades / tower_events | 0 / 0 | `2da686833581d5358a1806aa41ad7ba16371bc3db190fe2a8ccfe62b9cb46627` / `83f45a36ab422a92d198d5d9289c40db94e95fe2cba7af2322e2d37aa8e718c2` |
| m8 wf7 trades / tower_events | 0 / 0 | `3371f1e62153e8aa216ae6f28530bcaa95423a1e3f9b5341e647ec1a78a3fca8` / `aa96b3e836be5a43514c425d572c5312d15d8c6950d991d094416e4d61af1cbb` |
| m8 wf8 trades / tower_events | 0 / 0 | `006c31f54cd72d8ec9c9461122068faf58377cad2d176f839f6a6e0c5ff601b7` / `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` |

lifecycle dump 是修正本体，不作 `cmp=0`：

| 前缀 | trigger 粒度 SHA-256 | 独立逐 bar SHA-256 |
|---|---|---|
| 20k | `7a927e9b23d6ed2e0ba06ba7eed8cc96fbda4574c77f56836a06394458bfdda8` | `940a69dbaa96b6ff19ee8dc934d6cae65b378c771fe512d34273d1507727bd1b` |
| 100k | `7ce0b1b39c4837657525696f695d1acbe1aec31414be8dc8e1392c6f31b8b5d9` | `3df5fbc732514929fcde7c6db6298b40539ca338e8af799ea0de58ac729db71c` |

证据：`/tmp/wt421e-final-{20k,100k}-p123.{stdout,stderr}`、
`/tmp/wt421e-final-{20k,100k}-{replay,lifecycle}.dump`、
`/tmp/wt421e-final-m8-default-{p3fold,wf7,wf8}/`。

### 9.5 p409 同前缀神谕对照与直接反证

**认识论等级：L2**（同一 BTC 100k 真实前缀上的反事实对照）；p409 的有效域是
post-completion holding 条件，不是生产结算真值，现行裁定见 §10。

p409 已在本轮同码、同 BTC 100k 前缀重跑：

| BTC 100k 指标 | p409 pan-live-only | p123 独立逐 bar 双通道 |
|---|---:|---:|
| identities / entries | 248 | 248 |
| 曾可证 | 124 | 151 |
| ForceOvertake | 113 | 0 |
| Force / 曾可证 | 91.129032% | 0% |
| Force 寿命 min / median / max | 9 / 378 / 20966 | 无 |
| Confirmed / NeverConstituted | 0 / 0 | 151 / 97 |
| IdentityVanished | 20 | 0 |
| 闪现终局 | 不适用（无完成通道） | 248（100%） |
| 非闪现寿命 | Force 113 只 | count=0 |
| MissingForceSeries / CoordinateMapFailed | 0 / 0 | 0 / 0 |

证据：

- `/tmp/wt421e-p409-btc100k.log`；
- `/tmp/wt421e-p409-btc100k-entries.jsonl`；
- `/tmp/wt421e-p409-btc100k-force-lifetimes.txt`；
- `/tmp/wt421e-p409-btc20k-verify.log`：`P409_VERIFY=1`，
  `verify_mismatches=0`，证明 forest_epoch 缓存与每 bar 强制重算的活窗集合一致；
- `/tmp/wt421e-first-vs-completion.tsv`：按
  `(level, side, seg_a, c_start, b_center_start)` 逐身份 join。

决定性对照：p409 的 248 个身份与生产 248 个首完成身份全量一一匹配；248/248 都满足
`p409.observed_at == COMPLETION_SIGNAL.as_of`，差值 min/max=`0/0`。因此不存在「逐 bar
循环漏掉更早活窗」可修；同一 bar 必须先观察后完成，结果必为零寿命闪现。

p409 的 113 个 Force 来自其硬编码 `structure_completed=false` 后继续延展；若在双通道账本
中复现这 113 个 Force，只能忽略同 bar 已到的真实首完成并继续延展，正是 #429 复审禁止的
「人工延迟结算造寿命」。本轮没有走该路径。

### 9.6 失败归因与遗留

**认识论等级与适用边界：L2，非 L3。** 以下归因只覆盖 commit `391936a486`、
当前 PanLive provider 与本轮 BTC 20k/100k 前缀；不得外推到其他标的、时段、provider
或完成定义。E 裁定已重译其中第 1–3 项，现行结论见 §10。

1. **裁定目标与既有完成定义不能同时成立。** 独立逐 bar 节拍已落码并实跑，但它不改变
   `PanLive` 与完成 `Event` 在数据源上的首次可见时点；两者逐身份完全同刻。
2. **验收 3 未满足，验收 4 只能部分满足。** 生产 dump 无合法 ForceOvertake 实例，
   故不伪造 ForceEvidence 链。
3. 若仍要求「终局反超主导 + 中位数百根」，须由编排者另裁至少一项语义：
   完成信号取值/身份桥/完成后是否吸收。executor 无权用实现技巧替裁。
4. #454/#497、roster、四份 shadow 报告、map 与 issue 状态均未触碰。

## 10. E 执行节（2026-07-28）

本节执行 #421 E 裁定（comment-5106372198）及收口包 a
（comment-5106781616）。§0–§9 原文保留为裁定前与逃生门阶段的历史记录；凡状态、
神谕或适用边界与本节冲突，以本节为准。

**现行总判定：满足 E 收窄后的 #421 生产验收口径。** E 已把“反超主导”移出本票生产
验收；本票只对当前 provider 首次可见即完成的闪现主导结算账本负责，不再要求生产
ForceOvertake 实例。

**统计口径**：对 `P421_LIFECYCLE_DUMP` 的离散事件逐行精确计数，以
`(level, side, seg_a, c_start, b_center_start)` 作身份 join，并对既有输出作精确
`cmp`；无抽样、置信区间或推断统计。p409 单列为 completion 后继续 holding 的反事实
计算，不与生产分母混算。

### 10.1 E 改判与证据

| 项目 | E 后判定 | 证据 |
|---|---|---|
| p409 反超读数 | **移出生产验收** | p409 的 `structure_completed=false` 会在真实完成后继续延展；113 个 ForceOvertake、91.129032% 与寿命 `9/378/20966` 是 **post-completion holding counterfactual**，回答“若完成后不结算会怎样”，不是生产真值 |
| §9.2 项 4（结算时序） | **满足** | BTC 100k 有 100000 条连续 `FEED cadence=bar`（0..99999，gap=0）；修订 `as_of` 回退=0；`retrograde_rejected=0`、`IdentityVanished=0`；248 个 entries 对应 248 个唯一 `COMPLETION_SIGNAL`，首完成覆盖 248/248、事件零丢弃 |
| 完成前活窗 destination | **登记 #527** | map #59 子票 #527「PanLive provider：完成前活窗可见（L1 先行）」承接当前 provider 在完成前不暴露活窗的上游缺口；完成前活假设生产化与 #421 解耦 |

因此，§9.2 项 4 的旧“部分满足”仅是 E 裁定前历史：其扣分条件是“必须交付生产
ForceOvertake 链”，而 E 已明确删除该生产门槛。现行结算门只要求真实完成照实结算、
事件零丢弃、身份与时钟单调；上述四项均有在案实测。

### 10.2 认识论等级与有效域

| 证据/声明 | 等级 | 有效域 |
|---|---|---|
| E 对生产验收、p409 角色与 #527 分工的定义性裁定 | **L0** | 定义/治理口径，不由数据本身推出 |
| F10a/F10b/F10c 及其他确定性合成测试 | **L1** | 验证逐 bar 两相 feed、闪现链和 ForceOvertake 状态机可执行；不证明真实 provider 会产生完成前活窗 |
| §9.2–§9.5 的 BTC 20k/100k、p123/m8 对拍及 p409 身份 join | **L2** | 单标的、固定前缀/窗口的真实数据验证；可否证旧神谕，但不得外推到其他标的、时段或 provider |
| 跨标的、跨时段、跨代理变量验证 | **L3：未做** | 本报告不声明 L3，也不据 BTC 单样本声称普遍闪现率或普遍终局分布 |

现行适用边界固定为 commit `391936a486` 的逐 bar sidecar、当前 PanLive provider、
ADR-0003 完成定义及本报告列出的 BTC/p123/m8 证据集。#527 改变 provider 可见性后，
应在其票内重新给出至少 L2 读数；本节结论不得自动继承。

### 10.3 #527/#559 后的口径失效登记（2026-07-28 晚，#559 条件 C4）

§10 落笔时的两条表述已被后续票取代，登记于此，**不静默改写上文**：

1. **`IdentityVanished=0` 不再是不变量。** §10.1 表格第 2 行把它列为「结算时序满足」的
   证据之一。#523 已钉死它只是「首见即完成」bug 的副产品（活窗与完成同刻出生 ⟹ 身份
   从不跨 bar 存活 ⟹ 无从消失）。#527 让 L1 活窗真实跨 bar 存在后该读数变为 37；
   **#559 编排者裁定（2026-07-28，comment-5108273992）撤销该不变量**，替换为
   「凡消失的身份必须带可审计原因码，且成因拆两类落账本字段」——(a) 假设被推翻
   （真终局，BTC 100k = 25 只）/(b) 观测接缝伪影（provider 换轨丢下，= 12 只）。
   引用本节 `IdentityVanished=0` 的下游判定须按新不变量重读。
2. **legacy trigger adapter 口径失效。** #430 收口条件 1 写入
   `v3-lifecycle-rebuild-impl-20260724.md` 的「`ReplayPrefixFeed`/`feed_replay_prefix`
   仅为 legacy/测试兼容」已过期——该对符号由 #527（`7b4547b623`）整体删除，
   现行唯一生产喂入口为逐 bar `feed_replay_bar`；测试改用本地夹具 adapter
   `feed_prefix_phases`。该文件已同步落再订正注记。
3. **完成钟由三分收窄为两分。** §10 未直接引用，但 #527 曾分列
   `completed_at`/`observed_completion_at`/`as_of` 三钟；#559 条件 C3 实测第三钟在生产
   248/248 恒等于 `as_of`（相构造与账本喂入同 bar 同调用链），已删除。完成可见性滞后
   读数口径不变（= `as_of − completed_at`）。

上述三条的实测与逐条对照见 `issue527-panlive-l1-provider-20260728.md`「#559 修复节」。

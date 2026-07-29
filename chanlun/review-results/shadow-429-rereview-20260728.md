# #429 复审：#421 返工后翻转核验

- 日期：2026-07-28
- 工位：`/tmp/kimi-nest-mainline`
- 固定审阅范围：`a12a1022d9ddd8d1cae867a107a3a33c359358cf..f6639a2a1bb5`
- 返工提交：`f6639a2a1b fix(theta): #421 返工 P-H1/P-H2/P-H3 生命周期语义`
- 总结论：**FAIL**
- Spec 轴：**FAIL（P-H1、P-H2 未翻转；P-H3 已翻转）**
- Standards 轴：**FAIL（HIGH×1 / MED×6 / LOW×1；原 S-H1 按裁定例外排除）**

## 1. 结论

返工报告的表面统计成立：100k post 的 229/229 个已完成身份均为正寿命，
`min/median/max=5/120/411`；`NeverConstituted=89`、`ForceOvertake=1`；缓存复用率
95.855%；非 lifecycle 产物全部 `cmp=0`。

但这批“正寿命”不是 #421/#426 定义的生产生命史。实现遇到同 trigger 的活窗与首个完成事件时，
先丢弃真实完成事件；只有 provider 在下一 trigger 重发同一完成事件，才把它计作
`completion_signal/channel_switch`。因此：

1. `Observed` 被记在原本已经结构完成的时点，`StructureCompleted` 被人为推迟到下一 trigger；
2. 唯一 `ForceOvertake` 是忽略了一个本应当场 `Confirmed` 的完成事件后，继续延展出来的反超；
3. 18 个首完成信号因未等到有效重发而漏出分母，并被记作 `IdentityVanished`；
4. `0/230` 不是“唯一首完成信号”的发生率；同一输入中真实首完成身份为 248。

这直接违反 #421 的“数据源切到完成事件通道的那一刻即结构完成，不另造判据”
（#421 票体 `Implementation Decisions/结构完成判据` 第 69-71 行）及 #426 的同文硬项
（#426 票体第 13-16、27-32 行）。故不能以正寿命数值、一个 ForceOvertake reason code 或
25/25 单测绿翻转原 FAIL。

## 2. 范围、隔离与证据源

Spec、Standards 两轴由两个独立新上下文只读复核；主上下文只合并固定 diff、生产产物、
仓规和独立重跑，不采用返工自查的自评结论作为证明。

固定范围实际含 8 个提交、22 个文件、`+3086/-183`；除 #421 三个提交外，还夹有
#446、#511-#514 旁支提交。Spec 轴只按 #421/#426 授权验收；Standards 轴按用户指定对
`a12a1022d9..f6639a2a1b` 全 diff 检查，因此旁支问题单列、不反向归因给 #421。
证据锚：`git log --reverse --oneline a12a1022d9..f6639a2a1b`、
`git diff --stat a12a1022d9 f6639a2a1b`；`git diff --check` 为 0。

复审期间当前 checkout 被外部并发推进到 `5c58ec6a417c`。执行
`git diff --quiet f6639a2a1b..HEAD -- rust/src/bin/p123_fast_replay.rs
rust/src/theta_v0/classifier/nest_lifecycle.rs
rust/src/theta_v0/classifier/level_view.rs` 返回 0，故 #421 目标 Rust 证据仍等于固定端点。
初始既有 `chanlun/agent-roster-2026-07-21.md` 修改和两份旧评审未跟踪文件未触碰、未纳入归因。
落盘后终检又观察到外部并发新增/修改
`rust/src/theta_v0/backtest/wverify_run.rs`、`rust/src/theta_v0/backtest/wverify_run/` 与
`shadow-430-rereview-20260728.md`；本复审未创建、读取或修改这些项，亦不纳入结论。

规格与谱系依据：

- #421 票体第 13-22、40-45、60-72、93-125 行；
- #426 票体第 5-38 行及评论 `5093953744` 第 14-34 行；
- #421 编排者评论 `5094346688` 第 2-11 行；
- `chanlun/escalate/r43-lifecycle-ruling-20260721.md:19-38`；
- `chanlun/escalate/r1-r3-ruling-confirmation-checklist-20260720.md:13-22`。

## 3. 决定性反证：正寿命由延迟首完成信号制造

### 3.1 声明与代码相互矛盾

`f6639a2a1b:rust/src/theta_v0/classifier/nest_lifecycle.rs:1113-1117` 正确声明：
完成事件通道的切换本身就是结构完成信号，且“不另造判据”。紧接着：

- `:1119-1121` 声明同 trigger 首见碰撞要把完成信号留到下一 trigger；
- `:1165-1172` 以 `provisional_bridge_hit` 为前置条件，book 中没有先前 Provisional
  就直接 `continue`，丢弃本次完成；
- `:1173-1183` 只对通过该门的重发事件计 `completion_signals/channel_switches`；
- `:3002-3061` 的 F9a 把该延迟固化成测试：as_of=99 已有真实完成事件却断言不完成，
  重发到 as_of=109 才断言完成，并把 `99 < 109` 当成“正寿命”。

“book 里必须先有上一 prefix 的 Provisional”是规格没有的第二完成判据；其目的和效果都是
避免零寿命统计，而不是照实记录数据源首完成时点。证据锚：
`nest_lifecycle.rs:1113-1121,1165-1202,3002-3061@f6639a2a1b`。

### 3.2 独立全量配对

配对口径为桥身份
`(level, side, kind, seg_a, seg_c_left, b_center_start)`，忽略规格授权可迁移的
`seg_c_full` 右端。全量扫描
`/tmp/wt421r-{pre,post}-100k-lifecycle.dump` 得到：

| 项 | pre | post | 独立核验 |
|---|---:|---:|---|
| Observed | 248 | 248 | 数值相同 |
| StructureCompleted | 248 | 229 | post 少 19 个完成，其中 18 个终局为 IdentityVanished |
| 正寿命 | 0/248 | 229/229 | post `min/median/max=5/120/411` |
| 终局 | Confirmed 151 / Never 97 | Confirmed 140 / Never 89 / Force 1 / Vanished 18 | reason 数值与自查一致 |

进一步的跨版本配对结果：

- 248/248 个 post `Observed.as_of` 都等于同桥身份的 pre
  `StructureCompleted.as_of`；
- 229/229 个 post 完成时点都恰好是其 `Observed` 后的下一条 FEED trigger；
- pre/post 的 1206 条 `(FEED as_of, live_windows, completion_events)` 完全相同，
  所以延迟不是上游事件晚到；
- 终局映射为：`Confirmed→Confirmed 137`、`Never→Never 85`、
  `Confirmed→IdentityVanished 9`、`Never→IdentityVanished 9`、
  `Confirmed→ForceOvertake 1`、`Confirmed→Never 4`、`Never→Confirmed 3`。

原始例：pre 在 as_of=98365 已 `Observed→StructureCompleted→NeverConstituted`
（`/tmp/wt421r-pre-100k-lifecycle.dump:2078-2081`）；post 在同一 as_of 只记
`Observed`，到下一 trigger 98701 才完成
（`/tmp/wt421r-post-100k-lifecycle.dump:2284-2289`）。

因此“229/229 正寿命”的算术 characterization **如实**，但“生产身份真实存活”
这一语义 characterization **不实**。

### 3.3 唯一 ForceOvertake 不是规格口径的真实可达

同一 level=2 / Short 桥身份：

- pre 在 as_of=17600 已 `Observed→FirstProvable→StructureCompleted→Confirmed`
  （`/tmp/wt421r-pre-100k-lifecycle.dump:693-697`）；
- post 在 as_of=17600 丢弃完成，只留活身份；到 17902 才
  `Supersedes→ForceOvertake`
  （`/tmp/wt421r-post-100k-lifecycle.dump:720-725`）。

post 的 ForceEvidence 六字段确实齐全：
`area_a=7253162464.856313`、`area_c=11970335251.256758`、
`dif_peak_a=1092708212.473755`、`dif_peak_c=1194896516.8129272`、
`hist_peak_a=496631013.87327087`、`hist_peak_c=524288816.3438473`
（post dump `:725`）。但它发生在一个本应已 Confirmed、停止延展的身份上，故只能证明
reason code 经延迟路径可达，不能满足“真实数据上的 ForceOvertake 终局”。

同类漏记例：pre 在 as_of=1909 已 Confirmed
（pre dump `:448-452`），post 在 1909 只观察，2047 记为
`IdentityVanished`（post dump `:449-454`）。

## 4. 原 §7 六项翻转条件逐条对照

| # | 翻转条件 | 判定 | 证据锚与理由 |
|---:|---|---|---|
| 1 | 正寿命身份 | **部分** | 229/229 与 5/120/411 数值复算一致；但 248/248 的 post 观察钟就是 pre 完成钟，229/229 都延到下一 trigger，见 §3。不是规格定义的正寿命。 |
| 2 | 两类否证真实数据可达 | **未满足** | NeverConstituted 真实存在；唯一 ForceOvertake 来自忽略 as_of=17600 已 Confirmed 的首完成信号，见 pre `:693-697` / post `:720-725`。 |
| 3 | audit 接缝可达 + 正确分母 | **部分** | F9 已只经生产 `feed_replay_prefix`，红 `/tmp/wt421r-ph2-red.log:432-446`、绿 `...ph2-green.log:435-438`；但代码在计数前丢弃无先前 Provisional 的首完成，真实 248 个首完成身份只计 230，`0/230` 分母漏 18。 |
| 4 | US15 复用既有缓存 | **满足** | 唯一 `derived/entries` 在 `p123_fast_replay.rs:861-862`；targeted/lifecycle 共用见 `:921-930,1121-1149`；dirty 判据与复用见 `:1195-1304`。100k `2099/87/2012`，复用率 95.855%。 |
| 5 | 字节护栏 | **满足** | 本复审对 10 对非 lifecycle 产物独立 `cmp` 均为 0，SHA 见 §7。lifecycle SHA 变化是修复面；数值 characterization 如实，语义解释不实。 |
| 6 | 测试 | **满足（既有红除外）** | 指定命令独立重跑为 `1992 passed / 1 failed / 135 ignored`；唯一红 `signal::extract_signals_bit_exact_digest_guard`，锚 `signal.rs:3382`、#491。 |

100k stderr 也与返工自查一致：

- pre：`triggers=1206 live_windows=89576 completion_events=125453
  channel_switches=89576 extension_suppressed=0`
  （`/tmp/wt421r-pre-100k-p123.stderr:5`）；
- post：`completion_signals=230 channel_switches=230 extension_suppressed=89098
  provider_requests=2099 provider_reevals=87 provider_reuses=2012`
  （`/tmp/wt421r-post-100k-p123.stderr:5`）。

“230 ≪ 89576”只证明新门缩小了 switch 计数，不能证明门符合结构完成定义。

## 5. Spec 轴：#426 转呈 18 项

| # | 转呈条款 | 判定 | 证据锚 |
|---:|---|---|---|
| 1 | 只在重估 trigger 喂 | 满足 | `p123_fast_replay.rs:876-878,1121-1149@f6639a2a1b` |
| 2 | 倒退显式拒绝、账本零改动、可审计 | 满足 | F5/T18；`/tmp/wt421r-nest-after-reapply.log:438,444` |
| 3 | 同身份切入完成通道即结构完成 | **部分** | 正常桥接可完成；首同刻完成被 `provisional_bridge_hit` 门丢弃，`nest_lifecycle.rs:1165-1172` |
| 4 | 完成后停止延展 | **部分** | 代码认可完成后会停；但规格定义的首完成后仍延展到下一 trigger，Force 例见 §3.3 |
| 5 | 切换处力度跃变专属覆盖 | 满足 | F3；`/tmp/wt421r-nest-after-reapply.log:450-452` |
| 6 | 两类否证在真实数据均可达 | **未满足** | Never 可达；Force 是延迟已 Confirmed 身份产生，pre/post dump `:693-697/:720-725` |
| 7 | 订单流逐字节不变 | 满足 | p123 stdout/replay 与 m8 trades 独立 `cmp=0`，见 §7 |
| 8 | 挂/不挂时数据源事件流逐字节不变 | 满足 | m8 tower_events 独立 `cmp=0`；F6 单测强度另列 S-L1 |
| 9 | 主接缝常规测试时限 | 满足 | lifecycle 25/25，0.01s；`/tmp/wt421r-nest-after-reapply.log:433-460` |
| 10 | 腿转段复制、不扩私有可见性 | 满足 | `nest_lifecycle.rs:1051-1064@f6639a2a1b` |
| 11 | 不进入装配/证书真值路径 | 满足 | sidecar 仅写独立 dump；订单、replay、tower_events 均 `cmp=0`；固定 diff 未见真值消费边 |
| 12 | 全库零新增失败 | 满足 | 1992/1/135；唯一红 #491，`/tmp/wt421r-cargo-test-lib.log:2566-2577` |
| 13 | 完成且不可验可审计 + 发生率 | **部分** | 生产接缝 F9 已可达；分母 230 漏掉 18 个首完成信号，见 §4.3 |
| 14 | p123 生产真接线 | 满足 | `p123_fast_replay.rs:1121-1149,1321-1411@f6639a2a1b` |
| 15 | 公共 API 文档订正 | 满足 | `nest_lifecycle.rs:154-184,225-245@f6639a2a1b`；feed 注释自相矛盾另按 090 记 HIGH |
| 16 | release 显式核验或降级声明 | 满足 | 无 cfg 的 `lifecycle_book.assert_invariants()`，`p123_fast_replay.rs:1148` |
| 17 | 喂入来自生产产窗/创新高预滤 | 满足 | `provide_pan_live_windows` 生产调用，`nest_lifecycle.rs:1137-1155`；完成事件复用 RunEntry |
| 18 | CONTEXT 只登记、编排者执行 | 满足 | 固定 diff 未改 `CONTEXT.md`；自查只作登记 |

计数：**满足 14 / 部分 3 / 未满足 1**。#421 US15 已满足，但“完整生命史”核心仍未满足；
两者不能互相抵消。

## 6. Standards 轴

### 6.1 原 findings 去向

- **S-H1 不复审**：`5022e31df5:.claude/rules/common/coding-style.md:15-19`
  已正式排除带修订留痕的单线程状态机/账本；当前账本有 `LifecycleRevision` 留痕，例外适用。
- **原 S-M2 已满足**：返工自查补了 r43 与 r1-r3 谱系，
  `chanlun/review-results/issue421-acceptance-selfcheck-20260727.md:157-161`。
- **原 S-L2 去向合理**：`leg_as_segment_copy` 是 #421/#426 明示授权复制，不提升既有私有
  函数可见性；只登记 Fowler Duplicated Code，不判违规。
- **原 S-M1 去向不完整**：#454 票体只覆盖 `nest_lifecycle.rs` 与 `advance`；
  未覆盖 `p123_fast_replay.rs`、`level_view.rs` 及本次新增的多个超长函数。
- **原 S-L1 未闭合**：返工承认 F6 不是生产字节门，却只写“由编排者另派”，没有票号或
  仓内自动门；外部一次性 `cmp=0` 不能替代未来回归门。

### 6.2 Findings

#### HIGH — S-H2：以重发 workaround 制造正寿命，违反 090

实现明知“切换本身即完成信号、不另造判据”，却为避免出生即完成而丢弃首完成、等下一 trigger
重发；F9a 进一步把 workaround 固化成正确行为。违反：

- `.claude/rules/no-patch-mentality.md:15-23,28-35,39-43` 的声明=能力、禁 workaround、
  禁半成品与声明膨胀；
- `.claude/rules/no-workaround.md:7-21` 的规格与实现根本不一致时不得绕过；
- `CLAUDE.md` 原则 090 的“照实”。

代码锚：`nest_lifecycle.rs:1113-1121,1165-1172,3002-3061@f6639a2a1b`。

#### MED — S-M1：规模/Data Clumps 仍超限，#454 只覆盖一部分

- 文件：`p123_fast_replay.rs=1997`、`nest_lifecycle.rs=3140`、
  `level_view.rs=1864` 行；
- 函数：`feed_replay_prefix` 91 行（`:1122-1212`）、
  `refresh_lifecycle_cache` 82 行（`p123_fast_replay.rs:1223-1304`）、
  `feed_lifecycle_trigger` 91 行且 10 参数（`:1321-1411`）；
- `advance` 仍约 206 行（`nest_lifecycle.rs:619-824`）。

违反 `.claude/rules/common/coding-style.md:21-26,45-51` 的文件 800、函数 50 上限。
#454 票体第 5-18 行只承接账本文件与 `advance`，所以返工自查
`issue421-acceptance-selfcheck-20260727.md:198,206-207` 的“仍归 #454”不能完整核销。

#### MED — S-M2：结果文档出现新口径冲突

`chanlun/review-results/v3-lifecycle-rebuild-impl-20260724.md:82-85` 指向本文件不存在的“§7.3”；
同文件 `:99-109` 又保留 `0/3715`，而返工自查 `:151` 声称唯一分母为 230。
这违反 090 声明一致性和 `.claude/rules/result-package.md:3-12` 的可传递结果包要求。
原“缺谱系”虽已修，新的现行口径冲突未登记去向。

#### MED — S-M3：仓内 integration/E2E 字节门仍缺

P-H3 单测只锁同一 `RunEntry` 的切片指针借用
（`p123_fast_replay.rs:1973-1995`）；F6 只比较同一数组的 `PartialEq` 与 Debug 文本
（`nest_lifecycle.rs:2827-2861`）。生产 pre/post `cmp=0` 是有效的一次性验收证据，但没有成为
仓内自动 integration/E2E 门。返工自查 `:209-210` 也承认仅“部分”，且未给 follow-up 票号。
违反 `.claude/rules/common/testing.md:3-8` 的 ALL test types 要求。80% 覆盖率因无 coverage
报告，**未能判定**。

#### MED — S-M4：旁支 #446 的 release 失败会静默无写

固定端点的 `ElementView::replace_overlay` 只用 `debug_assert!` 守边；release 下索引不在
overlay 或越界时 `get_mut` 返回 None，函数静默结束
（`rust/src/theta_v0/strategy/coverage/element.rs:120-128@f6639a2a1b`）。
调用者随后无条件返回 `existing`
（`rust/src/theta_v0/strategy/coverage/held.rs:119-136@f6639a2a1b`）。
违反 `.claude/rules/common/coding-style.md:29-35` 的禁止静默吞错。
该越界能否由当前 `overlay_seen` 生产不变量触发，**未能判定**；静态 fail-loud 缺口成立。

#### MED — S-M5：旁支 #511-#514 新 Python 测试不符合仓规

`scripts/tests/test_check_armR_trades_digest.py:1-69@f6639a2a1b` 使用 `unittest`，且新增函数签名
无类型注解；违反 `.claude/rules/python/testing.md:10-12,20-28` 的 pytest/mark 口径及
`.claude/rules/python/coding-style.md:10-13` 的全签名类型注解要求。

#### MED — S-M6：固定端点 #514 的可选 lineage 会遮蔽必验 base→final

`scripts/check_armR_trades_digest.py:213-228@f6639a2a1b` 在存在
`source_lineage_start` 时只验 `lineage→final`，不再验必填的
`source_base_head→final_verification_head`；合法 lineage 可掩盖倒挂 base。
端点之后的 `9928948aa3`（#515）才改为恒验 base→final、另验 lineage→base，故本项是
固定范围内真实残留、当前 HEAD 已修，不归因 #421。

#### LOW — S-L1：F6 名称/注释强于实际断言

`f6_feed_exit_is_sidecar_events_byte_identical` 比较的是原数组与其 Copy，以及两份 Debug 字符串，
不是生产序列化字节流（`nest_lifecycle.rs:2827-2861@f6639a2a1b`）。外部字节对拍成立，
但不提升该单测自身的保证强度。

### 6.3 仓内标准源覆盖

已逐项过：

- `no-patch-mentality.md`、`no-workaround.md`、`formalization-validity-domain.md`：
  命中 S-H2；唯一 Force 的 L2“真实可达”声明被否定；
- `common/coding-style.md`、`common/testing.md`、`testing-override.md`、
  `result-package.md`：命中 S-M1/S-M2/S-M3/S-M4；
- `python/coding-style.md`、`python/testing.md`：命中 S-M5；
- `common/security.md`、`common/patterns.md`、`common/performance.md` 及其余
  agent/git/hook/角色边界规则：固定 diff 未发现可锚定的新安全、公开 API、权限或角色越界；
  P-H3 的缓存复用符合性能方向。

### 6.4 Fowler 12 条

| # | 臭味 | 判定 |
|---:|---|---|
| 1 | Mysterious Name | 未发现固定 diff 新增的可锚定项 |
| 2 | Duplicated Code | 有：`leg_as_segment_copy`；规格明示授权，只登记 |
| 3 | Feature Envy | 未发现 |
| 4 | Data Clumps | 有：hist/dif/close_src、缓存/账本参数组反复穿透，计入 S-M1 |
| 5 | Primitive Obsession | `(usize, usize)` 段窗存在；沿用既有域形状，仅低风险观察 |
| 6 | Repeated Switches | 未发现 |
| 7 | Shotgun Surgery | 未发现可与本次单一语义变化建立因果的新增项 |
| 8 | Divergent Change | 有：p123 与 lifecycle 大文件继续承载多职责，计入 S-M1 |
| 9 | Speculative Generality | 未发现 |
| 10 | Message Chains | 未发现 |
| 11 | Middle Man | 未发现 |
| 12 | Refused Bequest | 未发现 |

“未发现”仅限固定 diff，不声明全仓不存在。

## 7. 独立产物与测试抽核

### 7.1 非 lifecycle 字节护栏

| 产物 | cmp | pre/post 相同 SHA-256 |
|---|---:|---|
| p123 20k stdout | 0 | `bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6c` |
| p123 20k replay dump | 0 | `fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a40` |
| p123 100k stdout | 0 | `d8b69c180c23c5e393bf3c330825d88ae9c989ff38f2b8865e049e1b5eb56da8` |
| p123 100k replay dump | 0 | `8a7327feb3b9ba29f69fa824af1f1ecc137d9303637b47ad8465b6d638705f84` |
| m8 p3fold trades | 0 | `65621e3505253a7fcde046a9eaa78bbc8c19798fe84ed626c88b56d6cb0cc21b` |
| m8 p3fold tower_events | 0 | `83f45a36ab422a92d198d5d9289c40db94e95fe2cba7af2322e2d37aa8e718c2` |
| m8 wf7 trades | 0 | `db6e14fab9f2e5767f38c832aead1efcf6c06b0831181e6c7c7cdaa3deaffbfb` |
| m8 wf7 tower_events | 0 | `aa96b3e836be5a43514c425d572c5312d15d8c6950d991d094416e4d61af1cbb` |
| m8 wf8 trades | 0 | `abf8245ffd880c0f4488cd02f8696e950cd8349de15c705b52a2add3d679eaf8` |
| m8 wf8 tower_events | 0 | `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` |

### 7.2 lifecycle SHA

- 100k pre：`e4c99b8615d64363cbb0d367a42f8df2550cbc00511a2ae8e340fa3f48e8d826`
- 100k post：`d45201143c3d2171216849d5acdcfefbb2195c6a889d9f853282dcbde94c6c0e`
- 20k pre：`4f900dbac64c2c890a5af85add1e657ca4497d3b5dfa08ce769783efbade3c1e`
- 20k post：`6a8877ee6549d50204407cf06dd466bd68874e4bf26b46d35468f699b4b80a52`

pre 二进制锚为 `ce074711d5edbb18b59fa5f0990907f78d2cba49`
（`/tmp/wt421r-pre-head.txt:1`）；之后到返工父端的 #514 只改 scripts，不触碰 #421 Rust。

### 7.3 US15 与测试

- 100k：`provider_requests=2099 / provider_reevals=87 / provider_reuses=2012`，
  复用率 `2012/2099=95.855%`；
- 20k shadow：`179 checks / 0 mismatch`
  （`/tmp/wt421r-post-20k-shadow.stderr:1`）；
- lifecycle 模块：25/25，0.01s
  （`/tmp/wt421r-nest-after-reapply.log:433-460`）；
- 指定命令：

```text
cd /tmp/kimi-nest-mainline/rust
CARGO_TARGET_DIR=/tmp/kimi-nest-target-check cargo test --lib 2>&1 | tail -3

test result: FAILED. 1992 passed; 1 failed; 135 ignored
```

唯一失败为在册 #491：
`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`
（`/tmp/wt421r-cargo-test-lib.log:2566-2577`）。零新增失败。

## 8. 未能判定

1. #450 结论对按级别数据解读的反冲：固定范围没有完成该复核，仍**未能判定**；
2. 80% 覆盖率：没有 coverage 产物，**未能判定**；
3. S-M4 的 release 越界分支是否可由当前生产不变量触发：未构造到，**未能判定**；
4. 本报告只证明所给 20k/100k 与 m8 三窗产物，不外推其他标的、窗口或未来提交。

## 9. 翻转边界、下游推论与影响声明

本 FAIL 翻转的最低条件：

1. 首个完成事件必须在其真实到达的同一 trigger 结算；不得依赖下一 trigger 重发。
   正寿命若要成立，须来自更早的生产活窗观察，或由编排者正式改裁 #421/#426 的完成定义；
2. 同一 100k 输入重跑时，首完成信号分母须覆盖全部真实首完成身份；不能再出现因丢弃首完成而
   `Confirmed/Never→IdentityVanished`；
3. ForceOvertake 实例须发生在合法首完成之前，而不是在忽略本应 Confirmed 的完成信号后产生；
4. 保留已通过的 F9 生产 audit 接缝、共享 dirty/cache、release assert 与所有字节护栏；
5. S-M1 须给 #454 未覆盖的文件/函数明确去向；仓内 integration/E2E 字节门须有具体票号；
   结果文档的 3715/230 与 §7.3 引用须统一。

下游推论：

- #421 不能按当前返工收为完整验收；#429 复审结论为 **FAIL**；
- #426 按编排者裁定继续 CLOSED，修复与再验收留在 #421，不因本报告重开；
- P-H3、F9 主接缝、release 显调、左端断言及非 lifecycle 字节护栏可保留；
- 当前 `ForceOvertake=1`、`IdentityVanished=18`、`completion_force_unavailable_rate=0/230`
  不得作为后续教义或发生率裁定输入；
- #450 后按级别解释继续挂“未能判定”。

影响声明：本复审未修改代码、票据、分支、既有文档、用户脏项或上述并发产物；只新增并更新
本报告 `chanlun/review-results/shadow-429-rereview-20260728.md`。

<oai-mem-citation>
<citation_entries>
MEMORY.md:246-249|note=[定位 SPEC 421 生命周期原始意图并以实时证据复核]
extensions/chronicle/resources/2026-07-27T14-56-00-iqFt-10min-memory-summary.md:45-53|note=[确认产生观察确认否证历史和行为护栏]
</citation_entries>
<rollout_ids>
</rollout_ids>
</oai-mem-citation>

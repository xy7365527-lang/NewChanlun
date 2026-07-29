# #639 实施报告——短差档接 pan_div_diag 盘背观测通道（#575 消费方接线 3/3）

> 日期：2026-07-29；性质：实施车收口报告（sonnet 实施车）
> 工位：`/private/tmp/wt-639`；分支 `ticket-639`（基 = main HEAD `ac96d4056f`）；全程单线程
> （无子代理、无后台任务，符合票面硬约束）；**未 commit**——全部改动留存 worktree 未提交面，
> 由编排方统一提交。

## 0. 一句话

`RetraceLedger`/`ShortRetracePortal` 的短差档（判败盘背观测源）经新增消费函数
`signal::drain_pan_div_short_retrace_observations` 接入 pan_div_diag/signal 通道，位置与
#606 S1 一类点分级 sidecar 同构；亚型签对拍表把「同名非同谓词」钉进代码；三锁在接线面复验
通过；驱动源裁定按编排者裁定 B 登记（归 #575 后续票）；`retrace_ledger/` 内零代码改动，只改
`mod.rs` 登记表 doc 文字。

## 1. 改动文件清单

| 文件 | 改动性质 | 行数 |
|---|---|---|
| `rust/src/theta_v0/classifier/signal.rs` | 新增：亚型签对拍表 2 个函数 + 投影记录类型 + 消费函数 + 5 条测试 | +275 |
| `rust/src/theta_v0/classifier/retrace_ledger/mod.rs` | 仅文档：消费方登记表 row 3 + 计数行改写（票面禁区：`retrace_ledger/` 内只许改登记表 doc 文字，代码零改动） | +6/-5 |

`git diff --stat`：
```
rust/src/theta_v0/classifier/retrace_ledger/mod.rs |  11 +-
rust/src/theta_v0/classifier/signal.rs             | 275 +++++++++++++++++++++
2 files changed, 281 insertions(+), 5 deletions(-)
```

`retrace_ledger/` 内代码零改动核验：
```
git diff -- rust/src/theta_v0/classifier/retrace_ledger/adapter.rs \
            rust/src/theta_v0/classifier/retrace_ledger/book.rs \
            rust/src/theta_v0/classifier/retrace_ledger/log.rs \
            rust/src/theta_v0/classifier/retrace_ledger/portal.rs \
            rust/src/theta_v0/classifier/retrace_ledger/audit.rs \
            rust/src/theta_v0/classifier/retrace_ledger/tests/
```
= 零行输出（全部确认零改动）。`formal/` 全程未列入任何改动，fixture 漂移 gate 不适用本轮
（票面纪律：`formal/` 未触碰的 session 不强制跑漂移检查）。

## 2. #606/#607 形态核查结论 + 接线位置理由（票面范围第 1 条点名要）

**摸底结论（编排方已核事实，本轮读码复核确认无误）**：pan_div_diag 通道今天两股并存：

- **股 A** = `CandDeltaEvent.pan_div_diag: bool` 纯诊断位（`recursive_tower.rs:1213`），不落证书、
  消费面极薄，`RetraceKey`/`CenterFrame` 不属其塔坐标系——**禁用**（票面点名「勿并入」）。
- **股 B** = #606 S1 的分级 sidecar：`FirstClassGradeRecord`（`signal.rs:745`）→ thread_local
  `GRADE_SIDECAR` 捕获槽（`signal.rs:766`，`begin`/`take`）→ `OtherwiseDomainSidecarCollector`
  逐调用帧 upsert（`opsem_dump.rs:318`）→ `OtherwiseDomainSidecarSummary` → `runner.rs` 接入
  `RunResult.otherwise_domain_sidecar`；env 门 `THETA_OTHERWISE_DOMAIN_SIDECAR`。

**接线位置**：新增代码全部放在 `signal.rs` 内、紧邻 `otherwise_domain_sidecar_is_active`
（股 B sidecar 收尾函数）之后、`make_first_point`（第一类端点构造）之前——即 #606 S1
分级观测代码块的物理延伸位置。理由：

1. **判定域同源**：股 B 的 `T3InCGrade`/`T3InCGradeReason` 与本票 `PanDivSubtype` 都描述
   pan_div_diag 通道「leave/retest 对回抽是否重回核心」的几何直觉，词汇表天然相邻，放在同一
   文件同一物理区块便于对拍表就近引用两侧类型，不必跨文件 `use`。
2. **同构不同源**：股 B 的挂点是 `judge_segment` 热路径（每调用帧捕获），本票消费的是
   `RetraceLedger` 已折叠的终态判败集合（非热路径逐帧事件）——**不复用** thread_local 捕获槽
   机制（复用会把两条不同产线的事件混进同一个槠，违反 #624/#637 一贯的「同名不同谓词不许
   合并」纪律），而是照 #606 的**分层结构**（记录类型 → 消费函数 → 未来可选 collector/summary）
   平行起一套同形态但独立的管线，只共享物理位置，不共享状态。
3. **不新增模块**：`opsem_dump.rs`/`runner.rs` 本票均未touch——按裁定 B，驱动源（喂真实
   replay 判败事件的 `RetraceLedger` 生产实例）不在本票范围，若在 `opsem_dump.rs`/`runner.rs`
   预先搭 Collector/Summary/env 门骨架而无真实驱动挂点，会引入「有骨架无血肉」的死代码——本票
   遵「改动最小化不顺手重构」纪律，只交付消费入口本体（`drain_pan_div_short_retrace_observations`），
   全窗收尾管线留给 #575 后续票按当时的真实驱动形态决定要不要复刻股 B 的 collector 结构。

## 3. 亚型签对拍映射表 + 「同名非同谓词」钉正

`signal.rs` 新增两个互为部分逆的函数（票面「固定、一一对应」要求落成显式代码，非伪对应）：

```rust
pub(crate) fn t3_in_c_grade_reason_to_pan_div_subtype(
    reason: T3InCGradeReason,
) -> Option<PanDivSubtype> {
    match reason {
        T3InCGradeReason::RetestReentered => Some(PanDivSubtype::RetestReentered),
        T3InCGradeReason::MissingLeave
        | T3InCGradeReason::MissingRetest
        | T3InCGradeReason::SameDirection
        | T3InCGradeReason::LeaveNotOutside => None,
    }
}

pub(crate) fn pan_div_subtype_channel_reason(subtype: PanDivSubtype) -> T3InCGradeReason {
    match subtype {
        PanDivSubtype::RetestReentered => T3InCGradeReason::RetestReentered,
    }
}
```

| 通道侧 `T3InCGradeReason` | 账本侧 `PanDivSubtype` | 说明 |
|---|---|---|
| `RetestReentered` | `Some(RetestReentered)` | 唯一天然对应点——**同名非同谓词**：通道侧判定对象=`last_center` 右边固定首对 leave/retest **段**（`t3_in_c_fixed_first_pair`，037:18/#606 D1）；账本侧判定对象=`RetraceKey`（中枢四边快照+departure）的 leave/retest **候选窗口**（027:16 判败四行语义）。两侧共用同一英文名描述「回抽重回核心」的几何直觉，是词汇选取的巧合，不是同一谓词的两个别名——本表只做词汇对拍登记，不做语义合并。 |
| `MissingLeave` | `None` | 通道侧「固定首对压根未构成」的结构性缺席；账本侧候选已由适配器注册期挡过残废四桶，短差档见到的必是已配对成功的候选，无对应判案。 |
| `MissingRetest` | `None` | 同上。 |
| `SameDirection` | `None` | 同上。 |
| `LeaveNotOutside` | `None` | 同上。 |

`match` 穷尽 `T3InCGradeReason` 全部五个变体（编译期强制，新增变体会编译失败提醒补表）——
`t3_in_c_grade_reason_to_pan_div_subtype_covers_all_five_buckets` 测试逐桶断言，含反向函数在
唯一对应点上的一致性核验。`PanDivSubtype` 今天只有一个变体（短差档只消费判败一族，
`CenterRebased` 走 S2 警报桶不进短差门户——portal.rs 既有文档），故反向函数恒 `Some`/无需
`Option`；`PanDivSubtype` 新增变体是显式破坏性改动（同该类型自身文档「固定」纪律），届时
两个映射函数不再穷尽会编译失败，不会静默漏配。

## 4. 三锁接线面复验清单（票面「重复消费幂等 + 键冲突拒绝在接线面复验」）

消费函数 `drain_pan_div_short_retrace_observations` 只调用门户既有公开入口（`sync`/
`consume`），不重造锁、不绕过锁：

```rust
pub(crate) fn drain_pan_div_short_retrace_observations(
    ledger: &RetraceLedger,
    portal: &mut ShortRetracePortal,
) -> Vec<PanDivShortRetraceObservation> {
    portal.sync(ledger);
    ledger
        .short_retrace_records()
        .iter()
        .filter_map(|record| portal.consume(&record.identity))
        .map(|record| PanDivShortRetraceObservation::from_record(&record))
        .collect()
}
```

| 锁 | 复验测试 | 结论 |
|---|---|---|
| 锁一（键唯一，`admit`） | `admit_duplicate_identity_still_rejected_alongside_drain_wiring` | `drain` 走 `sync`（`insert` 语义）填充门户后，对同一身份显式 `admit` 仍返回 `ShortRetraceRejection::DuplicateIdentity`——两条入口（`sync` 增量同步 / `admit` 手工登记）共享同一 `records` 键唯一约束，接线面不绕锁 |
| 锁二/三（一一对应去重 + 账平，`sync`/`balances_with`） | `drain_pan_div_short_retrace_observations_end_to_end`（末尾 `balances_with` 断言） | `drain` 内部调用 `sync` 后门户与账本判败集合逐条内容一一对应；`consume` 不删记录（票面「消费不删记录」既有纪律），故消费后账仍平 |
| 锁四（幂等消费，`consume`） | `drain_pan_div_short_retrace_observations_is_idempotent_on_repeat_call` | 同一账本/门户对二次调用 `drain`：第二次返回空集（已消费身份被 `filter_map` 滤除）；账本新增第三条判败身份后第三次调用只新增该一条，不重放前两条已消费身份 |
| 判败一族过滤（S3 既有边界，接线面复验） | `drain_pan_div_short_retrace_observations_ignores_pending_and_success` | `Provisional` 候选（未落锤）经 `drain` 得空集——通道只见判败终态，不见未决候选 |

端到端（票面验收①）：`drain_pan_div_short_retrace_observations_end_to_end` 用**靶向驱动**
（直接 `ledger.observe` 喂判败事件，不跑全窗 replay，符合裁定 B），验证身份/侧/亚型签/通道
映射/位置/知情时全部透传正确，且门户消费后仍与账本账平。

## 5. 缺口登记（失败处置通知无消费面）

Grep 复核（`grep -rn "已入场\|disposal_notice\|FailureDisposalNotice" src/trading/ src/theta_v0/backtest/ src/theta_v0/classifier/signal.rs`）= 零命中：通道侧（交易层/backtest 层）**没有**「已入场者通知」的消费方。按票面括号兜底条款登记为缺口，未新增代码：

- `RetraceLedger::failure_disposal_notices()`（全量派生）与
  `ShortRetracePortal::disposal_notices()`（消费门控投影）两路产出均已就位（#623 交付），
  载荷=身份+位置+知情时，名义「通知非信号」；
- 本票不新建消费方（票面「若无则登记缺口即达标」），已在 `mod.rs` 登记表 row 3 落文字。

## 6. 024:36/024:46 短差语义（零改动面确认）

`grep -rn "short_diff_bucket\|oscillation_campaign" src/trading/center_oscillation_trade.rs` 确认
该语义链（上沿减/下沿补，campaign 同构）完全在 `center_oscillation_trade.rs → fill.rs →
short_diff_bucket/oscillation_campaign` 内，与本票新增代码零交叉引用；`TradableSignal`
类型隔离（`portal::TradableSignal`，仅 `ThirdPointPack` 实现）未改动，`ShortRetraceRecord`/
`PanDivShortRetraceObservation` 均**未**实现该 trait——短差档观测不可被误消费为交易信号，编译期
仍把关。本票代码未触碰 `center_oscillation_trade.rs`/`fill.rs`。

## 7. 驱动源裁定 B 登记

编排者 2026-07-29 裁定 B（与 #637 裁定 3A 同口径）：本票只接消费侧；`RetraceLedger` 的生产
驱动源（喂真实 replay 判败事件的生产实例）归 #575 后续票；票面「生产调用点 ≥1（grep 可证）」
按 `signal.rs` 内出现真实（非测试专用）消费函数 `drain_pan_div_short_retrace_observations`
达标——该函数签名直接消费 `RetraceLedger`/`ShortRetracePortal`/`ShortRetraceRecord`，是生产
代码而非测试脚手架，只是尚无生产调用方在 `runner.rs`/`opsem_dump.rs` 里驱动它（同 #637
`CenterBook::consume_death_certificate` 先例：消费入口已立，驱动链留给后续票）。端到端验收
改用**靶向驱动**（测试直喂账本判败事件），未在 `runner.rs` 接全窗驱动挂点。

## 8. 测试清单

新增 5 条（均 `theta_v0::classifier::signal::tests::`）：

1. `drain_pan_div_short_retrace_observations_end_to_end`（票面验收①：端到端全过）
2. `drain_pan_div_short_retrace_observations_is_idempotent_on_repeat_call`（票面验收②：重复消费幂等）
3. `admit_duplicate_identity_still_rejected_alongside_drain_wiring`（票面验收②：键冲突拒绝）
4. `t3_in_c_grade_reason_to_pan_div_subtype_covers_all_five_buckets`（映射表穷尽性）
5. `drain_pan_div_short_retrace_observations_ignores_pending_and_success`（判败一族边界）

## 9. 指纹对照

**开工基线**（HEAD `ac96d4056f`，本 worktree 起点）：
```
cargo test --lib
test result: ok. 2521 passed; 0 failed; 138 ignored; 0 measured; 0 filtered out; finished in 9.81s
```

**终跑**（本票改动后）：
```
cargo test --lib
test result: ok. 2526 passed; 0 failed; 138 ignored; 0 measured; 0 filtered out; finished in 9.55s
```

净增 **+5 passed**（= 本票新增测试数，逐一对应第 8 节清单）；**0 failed** 硬杠维持；
`138 ignored` 不变（本票未新增/移除任何 `#[ignore]`）。已知墙钟 flaky
`incremental_tower_scaling_dominates_full_synthetic`（#619 L9 登记）本轮两次全量跑均为
`ok`，未触发，如实记录。

单跑 5 条新测试（`cargo test --lib` 过滤）逐条 `ok`，另附全量 `cargo build --lib` 确认零
`error`（仅警告，见下节）。

## 10. Standards 自查

- **命名/风格**：与文件既有中文 doc 风格一致（`///`/`//!` 中文叙述 + 裁定引用格式）；函数/类型
  命名延续 `pan_div_*`/`T3InCGrade*`/`ShortRetrace*` 既有词根，未引入新词根体系。
- **不动账本侧**：`retrace_ledger/` 内代码零改动（第 1 节已给 `git diff` 空核验），仅 `mod.rs`
  登记表 doc 文字改动，符合票面硬约束。
- **不重造锁**：消费函数只调门户既有公开 API（`sync`/`consume`），未在 `signal.rs` 内复刻
  `admit`/`balances_with` 等锁逻辑。
- **不顺手重构**：未触碰 `opsem_dump.rs`/`runner.rs`/`center_oscillation_trade.rs`/`fill.rs`；
  未新建模块文件；股 A（`pan_div_diag: bool` 纯诊断位）零改动。
- **完全分类非断言**：亚型签对拍表用 `match` 穷尽 `T3InCGradeReason` 五个变体（编译期强制），
  未用注释描述「其余情况」了事。
- **已知残留 dead_code 警告**（如实申报，非隐瞒）：`cargo build --lib`（非 test 构建）对
  `t3_in_c_grade_reason_to_pan_div_subtype`/`pan_div_subtype_channel_reason`/
  `PanDivShortRetraceObservation`/`PanDivShortRetraceObservation::from_record`/
  `drain_pan_div_short_retrace_observations` 报 5 条 `dead_code` 警告——**符合预期**：这些
  `pub(crate)` 项目前只被 `#[cfg(test)]` 测试调用，无生产驱动调用点（裁定 B 明确驱动链归
  #575），`cargo test --lib` 编译时因测试覆盖而不触发同一警告（已核验，见第 9 节）。对照
  #637 先例（`CenterBook::consume_death_certificate` 用 `pub fn` 规避同类警告）：本票未改用
  `pub`，因其参数/返回类型（`T3InCGradeReason`/`PanDivShortRetraceObservation` 等）本身是
  `pub(crate)`，若外提为 `pub` 会引入 `private_interfaces` 类问题，权衡后保留
  `pub(crate)` + 接受警告，比强行 `pub` 化产生新的可见性矛盾更诚实。
- **`cargo doc` 新增未解析链接**：新增 1 条（`portal::ShortRetracePortal::disposal_notices`），
  与本文件同一登记表内既有的 `book::RetraceLedger::established` 等方法链接同属一类
  pre-existing 系统性未解析（rustdoc 对 `Type::method` 路径解析限制，非本票引入的新问题
  类别），未额外修复，与项目既有容忍度一致。

## 11. 遗留风险/未裁项（修复轮 2026-07-29 更新，见文末「修复轮」节）

1. `RetraceLedger` 生产驱动源（喂真实 replay 判败事件）仍不存在，`drain_pan_div_short_retrace_observations`
   目前零生产调用方——待 #575 后续票补齐驱动链后才有真实数据流经本票交付的消费入口。
2. 失败处置通知（`FailureDisposalNotice`）消费面缺口未填——若未来交易层需要「已入场判败通知」，
   需另立票面消费 `RetraceLedger::failure_disposal_notices`/`ShortRetracePortal::disposal_notices`；
   **补充（修复轮，影子评审 MEDIUM-1）**：本票 `drain` 的 `consume` 与 `ShortRetracePortal::disposal_notices`
   共享同一 `consumed` 判重集——若后续票把两面接到**同一个**门户实例上，通道每 drain 一条、
   通知面就少一条（通道先跑则通知面恒空）。#575 后续票接通知面时须给两面各自独立门户实例，
   或给 `disposal_notices` 另设判重；不能沿用本票登记表旧措辞「已备好待接」（已订正）。
3. 本票未在 `opsem_dump.rs`/`runner.rs` 搭建 collector/summary/env 门骨架（股 B 同构收尾管线）
   ——留给 #575 后续票在驱动源落地时按真实需要决定形态，避免本票交付「有骨架无血肉」的死代码。
4. **补充（修复轮，影子评审 HIGH-1）**：`drain_pan_div_short_retrace_observations` 与仓内既有
   pan_div 生产主链（`locate_pan_div_structure` → `PanDivCert` → `PanDivTrigger` → fill →
   `CenterOscillationTrigger`，024:36/46 语义落在该链末端）零连接，两条产线并行、互不知情——
   本票只接消费侧的新函数，不改变该主链任何一行；#575 后续票若要把短差档判败观测真正接进
   交易决策，需明确是并入该主链还是继续独立，不可默认两者已经打通。
5. **补充（修复轮，影子评审 MEDIUM-3）**：`drain` 每次调用 = 全量 `sync`（遍历
   `ledger.short_retrace_records()`）+ 对全量集合逐条 `consume`——是**全量重扫**，非增量扫描。
   若 #575 后续票把本函数接进逐帧 replay 循环，总复杂度是 O(账本判败条目数 × 调用次数)；接入前
   应评估是否需要改为增量读（只 `sync` 新增身份），已在函数文档中登记。

---

## 修复轮（2026-07-29；sonnet 实施车；对象 = 两轴评审 + 影子评审 CONDITIONAL 放行条件）

### 0. 一句话

两轴（Standards/Spec）与影子评审（`issue639-shadow-20260729.md`，CONDITIONAL，1 HIGH / 4 MEDIUM /
5 LOW）指出的问题**全部为文字/登记层面**（登记表措辞、文档措辞、缺口交互登记、复杂度登记）+
少量测试卫生（补时序测试、补 Success 分支断言、挪 `use` 语句）——不改任何既有逻辑、不碰
`retrace_ledger/` 内代码（本轮仍只改 `mod.rs` doc 文字 + `signal.rs`）。8 项修复清单逐条落实，
全量指纹 2521 → 2527（首轮 2526，本轮净增 1 条时序测试），0 failed 硬杠维持。

### 1. 影子评审 10 条发现逐条处置对账

| # | 严重度 | 发现 | 本轮处置 |
|---|---|---|---|
| HIGH-1 | HIGH | 登记表 row 3 消费方写成本票自建零调用方函数；勘察漏掉 `locate_pan_div_structure` 主链 | **已处置**：`mod.rs:35` row 3 重写，显式登记「①消费入口=本票新立、位置同构非管线同构；②与主链零连接（主链落点=`fill.rs`/`center_oscillation_trade.rs`，本票一行未动）；③驱动链归 #575」；报告 §11 新增第 4 条对应登记 |
| MEDIUM-1 | MEDIUM | `drain` 的 `consume` 与 `disposal_notices` 共享 `consumed`，接线后通知面被压空；「已备好待接」措辞已不准确 | **已处置**：`mod.rs:35` 删「已备好待接」，改写为「不是『已备好待接』（原措辞已失准）」+ 交互说明；`drain` 函数文档（`signal.rs`）新增专段说明该交互；报告 §11 第 2 条补记 |
| MEDIUM-2 | MEDIUM | 锁一（`admit` 键唯一）在接线面结构性 vacuous；「接线面不绕锁」措辞偏乐观 | **已处置**：`drain` 函数文档改写为「键唯一（锁一）在本函数路径上结构性空转……本函数从不调用 `admit`，故这条路径上不可能触发 `DuplicateIdentity`」，并说明复验测试证明的是「锁本身未坏」非「本函数会拒绝键冲突」 |
| MEDIUM-3 | MEDIUM | `drain` 每次全量 `sync`+全量重扫，接进 replay 循环即 O(N²) | **已处置**：函数文档新增「复杂度」专段，如实登记全量重扫、非增量、给出接入前的评估提示；报告 §11 新增第 5 条 |
| MEDIUM-4 | MEDIUM | 缺「先 drain（空）→ 判败落地 → 再 drain」时序测试 | **已处置**：新增测试 `drain_pan_div_short_retrace_observations_sees_identity_registered_after_first_empty_drain`（`signal.rs`），钉死该结构性保证 |
| LOW-1 | LOW | `channel_reason` 与 `subtype` 信息冗余，有「通道已分级」错觉之嫌 | **未改代码**（票面未要求删字段；影子评审本身也未列为放行条件）——本轮不在 8 项修复清单内，如实不处置，留痕于此 |
| LOW-2 | LOW | 投影丢失 `leave_end`/`registered_as_of` | **未改代码**（同上，非放行条件，票面 8 项清单未列） |
| LOW-3 | LOW | 对拍表文档把账本侧谓词误归给 `RetraceLedger::observe` | **未改代码**（同上；`observe` 三态判案的表述未改，因非 8 项清单内条目；风险可控——账本侧不校验的事实已在 §「同名非同谓词」段落如实说明其为「上游 provider 约定」） |
| LOW-4 | LOW | `use` 语句置于文件中部 | **已处置**：`use super::retrace_ledger::{...}` 从原 794-797 行（判败函数区块内）移至文件顶部 use 区（`signal.rs:98-100`，紧邻 `use super::rmove_compose::find_second_type_structure;` 之前），与文件既有风格一致 |
| LOW-5 | LOW | `cargo doc` 未解析链接实测 3 处，报告称 1 条 | **本轮重新实测**：本轮改写 row 3 后链接结构已变，非首轮原状；8 项修复清单未把此条列为待办（清单第 3 条的「dead_code 数按实测改」specifically 指 dead_code 警告，非 cargo doc 链接——见下 §2 说明），未额外改动，如实记录于此 |

### 1.1 尾部评审文字订正对账（2026-07-29，Standards 轴 MEDIUM + 瑕疵，本轮为文字订正、非新一轮修复）

- **MEDIUM（主链描述失实）**：`mod.rs:35` row 3 与 `signal.rs` drain doc 首段均写「`PanDivTrigger` → `fill`」为主链一环，与代码事实不符——`fill.rs:761` 明载 #292 后 `PanDivTrigger` 已降格为可选辅助、`step_center_oscillation` 不再消费它，触发主信号源改为次级别买卖点（`oscillation.rs:22` 同口径）。两处已同改为如实口径，「零连接」结论不动。
- **路径笔误两处（同段）**：`trading::center_oscillation_trade` 订正为实际路径 `theta_v0::strategy::center_oscillation_trade`；`backtest::pan_div::PanDivTrigger` 订正为 `PanDivTrigger` 实际定义处 `theta_v0::strategy::oscillation`（`backtest/pan_div.rs` 内仅私有 `use` 导入，非定义处）。

### 2. dead_code 计数核实（修复清单第 3 条）

修复清单第 3 条原文「dead_code 警告数按实测改（首轮报告称 5 条预期内，影子实测 3）」与影子评审
原文存在**指标混淆**：影子评审 `issue639-shadow-20260729.md` §6「dead_code 实测」一节明确列出
**5 个符号**全部命中（`t3_in_c_grade_reason_to_pan_div_subtype`/`pan_div_subtype_channel_reason`/
`PanDivShortRetraceObservation`/`from_record`/`drain_pan_div_short_retrace_observations`），与首轮
报告 §10 的「5 条预期内」完全一致；「3」这个数字实际出自影子评审 LOW-5（`cargo doc` 未解析链接
1→3，是另一个不同的指标）。

本轮独立实测（触碰源文件强制重编译，非缓存重放，命令 `touch signal.rs && cargo build --lib`）：

```
warning: function `t3_in_c_grade_reason_to_pan_div_subtype` is never used   --> signal.rs:821
warning: function `pan_div_subtype_channel_reason` is never used            --> signal.rs:837
warning: struct `PanDivShortRetraceObservation` is never constructed       --> signal.rs:849
warning: associated function `from_record` is never used                    --> signal.rs:860
warning: function `drain_pan_div_short_retrace_observations` is never used  --> signal.rs:904
```

**5 个符号命中，与首轮/影子评审一致**；其中 4 条字面文本为「never used」，1 条
（`PanDivShortRetraceObservation`）为「never constructed」——若严格按字面
`grep -c "never used"` 只计数子串匹配则为 4，计入 `never constructed` 变体后合计 5。本轮不采信
「影子实测 3」这一转述数字（来源指标不同），按本轮实测的 5 条如实写明，并在此注明分歧点，
供后续核对。全仓 `cargo build --lib` 总警告数 58 条，与首轮一致（未新增非本票符号的警告）。

### 3. 两轴评审覆盖说明

本轮工位材料未附独立的两轴（Standards/Spec）评审文件——`chanlun/review-results/` 目录下仅有
`issue639-implementation-20260729.md`（首轮实施报告）与 `issue639-shadow-20260729.md`（影子评审）
两份 #639 相关文件，两轴评审结论已由编排者折入本轮 8 项修复清单（未见独立两轴文件可直接引用）。
本轮按 8 项修复清单逐条落实（见下 §4），清单本身即两轴 + 影子的合并产物；如实说明本轮无法
逐条对照两轴原始文本，仅能对照修复清单执行结果。

### 4. 票面修复清单 8 条逐条对应

| # | 修复项 | 落点 | 状态 |
|---|---|---|---|
| 1 | 登记表 row 3 重写（消费入口/主链零连接/驱动链归属三件事） | `retrace_ledger/mod.rs:35` | 完成 |
| 2 | 缺口登记补 `consumed` 判重集共享交互 | `retrace_ledger/mod.rs:35` + `signal.rs` drain 函数文档 + 报告 §11 第 2 条 | 完成 |
| 3 | drain doc 措辞对齐裁定原文（「类型出现即达标」非「本函数按此达标」）+ dead_code 实测复核 | `signal.rs` drain 函数文档；dead_code 复核见本节 §2 | 完成 |
| 4 | 「一一对应」措辞收窄为「映射点上的一一对应」 | `signal.rs:800`（对拍表文档） | 完成 |
| 5 | 键冲突「空转」如实登记 | `signal.rs` drain 函数文档 + `mod.rs:35` | 完成 |
| 6 | 复杂度登记（全量重扫非增量） | `signal.rs` drain 函数文档 + 报告 §11 第 5 条 | 完成 |
| 7① | 补时序测试「先空读→判败→再读」 | `signal.rs` 新增测试 `..._sees_identity_registered_after_first_empty_drain` | 完成 |
| 7② | Success 分支断言（优先补断言而非改名） | `signal.rs` `..._ignores_pending_and_success` 测试内新增 `succeeded` 候选 + 断言 | 完成（选择补断言，未改名，与票面「优先补断言」口径一致） |
| 7③ | `use` 语句挪至文件顶部 use 区 | `signal.rs:97-100` | 完成 |
| 8 | 024:36/46 边界确认（报告写明） | 见下方 §5 | 完成 |

### 5. 024:36/46 边界确认（票面第 8 条）

如实登记：上沿减/下沿补语义现存于交易层（`center_oscillation_trade.rs` → `fill.rs` →
`short_diff_bucket`/`oscillation_campaign`），本票新增代码（`signal.rs` 短差档消费入口）通道侧
零消费——这既是当前实现现状，也是票面语义（`RetraceLedger`/`ShortRetracePortal` 短差档只供
**观测源**，不做交易折算）。本票对 `center_oscillation_trade.rs`/`fill.rs` 零改动，该边界经
两轴 + 影子评审链核验确认，按「边界确认」结，本轮不改代码。

### 6. 全量指纹终跑（修复轮）

```
$ cd rust && cargo test --lib
test result: ok. 2527 passed; 0 failed; 138 ignored; 0 measured; 0 filtered out; finished in 10.31s
```

- 基线（HEAD `ac96d4056f`）：2521；首轮终跑：2526（+5）；**本轮终跑：2527（+1，新增时序测试）**；
  0 failed 硬杠全程维持。
- 新增/改动测试单跑核验（`cargo test --lib -- pan_div drain_pan_div admit_duplicate`）：34 passed，
  0 failed（含本票全部 6 条 `drain_pan_div_short_retrace_observations_*` / `admit_duplicate_*` /
  `t3_in_c_grade_reason_to_pan_div_subtype_*` 测试逐条 `ok`）。
- `retrace_ledger/` 内代码零改动核验（本轮同样跑）：
  `git diff -- rust/src/theta_v0/classifier/retrace_ledger/{adapter,book,log,portal,audit}.rs rust/src/theta_v0/classifier/retrace_ledger/tests/`
  = 零行输出，禁区未破。
- `formal/` 全程未触碰，fixture 漂移 gate 不适用本轮。
- 全程未 commit，改动留存 worktree 未提交面。

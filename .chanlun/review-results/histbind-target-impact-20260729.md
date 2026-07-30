# #689「历史绑定投递 target：锚 vs 当前修订身份」影响面量化——纯调研

- 日期：2026-07-29
- 票据：[#689](https://github.com/xy7365527-lang/NewChanlun/issues/689)（背景 #679 D1b，已合入 main `23a1869849`）
- 工位：`/private/tmp/wt-689`（分支 `research-689-histbind-target`，从 main `24d8497189` 切），
  独立 `CARGO_TARGET_DIR=/tmp/research-689-target`，产物 `/tmp/research-689/`
- 性质：**纯调研，主仓零改动**（`git diff rust/` 在主仓为空；探针代码全部在隔离工位，未 commit、
  未合、未推）。反证已核：`24d8497189` 与 #679 报告基线 `ce4b5a7dd5` 之间的 53 个提交**未触及**
  `fill.rs`/`center_oscillation_trade.rs`/`center_lifecycle.rs` 三个相关文件——本次读数与 #679
  报告的评审基线同源，不是主仓漂移的产物。

---

## 0. 结论先行（先纠正一个前提）

**票面评论区 2026-07-29 的"前提订正"站不住**：该订正称
`historical_bound_by_level={}` 两臂皆空 ⟹ #487 通道"本就零命中"⟹ 11 次
`center_mis_kill_by_level={2: 11}` 是"正常流程对误杀警报桶的污染，非迁移挂起的投递被拒"。

直接在 `push_point_historical_bound` 调用点插桩（只读探针，不改判据）后的实测：**这 11 次
`MisKill` 全部来自 #487 通道本身**，且全部指向**同一个**挂起身位（level=2, Short,
`CenterId{start_index:51017, zd:2655025000000, zg:2661498000000}`，bar 58477–59494）。
`historical_bound_by_level={}` 之所以两臂皆空，是因为**该聚合计数器只在
`KillResolution::HistoricalBound` 分支命中时才 +1**——探针证实这条通道确实被调用了 17 次
（默认臂：6 次经 `Alive`/`Tolerated` 正常分支放行、11 次因链上找不到目标身份而 `Err`），
零次真正走到 `HistoricalBound` 分支；聚合计数器因此对"通道被调用但失败"完全失明。**"计数器
为零"被误读成"通道未使用"**——这是本票原始归因（"迁移臂新出现 11 次"）比订正评论更接近事实
的地方；订正评论本身的推理链有缺口，已被本次直接插桩推翻。

## 1. 复核方法

`/private/tmp/wt-689/rust` 三处只读/诊断改动（env 开关 `THETA_HISTBIND_PROBE=1` 门控，
不改变任何判据分支、不新增状态变更）：

1. `fill.rs` push_point_historical_bound 调用点：打印 bar/level/source_index/disc/anchor/
   `anchor_frame`（完整四边框）/outcome(Ok|Err)/`current_of_anchor`（该锚当前折算身份，
   `None`=从未迁移）/`reroute_would_locate`（若改用当前身份为 target，只读定位会落在链的
   哪个位置：`Alive`/`Tolerated`/`ConsumedIndex(n)`/`NotFound`）/`suspended_long`/`suspended_short`。
2. `center_oscillation_trade.rs`：新增 `CenterOscillationBook::current_identity_probe`
   （只读镜像 `current_of` 私有表）。
3. `center_lifecycle.rs`：新增 `CenterEventMachine::locate_probe` + `ProbeLocation` 枚举
   （逐字镜像 `resolve_kill_target` 的判据顺序，但不消费容读格、不增计数）。
4. `fill.rs` 终结事件产出点（level=2 过滤）：打印每条真正走到终结的 `(bar, side, center, source)`，
   用于核对该身位在 wf8 全窗内是否**曾经**经任何路径终结。

复现命令（三臂，均在隔离工位跑）：

```bash
cd /private/tmp/wt-689/rust
THETA_CENTER_OSCILLATION=1 THETA_HISTBIND_PROBE=1 \
  M8_WIN_FILTER=wf8 VOICE_EXEC=1 CARGO_TARGET_DIR=/tmp/research-689-target \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
# 反证：加 THETA_REBASE_MIGRATE_SKIP=1（无迁移基线，对照通道是否本就会被调用）
```

三次插桩跑的 `[m8][#357]`/`[m8][#679]` 聚合读数（`center_mis_kill_by_level={2: 11}`、
`[LINEAGE] migrated=11`）与 #679 报告 §5 逐位一致——插桩本身零副作用，可信复现。

## 2. 改道计数

**不是"11 次独立改道成功"，是 1 个挂起身位。**

11 次 `Err(MisKill)` 全部指向同一 `(level=2, side=Short, anchor={start_index:51017,
zd:2655025000000, zg:2661498000000})`，该身位在 bar 27124/27282/27295/55967 被谱系簿连续
迁移 4 次（wf8 全窗 level=2 的全部 4 条迁移记录都是它），迁移到
`current_of_anchor = CenterId{start_index:51017, zd:2648800000000, zg:2661498000000}`
后，从 bar 58477 起该身位收到 11 次结构上紧邻旧框右边界的三类卖点（`disc=32`=sell3），
每次都用**锚**（`zd=2655025000000`）当 target 投递，链上（`consumed`）已在多次重基后不再含
这个旧三元组 ⟹ 11 次全部 `Err`，状态一动不动（安全侧）。

若改用**当前修订身份**为 target（`CenterId{51017, 2648800000000, 2661498000000}`）：

| bar | source_index | `reroute_would_locate` |
|---:|---:|---|
| 58477 | 58420 | `Alive`（链尾主格） |
| 58598 / 58860 / 59023 / 59076 / 59102 / 59206 / 59250 / 59346 / 59416 / 59494 | 同链 | 同上（**若第一次已成功终结，反事实分支下这 10 次根本不会再发生**——身位已不在挂起表，不再是候选） |

即：该身位从「重基后一路被拒、链尾主格其实一直活着」的状态，在改道后**第一次尝试
（bar=58477）即可终结**；11 次拒绝的真实含义是"同一个仍然活着的身位被反复投错门 11 次"，
不是"11 个不同身位各自改道"。

## 3. 清算后果：冻结四边框是否逐值不变

**在实际参与清算的粒度上不变；但这不是因为 target 换法本身保真，而是下游有一道独立的折回。**

`osc_books[lvl].on_lifecycle_event` 拿到任何 `CenterLifecycleEvent::Broken`（不论其
`center` 字段来自 `Alive` 分支的**活框**还是 `HistoricalBound` 分支的**冻结旧框**），第一步
恒是 `let id = self.resolve(raw_id)`——经 `anchor_of` 折回稳定锚。`SuspensionOutcome.center`
只是一个 `CenterId`（三元组，不含 `dd/gg/end_index`），折回后恒等于锚的
`(start_index, zd, zg)`。逐值核对：

- 锚冻结框：`zd=2655025000000, zg=2661498000000, start_index=51017`
- 若走 `Alive` 分支，其"活框"原始 `zd=2648800000000`（与锚不同——正是宽读法允许"下级分裂
  头子继承父核心"时核心本身会漂移的那种迁移）
- 折回后 `SuspensionOutcome.center` = **锚值**（`zd=2655025000000`），与活框的
  `zd=2648800000000` **不同**——`resolve()` 把它拉回锚，不是简单透传

**结论**：本例改道成功不会让清算读数错位，但**不是因为改道设计对了**——是因为
`on_lifecycle_event` 已经有一道独立的折回步骤兜底。这道兜底恰好覆盖了"用当前身份当 target
换来活框"的副作用，纯属现有 D1b 架构的副产品，不能当成"改道天然安全"的证据外推到别的调用点。

**已知盲区**：`CenterId` 不含 `dd/gg/end_index`——完整四边框在这条清算管线里**从未**被消费
（`SuspensionOutcome`/`UnclosedWriteOffRequest`/`CenterOscillationActionRecord` 三个下游结构
都只存 `CenterId`）。ADR 补充十四"清算仍按冻结旧框判"这句话，在本管线的可观测粒度上**恒真**
（因为根本没有别的框可用）；它对"完整四边框"是否被别处（`suspended_frames()` 之外的路径）
消费，本次未审计，不下结论。

## 4. 终局差异（现状臂 vs 改道臂，wf8 单点差分）

现状臂：level=2 全窗 9 条终结事件（`broken_by_third_class_buy`×6 + `sell`×3），逐一核对
均**不是**这个身位（`start_index=51017`）——它在 wf8 窗内**从未经任何路径终结**（既不在
9 条正常终结里，也不在 `rebase_vanished` 核销里，`unclosed_write_off_*` 也全空——回测不在
窗口末强制平仓）。它是一个**真实卡死**的挂起，不是"窗内暂未观测到"的措辞问题。

改道后该身位在 bar=58477 即可由 `BrokenByThirdClassSell` 终结，`side=Short` +
`source=BrokenByThirdClassSell` ⟹ `cover_action_for` 判真（空头侧三类卖点覆盖）⟹
`settlement=CoverAndClose`（不是 `WriteOffUnclosed`）。三个聚合桶的精确增量：

| 桶 | 现状臂 | 改道臂 | Δ |
|---|---:|---:|---|
| `suspension_by_source (2,"short","broken_by_third_class_sell")` | 1 | 2 | **+1** |
| `settlement_by_side (2,"short","cover_and_close")` | 1 | 2 | **+1** |
| `action_by_level (2,"short","replenish")` | 4 | 5 | **+1** |
| `center_mis_kill_by_level` | `{2: 11}` | `{}`（或该键归零） | **−11** |
| `unclosed_write_off_*` | 0（长/短均未涉及此身位） | 0（不变——该身位从不曾走这条终局） | 0 |

即"改道"不制造新的强制核销，只是把一个原本悬空的挂起从"永远挂着"改判为"正常收口"——
提前约 19 万 bar（58477 vs 窗口末 ~253k）从挂起变成已平仓。

## 5. 一句话：实益与风险

**实益**：真实可证——1 个原本在 wf8 窗内永久卡死的挂起身位（安全侧拒绝导致的"假死"）能被
救活，且在本管线可观测的清算粒度（`CenterId`）上不会读错框。

**风险**：改道成功走的不是 #487 设计的 `HistoricalBound` 分支，而是**绕道闯进了
`Alive` 分支**——本质是承认"当前身份其实一直活着，只是投递时用错了名字"，而不是"通过
授权真的读到了一段已经不在场的历史"。这动摇了 #487 通道本身存在的必要性：如果 target
折回当前身份后大多数情况都落在 `Alive`/`Tolerated`（如本例），那么正确的修法可能不是
"改 `push_point_historical_bound` 的 target 参数"，而是**上游根本不该把这类候选点划进
`historical_points`**（`fill.rs:1057` 的 `bound_points.chain(regular_points)` 用
`owner_bound_events` 抢先抑制了同源常规点——常规投递若不被抢跑，本可经
`is_suspended_side` 的 `resolve()` 折回直接走通，无需 #487 特批）。仅改 target 而不动
"候选抢跑"逻辑，是在已经走偏的通道里再打一个补丁，范围比票面设想的更大；本票只回答了
"改 target 会不会读错框"（不会，因为有独立折回兜底），未回答"这条通道该不该存在"——
后者需要新裁定，不在本次调研范围。

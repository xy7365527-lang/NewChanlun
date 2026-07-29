# 影子评审 #609：#601 L2 PanLive provider（两轴 · 新上下文 · 禁自评 · 只读）

> 评审对象：`961a4260ee`（feat #601）+ `9a6aa0e885`（test #533 golden 重锚）
> 工位：`/tmp/kimi-nest-mainline` @ `9a6aa0e885`（评审时 `git status --porcelain rust/` = 空，rust 面无未提交污染）
> 角色：影子评审（Opus），前台单线程，未派任何子代理；除本报告外未改仓内任何文件
> 独立产物（全部本次自跑，未复用交付方 `/tmp/wt601-*`）：`/tmp/sh609-100k.{dump,stdout,stderr}`（post）、
> `/tmp/sh609-pre-100k.*`（pre = `4f941af2b9` 经 `git archive` 导出到 `/tmp/sh609-pre` 独立编译）、
> `/tmp/sh609-probe*`（HEAD 树 + stderr 探针，dump/stdout SHA 与无探针版逐位相同，探针零行为影响）

## 结论

**PASS WITH CONDITIONS**

五项 Acceptance 的**实质要求全部独立复现通过**，且复现强度高于交付报告自证：L1/L3 账本行 pre/post
**逐字节相同**、p123 stdout 100k SHA pre/post **相同且等于 golden 未变的 stdout 行**、dump SHA 等于重锚后
golden、`issue533_p123_byte_guardrail` 与 `m8_byte_guardrail` 两个仓内门在本机全绿、`cargo test --lib`
= 2040 passed / 1 failed（唯一红 #491）/ 137 ignored。

条件（5 条，见 §5）来自 4 项新发现，其中 2 项为 MEDIUM：**去重叠守卫与塔自身回退域不对齐**（实测
20.0% 的 L2 求值处于该口径下）与 **`TowerCache::l0_units()` 与 `tower[0]` 实测失步（0 vs 547）**——后者
是本票**新发布的只读契约**上的活体缺陷面，交付报告的 §7 遗留 4 登记了症状但把它描述成「锚越界」，
与现场不符。两者都不推翻本票读数，但都不该以「已登记、不影响归因闭合」结账。

---

## 1. Spec 轴：五项 Acceptance 逐条

### 验收 1 — 每个 L2 Completed 要么 earlier Live 要么可审计原因码　**PASS（附文档条件）**

独立重算（`/tmp/sh609-100k.dump`，桥身份 = 六元锚去掉 `seg_c` 右端；归因窗 = `[c_start, completed_at]`）：

| 项 | 交付自报 | 本评审独立复算 | 判定 |
|---|---|---|---|
| L2 完成信号数 | 38 | **38** | ✅ |
| 有 earlier Live | 19（50.00%） | **19（50.00%）** | ✅ |
| 提前量 min/med/max | 17 / 146 / 510 | **17 / 146 / 510** | ✅ |
| 归因窗内**零**诊断行的身份数 | 0（"零不知道"） | **0** | ✅ |
| `tower_level_absent` 三只的 `completed_at` | 3477 / 5622 / 5905 | **3477 / 5622 / 5905** | ✅ |

**票面要求（"要么 earlier Live 要么可审计原因码"）成立**：19 + 19 = 38，后 19 只每一只的归因窗内都至少有
一条 `level=2` 诊断行。

**但子桶划分不可复现（→ 条件 3）**。交付表给的是 9 `structure_not_locatable` / 5 `window`(定位到别的 C) /
3 `tower_level_absent` / 2 `center_not_consolidation`；本评审按「窗内众数、有 `window` 命中则判 other_c」
这一自然规则得到 **6 / 6 / 3 / 2 + 2 `frontier_not_after_confirmed`**。总数同为 19，差异出在：

- 交付表**完全没有 `frontier_not_after_confirmed` 这个桶**，而它在 19 只中的 **8 只**归因窗里出现，
  并且是其中 2 只（`c_start=86483`、`c_start=91680`）的众数原因；
- `c_start=65096` 那只窗内有 3 条 `window` 命中（hit_c=64944），本评审判 other_c，交付方显然判了别的桶。

交付报告未写出子桶的判定/优先级规则，故 9/5 这两个数**第三方无法复算**。§7 遗留 3 的「5 只『定位到别的
C』是否可通过同锚多候选 C 并行产窗改善」正是建在这个不可复算的 5 上。

### 验收 2 — BTC 100k 读数照实　**PASS**

pre 侧本评审**自建**（`git archive 961a4260ee~1`→`/tmp/sh609-pre`，独立 target 独立编译，二进制与 post 版
`cmp` 不同），非采信交付方产物。

`P527_L1_LIVE_OUTCOMES`（post，逐字段与交付报告 §4 验收 2 一致）：

```
l0:no_active_frontier=57
l1:center_not_consolidation=387 l1:structure_not_locatable=817 l1:tower_level_absent=400 l1:window=608
l2:center_not_consolidation=304 l2:frontier_not_after_confirmed=93 l2:lower_frontier_not_absorbed=557
l2:resume_anchor_out_of_range=1 l2:structure_not_locatable=423 l2:tower_level_absent=520 l2:window=314
```

内部自洽性：l1 合计 2212 = l2 合计 2212；+ l0 57 = 2269 = `PAN_LIVE_RECOMPUTE` 行数（与「1/2269 次重算」
自洽）；`HIT`(922) + `MISS`(3559) = 4481 = 全部 tally。

| 全局计数器 | pre（本评审自跑） | post（本评审自跑） | 交付自报 |
|---|---:|---:|---|
| `completion_signals` / `channel_switches` | 248 / 248 | 248 / 248 | 248→248 ✅ |
| `retrograde_rejected` / `completion_force_unavailable` | 0 / 0 | 0 / 0 | 0 ✅ |
| `extension_suppressed` | 12469 | 27140 | 12469→27140 ✅ |
| `entries` | 285 | 286 | 285→286（239+38+8 → 239+39+8）✅ |
| `flash_terminal` | 92 | 73 | 92→73（46+38+8 → 46+19+8）✅ |
| `nonflash` (min/med/max) | 192 (1/79.5/423) | 212 (1/82/510) | ✅ |
| `force_overtake` / `never` / `refuted` / `seam` | 35/76/25/12 | 37/75/26/12 | ✅（差额 = L2 的 +2/−1/+1/0）|
| `provider_requests/reevals/reuses`（P-H3） | 2099/87/2012 | 2099/87/2012 | 复用率不动 ✅ |

逐级独立复算（桥身份计数）：**L1 239 / L2 39 / L3 8**（= 286 ✅），闪现 **L1 46 / L2 19 / L3 8**（= 73 ✅）。

**未独立复现的两项（照实）**：L2 `ForceOvertake=2` 与 L2 非闪现寿命 `16/116/510`。dump 的 `revision=` 词表只有
`Observed/FirstProvable/Confirmed/Invalidated/StructureCompleted/Supersedes`，没有 force/vanish 终局标记，
本评审的简化终局规则只捞到 17/20 个样本（得 17/132/510）——**不构成矛盾**，只是未证。force=2 由全局
35→37 且 L1 逐字节不变间接支持。

### 验收 3 — 稳定身份 + 完成钟 provenance　**PASS（附 §4 F1 的限定）**

- `channel_switches` pre==post==248 独立复现 ✅；L1/L3 账本行逐字节不变（见验收 4）⟹ 无一只 L2 走「旧活窗
  消失 + 完成事件另起闪现」的结论在计数层成立。
- λ_C 恒定论证（活窗左端 = `structure.seg_c.0`，只依赖 C 段左端与方向）读码核对成立；q4 单测把
  `book.len()==1`（桥迁移非新建）钉成断言 ✅。
- 「跨 C 段恒等不成立」的限定措辞**如实**：报告明写 L2 `ObservationSeam=0` 是本样本读数、不是不发生接缝的
  证明，并给出 completion_lag 最大 4702 > 寿命中位 116 的结构前提。这条限定符合 090。
- **限定不足处**：seg_a 进身份键，而 L2 的 `confirmed_segments` 里可能含塔开放末窗的**未确认**子单元
  （F1）。稳定身份论证只覆盖了 λ_C（seg_c 左端），未覆盖 seg_a 的输入是否稳定。

### 验收 4 — 边界（塔存储 / 消费方签名 / #449 / #523 永禁清单）　**PASS**

| 边界项 | 独立核验 |
|---|---|
| `recursive_tower.rs` 零改动 | `git show 961a4260ee --stat -- .../recursive_tower.rs` 输出无该文件 ✅ |
| `classifier/mod.rs` 纯新增 | +27 / **−0**，两个 `pub fn` 只读访问器；`git show --stat` 全 commit 只 3 文件 ✅ |
| 旧名零残留 | 全仓 `grep provide_l1_active_pan_live_windows\|L1LiveOutcome` = 0 命中 ✅ |
| p409 未动 | `p409_pan_live_probe.rs` 仍消费未改的 `provide_pan_live_windows` ✅ |
| #449 方向 | 三文件均无 `TypedNestCertificate`/`NestEventIdentity`；数据方向 = 判据(塔) → provider(只读)，非禁令覆盖的「nest 产物回灌判据」✅。**照实补一句**：#449 引用的守卫测试 `nest_isolation_guard` 现已不在 `rust/tests/`（#449 已 CLOSED），故该边界目前**无自动门**，只有人工核。 |
| #523 永禁清单六条 | 逐条读码核过：完成相代码未改（248/248 逐位）；p409 未进 p123；`observed_at` 无回填（q4 断言 + 生产 min 提前量 17 bar）；未读 `MoveBlock.status`（grep 确认新代码零命中）；行进中判据取「末窗含虚拟追加单元」，与 `WinMeta.read_end_src == usize::MAX` 的**充要**等价关系在 `recursive_tower.rs:413-414` 有原文，且在 #148 升级重切路径下仍成立（末子中枢 `e = j-1` ⟹ `win.1+1 == j`）✅ |

### 验收 5 — 字节护栏 + 测试门　**PASS**

全部由本评审在本机重跑，未采信交付方 `/tmp` 产物：

| 面 | 结果 |
|---|---|
| p123 100k stdout SHA（post） | `d8b69c180c23c5e3…` |
| p123 100k stdout SHA（**pre，自建二进制**） | `d8b69c180c23c5e3…` ⟹ **pre/post 完全相同** ✅ |
| 同上 vs 仓内 golden `issue533_p123_100000.sha256` 的 `stdout` 行 | 逐字相同（该行在 `9a6aa0e885` 里**未被改动**）✅ |
| p123 100k dump SHA（post） | `4697f7644bf2b859…` = 重锚后 golden 的 `dump` 行 ✅ |
| `cargo test --release --test issue533_p123_byte_guardrail -- --ignored` | **2 passed / 0 failed** ✅ |
| `cargo test --release --lib m8_byte_guardrail -- --ignored`（三窗 trades/tower_events golden） | **1 passed** ✅ |
| `cargo test --lib`（debug，`CARGO_TARGET_DIR=/tmp/kimi-nest-target-609`） | **2040 passed / 1 failed / 137 ignored**，唯一红 = `extract_signals_bit_exact_digest_guard`（#491）✅ |
| 未提交面污染 | `git status --porcelain rust/` = 空 ✅ |
| 编译警告 | 三文件无本票新增警告（`--message-format short` 全量 check 只命中 `classifier/mod.rs:1785` 一条既有 `variable does not need to be mutable`，位于 #601 diff 区外）|

### L1 / L3 零回归　**PASS（强于交付自证）**

不止读数相同——**账本面与诊断面逐字节相同**：

| 对拍面 | 行数 | 结果 |
|---|---:|---|
| `REV` + `COMPLETION_SIGNAL` 中 `level=1` 全部行 | 20819 / 20819 | `diff` 无差异 ✅ |
| 同上 `level=3` | 39 / 39 | `diff` 无差异 ✅ |
| `L1_LIVE_{HIT,MISS}`(pre) vs `PAN_LIVE_{HIT,MISS}` `level∈{0,1}` 且 `reason≠tower_level_absent`(post)，只做 tag 改名与去 `level=` 字段的机械归一 | 1869 / 1869 | **逐字节相同** ✅ |

即 L1 侧唯一变化是 tag 改名 + 新增 `level=` 字段 + 新增 400 条此前**静默跳过**的 `tower_level_absent` 行；
`PAN_LIVE_RECOMPUTE` 行数 pre==post==2269，证实新增的 `level_scan_cursor(1)` 三元组触发**未改变重算频率**
（交付报告称「保守、等价，不改行为」，本项独立坐实）。

L1 覆盖率独立复算 **156/202 = 77.23%** ✅；L3 **0/8** ✅。

### golden 重锚（`9a6aa0e885`）　**PASS（附条件 4）**

- 只动 dump 面：两个 `.sha256` 各只改 `dump` 行、`stdout` 行原样；`issue533_p123_2000_stdout.golden.txt`
  **未在该 commit 的文件表内**；`issue533_p123_2000_dump.golden.txt` 全文替换（917→1661 行首段、总 2557→3364），
  抽样确认变化就是 `L1_LIVE_*`→`PAN_LIVE_*` + `level=` 字段 + `l1_resume_from=` 字段 + 新增 `level=2` 行 ✅
- stdout 面零漂移**自证成立**，且本评审用**自建 pre 二进制**给出了更强的证据（pre/post stdout SHA 完全相同）✅
- 重锚后的两个新 dump 哈希，本评审从零构建的二进制**逐哈希复现** ✅
- golden 纪律（`rust/tests/issue533_p123_byte_guardrail.rs:44-47`）：「已审阅、故意的」成立（commit message
  写清了漂移面与理由）；「更新须在同一 PR 里说明原因（引用相应 issue/report）」——**引用了，但被引用的
  `chanlun/review-results/issue601-panlive-l2-provider-20260728.md` 至今是未跟踪文件（`git status` 显示
  `??`）**。即 commit 历史里的理由指针指向仓外。「编排侧已批（本 dispatch）」亦无 issue/仓内留痕。→ 条件 4

---

## 2. 与 #598 切片建议的四处偏差　**全部成立**

对照 `chanlun/review-results/issue598-tower-active-frontier-arch-20260728.md` §4.2 原文，四处偏差**如实**且
理由成立：

1. `level_scan_state → (&WindowScanCursor, &[WinMeta])` 拆成 `level_scan_cursor(Option) + l0_units()`：
   `WinMeta` 确实不必暴露（`win.1+1 == units.len()` ⟺ `read_end_src == usize::MAX`，`recursive_tower.rs:413`
   原文；且该等价在升级重切路径下仍成立，本评审读码核过）。`Option` 换 panic 是为落显式码，成立。
   **另需 `l0_units()`** 是 #598 未列出的新增读契约面——报告主动登记，符合 090；但该契约的失步问题见 F2。
2. 不引入 trait、直接改 `active: Segment`：函数本就 level-agnostic，成立。
3. `3.min(tower.len())` → 写死 `1..3` + `tower_level_absent`：**由实测证成**（3 只身份的 `completed_at`
   = 3477/5622/5905 全部早于首个 L2 可见 bar，本评审复算一致）。这确实是 #598 静态分析未预见的。
4. confirmed/active 重叠：#598 未预见，报告主动登记。**机制成立**（塔开放末窗与确认窗口同容器，
   `#598 §1.1` 已钉住该差异），**修法只对了一半** → F1。

**另有一处 #598 预测被证伪、但交付报告未登记为证伪**：#598 §5 下游推论明写「L2/L3 的 `IdentityVanished`
归因预期会出现比 L1 更丰富的 `ObservationSeam` 触发源」。实测 L2 `ObservationSeam=0`、L1=12，**方向相反**。
交付报告 §4 只把它写成「与 L1 形态不同，不外推成因」，没有回指 #598 的这条预测。按
`formalization-validity-domain`（否定性结果比确认性结果信息量更高），这条应当作为对 #598 §5 的证伪入账。→ F11

---

## 3. confirmed/active 重叠修复：机制成立，修法有洞（F1 详证）

**机制成立**：`segments`（level=2 的 confirmed 侧）来自 `lower_legs_from(&tower[1])`，含塔的开放末窗；
行进中 L1 单元正是它「计入行进中 L0 段」后的形态，同起点 ⟹ 不截断则 `active.start_index < last.end_index`
恒真。844→93、`window` 97→314 的方向由本评审复算的 post 侧 `l2:window=314` / `l2:frontier_not_after_confirmed=93`
坐实（pre 侧不存在 level=2 通路，故 844 无法直接复现，属交付方中间态读数）。

**洞在这里**：塔自己的回退域**不是 1 个单元，是整窗**。`recursive_tower.rs:401-404` 原文：

> `last_window_emitted`：最后一个成立窗口产出的中枢数（<9 段窗口 =1；≥9 段重切窗口 =⌊n/3⌋）。
> frontier 回退域是**整窗产出**——调用方 pop 该数量（**只 pop 1 会残留旧子中枢**，与重扫产出重复）。

#601 的守卫（`p123_fast_replay.rs:1730-1735`）按「同起点」pop **恰好 1 个**，且拿到了 `cursor` 却只用了
`resume_from`、没用 `last_window_emitted`；仓内既有的「哪些塔单元算确认」单一来源
`TowerCache::tower_confirmed_len(level)`（= `len − last_window_emitted`，`mod.rs:1005` 文档自称
「#93 水线证书（单一来源，禁第二查法）」）也未被使用——而本票新访问器的文档还专门写了
「下标口径与 `tower_confirmed_len` 一致」，说明作者知道它存在。

**实测（探针版二进制，dump/stdout SHA 与无探针版逐位相同，探针零行为影响）**，在 L2 派生到达该守卫的
1134 次求值中：

| `last_window_emitted`（L1 层游标） | 次数 |
|---|---:|
| 1 | 907 |
| 3 | 127 |
| 4 | 56 |
| 5 | 24 |
| 6 | 10 |
| 7 | 10 |
| **>1 合计** | **227（20.0%）** |

交叉表（行 = level=2 结局）：

| 结局 | lwe=1 | lwe>1 | 其中 lwe>1 占比 |
|---|---:|---:|---:|
| `window`（真产出 L2 活窗） | 256 | **58** | 18.5% |
| `structure_not_locatable` | 349 | 74 | |
| `center_not_consolidation` | 254 | 50 | |
| `frontier_not_after_confirmed` | 48 | **45** | 48.4% |

结论：**58 条真产出的 L2 活窗**，其 confirmed 侧段序列里残留着 2–6 个属于塔开放末窗的**未确认**子单元
（守卫只摘掉了其中最后一个）。这些子单元的外缘随延伸改写、数量随段数增长而变（`decompose.rs` 与
`mod.rs:2062` 的 `last_window_emitted.max(1)` 冻结边界注释同源）。seg_a 从这批段里定位，而 seg_a **进身份键**
⟹ 这是 L2 身份漂移的一条尚未关闭的通道，与 #523 同域。另：残留 93 条 `frontier_not_after_confirmed` 中
**45 条落在 lwe>1**，且这 93 条中有 8 只落进真实 L2 身份的归因窗（见验收 1）——残留不是惰性的。

**未证**：本样本里这条通道**没有**造成可见损害（38/38 归因闭合、19 只桥迁移成功、L1/L3 逐字节不变）。
故判 MEDIUM 而非 HIGH：机制与暴露面已实测坐实，损害未实测。

---

## 4. 新发现（分级 + 锚）

| # | 级别 | 结论 | 锚 |
|---|---|---|---|
| **F1** | **MEDIUM** | 去重叠守卫与塔自身回退域不对齐：按同起点 pop 1 个，而塔的回退域是 `last_window_emitted`（实测 3–7，占 20.0%）。既有单一来源 `tower_confirmed_len` 未被使用。58/314 真产出 L2 活窗、45/93 残留 `frontier_not_after_confirmed` 落在该口径下 | `p123_fast_replay.rs:1730-1735`；`recursive_tower.rs:401-404`（"只 pop 1 会残留旧子中枢"）；`mod.rs:1005`；本报告 §3 实测表 |
| **F2** | **MEDIUM** | 新发布的只读契约 `TowerCache::l0_units()` 与 `tower[0]` **实测失步**：as_of=71040 时 `l0_units.len()==0` 而 `tower[0].len()==547`。这不是交付报告 §7 遗留 4 描述的「`resume_from > l0_units.len()` 的轻度不同步」，是**访问器返回空**。本票的 `l1_resume_from > l0_units.len()` 守卫只是**恰好**接住（因该 bar `resume_from=535>0`）；若同样失步发生在 `resume_from==0` 的 bar，派生会走到 `detect` 上、因 `i+2 < 1` 不成立而静默落 `no_window_formed`，无任何失步信号。100k 内 1/1692 次 level=2 求值 | `mod.rs:1036-1040`（`l0_units()`）、`nest_lifecycle.rs:1785` 区的 `ResumeAnchorOutOfRange` 守卫；实测 `SH609D MISMATCH as_of=71040 resume_from=535 l0_units=0 tower0=547 tower_len=4` |
| **F3** | LOW-MED | 归因子桶（9 / 5）不可第三方复算：判定规则未写出；`frontier_not_after_confirmed` 桶在表中缺席，却是 2 只身份的众数原因、并出现在 19 只中的 8 只归因窗内 | 交付报告 §4 验收 1 表；本报告 §1 验收 1 复算 |
| **F4** | LOW | `unreachable!("循环上界 3.min(tower.len()) ⟹ level ∈ {1,2}")` 引用的代码不存在（实际是 `for level in 1..3`）。结论仍成立，理由句陈旧 | `p123_fast_replay.rs:1738` |
| **F5** | LOW | 原因码 `resume_anchor_out_of_range` 承载两个不同条件，其中一个（`l1_scan == None`）**静态不可达**：`tower.len() == cache.levels.len()`（`mod.rs:1730-1733` push 与 `tower_snapshots.push` 同迭代、两处 truncate 同步），故 `2 < tower.len()` ⟹ `levels.get(0)` 必 Some。实测亦印证：433 个 bar 的 `l1_resume_from=MAX`，无一进入 level=2 | `p123_fast_replay.rs:1660-1673`；`mod.rs:1005-1030`、`:1730`、`:2228` |
| **F6** | LOW | 静默跳过并未清除：`let Ok(lower) = lower_legs_from(&tower[level-1]) else { continue; }` 不落任何原因码，与其上方 20 行刚刚用 `tower_level_absent` 消灭静默跳过的纪律自相矛盾。该分支一旦触发，该 bar 该级在归因表里同样没有行 | `p123_fast_replay.rs:1676-1678` |
| **F7** | LOW | 090 一致性不彻底：dump tag 因「写死 L1_ 却承载 L2 = 声明膨胀」而改名，但同一理由适用的 `l1_live_outcomes` 字段、`L1LiveDiagRow` 类型、stderr key `P527_L1_LIVE_OUTCOMES` 全部原样保留 | `p123_fast_replay.rs:518,538,727` |
| **F8** | MEDIUM | 测试覆盖不对称：`active_l1_window_frontier` 覆盖很好（q1–q3 覆盖全部 5 个非成功变体 + 成功支，q4 覆盖桥迁移/零回填），但**本票最有后果的两处逻辑**——去重叠 pop 守卫、`tower_level_absent` 落码——都在 `p123_fast_replay.rs`，该文件 `#[cfg(test)]` 只有 2 条测试且都不覆盖它们。844→93 与边界条件「末项一律截断会误删真已终结单元」只在报告散文里，仓内无断言 | `p123_fast_replay.rs` 测试 mod（`lifecycle_window_stem_keeps_identity_while_extending_bar` / `run_entry_update_preserves_pan_memo_residence`） |
| **F9** | MEDIUM | #454 文件长债务被显著加剧且报告零提及：`nest_lifecycle.rs` 现 **4683 行**（生产段 2042 行，`#[cfg(test)]` 在 :2043），而 #454 立票时是 2212 行 / 生产段 978 行、上限 800。本票贡献 +460（生产段约 +183）。`recompute_lifecycle_window_stems` 现 **162 行**（上限 50），其 `match level` 臂内嵌套 ≥5 层 | `.claude/rules/common/coding-style.md`；`gh issue view 454`；`wc -l` 实测 |
| **F10** | LOW | `cargo fmt --check` 命中本票新增的 import 行（`use newchan_rust::theta_v0::classifier::center::UnitRange;` 排序位置）。另两文件 fmt 干净；仓库整体 fmt 本就不干净，故仅记为 nit | `p123_fast_replay.rs:177` |
| **F11** | LOW | #598 §5「L2/L3 预期比 L1 更丰富的 `ObservationSeam`」被实测证伪（L2=0 vs L1=12），交付报告登记了读数但未登记为对 #598 的证伪 | `issue598-…-20260728.md` §5 下游推论；本评审复算 `identity_vanished_seam` 全局 12（全部 L1）|
| F12 | 提示 | 交付报告 §3「三文件零警告」——该断言的口径是隔离 index 树；在当前 HEAD 上 `classifier/mod.rs:1785` 有一条既有 `variable does not need to be mutable`（#601 diff 区外，非本票引入）。措辞按 HEAD 读会不成立 | `cargo check --lib --message-format short` |

### #609 重点核验项 6（flaky 与两处未追）的建议

| 项 | 建议 |
|---|---|
| `open_ledger::parent_units_follows_latest_instance_vertical` 一次红后全绿 | **可接受未知**。本评审全量跑 2040 passed，该测试通过；`open_ledger.rs` 不引用 `nest_lifecycle`，本票三文件也不被它引用，判「不并入本票红绿」正确。但**建议挂一条 tracker 记录到 #571**（该测试的所属票）——一次红若无锚，下次复现时无从对照，"记为既有 flaky" 在报告里说过就等于没说 |
| L2 `ObservationSeam=0`（对照 L1=12） | **可接受未知**，报告的限定措辞（"本样本读数、非不发生接缝的证明"）合规。但**须补一步**：它证伪了 #598 §5 的明确预测（F11），应当作为否定性结果入账，而不是只当成"形态不同" |
| `resume_anchor_out_of_range` 1/2269 | **不可接受为已登记未知（→ 条件 1）**。现场实测是 `l0_units()` 返回空而 `tower[0]` 有 547 条，是**本票新发布的只读契约上的失步**，不是"锚越界"。当前守卫只是恰好接住；且该契约现在是外部读契约，其不变式被实测违反一次却无任何断言/诊断。建议单开 bug 票追 `l0_units_cache` 在该 bar 为空的根因，并在守卫处加 `l0_units.len() != tower[0].len()` 的独立原因码 |

---

## 5. 条件（PASS 的前置）

1. **F2 单开票追根因**：`TowerCache::l0_units()` 与 `tower[0]` 失步（0 vs 547 @ as_of=71040）。至少要么给出不变式
   证明并加断言，要么加独立原因码把失步与 `no_window_formed` 区分开。
2. **F1 收口或给反证**：把去重叠守卫改成用 `last_window_emitted` / `tower_confirmed_len(level)` 的整窗口径；
   若坚持同起点单 pop，须给出「残留的 2–6 个未确认子单元不可能改变 seg_a」的证明（本评审实测该情形覆盖
   58/314 真产出活窗）。
3. **F3 补口径**：写出归因子桶的判定/优先级规则并重发子桶表，或把 `frontier_not_after_confirmed` 单列成桶。
   §7 遗留 3 的下游推论依赖这张表。
4. **golden 纪律补齐**：把 `chanlun/review-results/issue601-panlive-l2-provider-20260728.md` 入库（现为未跟踪），
   使 `9a6aa0e885` 的理由指针在历史里可达；「编排侧已批」补一条 issue 留痕。
5. **F9 登记**：把本票对 #454 的增量（+460 行 / 生产段 2042 行，`recompute_lifecycle_window_stems` 162 行）
   写进 #454，或另立子票。debt destination 在案不等于增量可以不记。

F4–F8、F10–F12 建议随下一次触碰该文件时顺手清（F6、F8 值得优先）。

---

## 6. 结果包六要素

1. **结论**：#601（`961a4260ee`）+ golden 重锚（`9a6aa0e885`）判 **PASS WITH CONDITIONS**。五项 Acceptance 的
   实质要求全部独立复现；L1/L3 逐字节零回归、stdout 面零漂移、两个仓内字节门全绿、测试门唯一红 #491——
   均由本评审自建 pre/post 二进制复跑坐实。4 项新发现构成 5 条条件，其中 F1/F2 为 MEDIUM。
2. **定义依据**：
   - 「行进中」判别的充要形态 = 末窗含虚拟追加单元（`win.1+1 == units.len()`）⟺ `WinMeta.read_end_src ==
     usize::MAX`——`recursive_tower.rs:411-414` 原文；本评审在 #148 升级重切路径上复核该等价仍成立
     （末子中枢 `(s, e=j-1)`，`win.1+1 == j`）。
   - 「哪些塔单元算确认」的单一来源 = `TowerCache::tower_confirmed_len`（`mod.rs:1005` 自称 #93 水线证书、
     禁第二查法）；「frontier 回退域 = 整窗」= `WindowScanCursor.last_window_emitted` 文档
     （`recursive_tower.rs:401-404`）。F1 的判定直接取这两条。
   - 「归因闭合」= 每只 L2 完成身份在其 C 活跃期 `[c_start, completed_at]` 内至少有一条 level=2 诊断行
     或存在 `observed_at < completion_as_of`（#559 C1 口径 + 票面验收 1）。
   - golden 变更纪律 = `rust/tests/issue533_p123_byte_guardrail.rs:44-47`（已审阅、故意、同 PR 说明原因并
     引用 issue/report）。
   - 认识论等级（`formalization-validity-domain`）：本评审全部读数为 **L2**（BTC 单标的、100k 单窗真实数据）；
     F1 的暴露面统计（20.0% / 58 / 45）同为 L2，不得外推为跨品种常量。
3. **边界条件（本评审结论在何时翻转）**：
   - 若 F1 的残留未确认子单元被证明**不进入 seg_a 定位路径**（例如 `locate_pan_div_structure` 的 A 锚
     恒早于 B 中枢、而 B 中枢恒早于残留子单元），F1 降为 LOW（纯洁癖问题）；
   - 若 F2 的 `l0_units()` 为空被证明只发生在 `tower[0]` 同时不被消费的 bar 上，F2 降为 LOW；
   - 若交付方给出归因子桶规则且按该规则复算得到 9/5，F3 撤销；
   - 若本机的 BTC 数据与交付方不同（本评审用 `analysis/data_cache/btc_1m_full.json` 符号链接至
     `~/Projects/NewChanlun/…`，与交付方同一文件），所有读数复现结论作废——已核为同一文件。
4. **下游推论**：
   - #602（L3）会**继承 F1/F2**：它要新增第三个只读访问器并复用同一套「虚拟追加 + 重扫」骨架，
     去重叠守卫与 `l0_units()` 类契约的问题会在 L2 层原样出现（且 L2 层的 `last_window_emitted` 分布未测）。
     建议 F1/F2 在 #602 动手**之前**收口，否则 L3 会把同一个洞复制一层。
   - #603/#602（被 #601 blocking）可以在 F1/F2 未收口的前提下推进读数类工作，但不应把 L2 的 `window=314`
     当作稳定身份的集合直接消费。
   - #598 的 §5 预测被证伪（F11），其"下游若消费 `VanishCause` 统计需注意复合来源"的建议仍成立，
     但"L2/L3 会更丰富"这条前提须撤回。
5. **谱系引用**：#523（PanLive 接缝根因，遗留 1/2/3）；#527/#559/#578/#591/#592（L1 先例与措辞限定）；
   #598（路线 i 裁定与切片建议，本评审核其四处偏差 + 一处预测证伪）；#449（判据/nest 方向禁令，本票方向
   核为不落入，但其自动守卫已不在仓内）；#533（字节护栏门与 golden 纪律）；#454（文件长债务）；#148
   （升级重切 = F1 的成因域，`b-route-recut-precheck-20260715.md` 为其可达性的仓内锚）；090（声明=能力，
   F7 依此判）；`no-patch-mentality`（F1 判为"修法只对一半"而非"可接受折中"依此）；
   `formalization-validity-domain`（F11 与本评审全部读数的等级标注）。
6. **影响声明**：本评审为**只读**。仓内唯一新增文件 = 本报告
   `chanlun/review-results/shadow-601-review-20260728.md`；未改任何生产代码/测试/fixture/issue 状态/map/roster。
   评审过程中的探针实验全部在 `/tmp/sh609-probe`（`git archive HEAD` 导出的独立副本）中进行，探针版二进制
   产出的 dump/stdout SHA 与无探针版逐位相同（`4697f764…` / `d8b69c18…`），证明探针零行为影响。
   pre 侧对照树在 `/tmp/sh609-pre`（`git archive 961a4260ee~1`），独立 target 目录，未触碰工位。

## 7. 复现命令（本评审自用，全部只读）

```bash
# post（工位 HEAD）
cd /tmp/kimi-nest-mainline/rust
CARGO_TARGET_DIR=/tmp/kimi-nest-target-609 cargo build --release --bin p123_fast_replay
P116_MAX_BARS=100000 P421_LIFECYCLE_DUMP=/tmp/sh609-100k.dump \
  /tmp/kimi-nest-target-609/release/p123_fast_replay ../analysis/data_cache/btc_1m_full.json \
  > /tmp/sh609-100k.stdout 2> /tmp/sh609-100k.stderr

# pre（961a4260ee~1 = 4f941af2b9），只读导出到 /tmp，不动工位
cd /tmp/kimi-nest-mainline && git archive 961a4260ee~1 | tar -x -C /tmp/sh609-pre
ln -sfn ~/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json \
        /tmp/sh609-pre/analysis/data_cache/btc_1m_full.json
cd /tmp/sh609-pre/rust && CARGO_TARGET_DIR=/tmp/sh609-pre-target cargo build --release --bin p123_fast_replay
P116_MAX_BARS=100000 P421_LIFECYCLE_DUMP=/tmp/sh609-pre-100k.dump \
  /tmp/sh609-pre-target/release/p123_fast_replay ../analysis/data_cache/btc_1m_full.json \
  > /tmp/sh609-pre-100k.stdout 2> /tmp/sh609-pre-100k.stderr

# L1/L3 逐字节对拍
for lv in 1 3; do
  grep -E "^(REV|COMPLETION_SIGNAL) as_of=[0-9]+ level=$lv " /tmp/sh609-pre-100k.dump  > /tmp/pre-lv$lv.txt
  grep -E "^(REV|COMPLETION_SIGNAL) as_of=[0-9]+ level=$lv " /tmp/sh609-100k.dump      > /tmp/post-lv$lv.txt
  diff -q /tmp/pre-lv$lv.txt /tmp/post-lv$lv.txt
done
grep -E "^L1_LIVE_(HIT|MISS)" /tmp/sh609-pre-100k.dump > /tmp/pre-l1diag.txt
grep -E "^PAN_LIVE_(HIT|MISS) as_of=[0-9]+ level=[01] " /tmp/sh609-100k.dump \
  | grep -v "reason=tower_level_absent" | sed -E 's/^PAN_/L1_/; s/ level=[01] / /' > /tmp/post-l1diag.txt
diff -q /tmp/pre-l1diag.txt /tmp/post-l1diag.txt

# 仓内门
CARGO_TARGET_DIR=/tmp/kimi-nest-target-609 cargo test --lib
CARGO_TARGET_DIR=/tmp/kimi-nest-target-609 cargo test --release --test issue533_p123_byte_guardrail -- --ignored
CARGO_TARGET_DIR=/tmp/kimi-nest-target-609 cargo test --release --lib m8_byte_guardrail -- --ignored

# F1/F2 探针（在 /tmp/sh609-probe 的 HEAD 副本里改 p123，打印 last_window_emitted / l0_units 失步）
```

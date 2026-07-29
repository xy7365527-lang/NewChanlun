# wave-1 plan 5a 实装卡：`trend_confirm_time` per-pair 游标驻留

> **复原卡（2026-07-27），非原文。** 原卡（2026-07-19）从未入 git 且已佚失（#479 登记）。本卡据 issue93 实装卡 §1、wave1-5a5b-blocker-clearance-20260720、r1-r3-ruling-confirmation-checklist-20260720（R1-R3 已确认）、scene-ledger 复原；行号锚已按 2026-07-27 工位代码重测。

- **票据关系**：#69 的 5a；复原票 #479。
- **工位基线**：`/tmp/wt-69`，分支 `ticket-69`，基于 `kimi-nest-mainline-20260717`。
- **本文性质**：纯文档复原，不声明 5a 已实装。现行代码仍明确登记 per-pair 游标和 assemble/provider 单源化“未实装”（`rust/src/bin/p123_fast_replay.rs:79-88`）；`ConfirmCursorStore`、`ConfirmCursor`、`assemble_level_view_resident` 在现行 `rust/src/` 均不存在。
- **裁定终态说明**：2026-07-20 清单正文仍写“待确认”；其后 `chanlun/escalate/g1-g2-window-ban-ruling-20260721.md:36-44` 登记编排者已逐项确认 R1-R3。本文以该终态为准，不重诉 R1-R3。
- **锚定规则**：下文代码行号均为 2026-07-27 `/tmp/wt-69` 的 1-based 行号；历史符号发生改名时同时列出历史名与现行名，禁止把旧名伪装成现存符号。

## 1. 作用面与缝形

### 1.1 唯一作用面

5a 只治理 `p123_fast_replay` 塔重放路径中 `trend_confirm_time` 对同一 divergence pair 的逐次重算：

1. `run_targeted_prefix_pass` 逐 bar 建塔并维护 per-level `LevelDerived` 与 per-run `RunEntry`（`p123_fast_replay.rs:762-800,781-782`）。
2. dirty run 调 `evaluate_run`（`:897-907`）；forced shadow 再调同一函数（`:940-950`）。
3. `evaluate_run` 从该 run 的投影种子生成 centers/blocks（`:1097-1112`），再走 `assemble_level_view`（`:1122-1134`）与 `provide_nest_candidate_events`（`:1135-1147`）。
4. 现行冷路径对 trend pair 计算两次：assemble 内 `trend_confirm_time`（`level_view.rs:1154-1165`），provider trend 分支再次调用（`:774-785`）。5a 的 seam 必须让同一 view 的完成态与 provider 事件消费同一份 per-pair 结果，不能保留两套判定。

本卡不治理 runner/NestChainGate 的逐 bar 全级事件派生；该链与 p123 的状态持有者、回归锁均不同，边界见 `issue93-implementation-card-20260721.md` §1。

### 1.2 成本靶与历史实测分布

佚失卡登记的历史分布为：

| 三态 | 历史占比 |
|---|---:|
| `Confirmed(t*)` | ≈ 50.9% |
| `TerminalFalse` | ≈ 43.1% |
| `Scanning` | ≈ 6% |

这些数值只作原卡成本证据的复原登记，**本工位未重跑**；样本规模、数据集、采样命令与原始计数已佚失，见 §9。不得把近似百分比写成新的验收阈值。

### 1.3 缝形与禁区

- resident 状态按 divergence pair 分隔，寿命按 level 分隔；持有者是 bin 内 `LevelDerived` 旁的 per-level `ConfirmCursorStore`。
- lib 新增显式入口 `assemble_level_view_resident`；旧 `assemble_level_view` 保持既有签名，内部委托 resident 核且传 `None`，所有旧调用继续走真冷路径。
- forced shadow 必须传 `None`，不得与 dirty 路径共享 store；否则两侧可能同吃陈旧状态而假绿。
- 稳定前缀只读 `TowerCache::tower_confirmed_len` 这一证书链，禁第二套游标水线、禁在 bin 重算水线。
- 不把 store 塞进 `TowerCache`，也不扩它的 lineage 硬契约；现行“不加运行时 lineage epoch”在 `classifier/mod.rs:1481-1490`。
- 不 fork `trend_confirm_time` 的教义谓词。冷、热两路必须同案同果，差别只能是扫描起点和已证累积量的复用。

### 1.4 历史“69 处调用点”的漂移登记

R2 原裁定据 preflight 实测“旧入口 69 处调用点签名不动”。在本工位按
`rg -n '\bassemble_level_view\s*\(' --glob '*.rs' rust/src`
重测为 **43 个文本命中，其中 1 个定义、42 个调用表达式**。历史 69 的文件清单已佚失，当前树无法复现。裁定语义不变：旧入口签名不得改；本文不把 42 反写成原卡事实。

## 2. 裁定锚：R1-R3 已确认，不重新诉讼

### 2.1 R1（5b 相邻约束）

接受 5b 两条 memo 失效链的构造性论证：水位链用严格 `<`；后继块链用
`b_idx + 2 < blocks.len()`，与 TURN 的 `ready = m - 2` 同构。接受 TURN 已声明残余：
末窗一次重扫产出至少两个子中枢的 bar 理论上仍可能提前落盘；拦截网为 V0 身份校验、
`P123_SHADOW=1` 全程对拍与全量双跑 dump diff。

R1 不授权 5a 另造 pan memo，只约束同工位排期和最终验收。

### 2.2 R2（5a 本卡决定项）

`ConfirmCursorStore` 放在 bin `LevelDerived` 旁，按 level 驻留：

- 决定性理由：shadow oracle 必须能走真冷路径。store 在 bin，forced 路径传 `None` 即可；store 若藏在 lib/TowerCache，两路共享状态，`mismatches=0` 可能是假象。
- 寿命理由：`LevelDerived` 已持有 lower snapshot、`lower_gen`、`lower_legs` 与 `lower_ends`（`p123_fast_replay.rs:353-370`），与游标读域同寿。
- seam 理由：新增 `assemble_level_view_resident`，旧 `assemble_level_view` 委托 `None`；旧入口调用点签名不动。
- 契约理由：不扩 `TowerCache` lineage epoch 硬契约。

### 2.3 R3（5b 相邻约束）

5b 的 `PanMemo` 放在 bin `RunEntry` 旁，因为 center 来自 run 投影种子、kind 门来自 run blocks；显式可选参数保证 forced 路径传 `None`。5a 不得把 pan 值混入 per-level store。

### 2.4 R4 排期约束

同一实装工位按三段推进：

1. 段一：lib 暴露面（历史登记“已落”；现行符号重锚见 §3.6）。
2. 段二：5a。
3. 段三：5b。

5a/5b 都改 B3-B5：`evaluate_run` 签名、dirty 调用点、shadow forced 调用点；不得并行制造两套互不兼容的签名。

## 3. 逐函数设计

### 3.1 `ConfirmState`：三态只表达已证事实

在 `level_view.rs` 增加供 resident 核与 view 消费的三态值：

```rust
enum ConfirmState {
    Confirmed(usize),
    TerminalFalse,
    Scanning,
}
```

语义契约：

- `Confirmed(t*)`：冷核已找到第一个 T2∧T5-OR 同真的段端点，`t*` 必与现行 `trend_confirm_time` 的首证钟相同。
- `TerminalFalse`：只在现行单调终假分支 `!force_ok`（`level_view.rs:619-622`）被已封输入证实时成立。T5 各 proxy 只增，真转假后不会复真。
- `Scanning`：尚无首证钟且未出现已证终假；包括结构/坐标/力度暂不可验和扫描尚未跨过可变尾部的情形。不得把现行任意 `None` 粗暴折成 `TerminalFalse`。

现行 `trend_confirm_time -> Option<usize>` 会把“终假”和“仍扫描”压成同一个 `None`。实施时须在**同一个判定核**内暴露三态；旧函数若保留，只能是该核的无状态投影，不能复制谓词。

### 3.2 `ConfirmKey` / `ConfirmCursor` / `ConfirmCursorStore`

`ConfirmCursorStore` 是 per-level 映射；每个 key 只对应一个 run 语境中的一个结构 pair。key 至少要封闭这些语义身份：

- `DivergencePairId`（现行字段见 `level_view.rs:451-456`）；
- run/provider window 与查询 level/version；
- pair 的 `seg_a`、`seg_c` 起点及 B 中枢稳定身份；
- 会改变 T2/T3/T4/T5 结果、但不随 `as_of` 单调增长的结构代次。

`as_of` 与可增长的 seg-c 末端不能伪装成身份键，否则每 bar 必 miss；反之，run、pair、B、版本任一变化都必须 key miss。原卡精确字段布局已佚失，以上是从现行读域反推的**完整性约束**，不是宣称复原了原 struct。

`ConfirmCursor` 至少保存：

- 已消费的 lower-leg 下标 `k0`；
- T2 的 envelope；
- T5 的 `acc_hi`、同色面积、DIF 峰谷、hist 峰谷；
- 已证三态与 `t*`；
- 与该状态对应的确认水线/结构指纹。

store 必须支持按 run/pair 剪枝，不能让已消失的 run 或 pair 永久滞留。

### 3.3 `trend_confirm_time`：冷核与 resident 核同源

现行函数在 `level_view.rs:544-635`：

- 静态前置与映射：`:557-568`；
- lower-leg 扫描界：`:570-578`；
- T2/T5 累积：`:579-603`；
- 终假：`:619-622`；
- 首证钟：`:624-631`。

实施要求：

1. 抽出一个接收可选 cursor 的单一扫描核；`None` 从零扫描，`Some` 从已封前缀续扫。
2. 先用 store 的稳定状态构造本次局部 cursor，再扫描可变尾；只有证书覆盖的累积状态可以回写 store。
3. 可变尾上的临时累积可用于本次返回，但在未获稳定前缀证书前不得跨 bar 持久化。
4. 冷核和 resident 核用同一 T2/T3/T4/T5 代码，不能一个调用旧函数、另一个重写近似逻辑。
5. 对相同输入，三态投影为现行可观察结果：`Confirmed(t*)` 映射 `Some(t*)`；其余映射 `None`。

### 3.4 `LevelAsOfView` 与 assemble/provider 单源化

现行 `LevelAsOfView` 只有 `query/cache_key/moves/pairs`（`level_view.rs:1028-1033`），无法把 pair 确认结果交给 provider，导致同 view 双算。

resident 核须把与 `pairs` 一一对齐的三态结果保存在 view 或等价 sidecar 中：

- assemble 的 trend completion 只消费这份结果；
- provider trend 事件也只消费这份结果；
- pair 顺序/身份必须以 `DivergencePairId` 校验，不得按偶然 vec 下标静默错配；
- legacy `assemble_level_view(... )` 委托 resident 核并传 `None`，仍产相同公开 view 语义；
- provider 的旧公开入口签名是否保持、sidecar 如何穿过 ext 入口，原卡精确 API 已佚失；实施必须以“单核单结果、旧调用不破坏”为硬约束。

### 3.5 bin 接线：`LevelDerived`、`evaluate_run`、dirty/shadow

在 `LevelDerived` 增加 `confirm_cursors: ConfirmCursorStore`。最终同工位签名语义为：

```rust
fn evaluate_run(
    /* 现有参数原序保留 */,
    confirm_cursors: Option<&mut ConfirmCursorStore>,
    pan_memo: Option<&mut PanMemo>, // 段三 5b 再接入
) -> Result<Vec<NestCandidateEvent>, String>
```

原卡的精确参数顺序已佚失；“两个可选状态都显式传入、forced 均传 `None`”是复原出的接口契约。

- dirty 路径（现行 `p123_fast_replay.rs:897-907`）传该 level 的
  `Some(&mut level_derived.confirm_cursors)`；phase 2 的 pan 参数先传 `None`。
- forced shadow（`:940-950`）两个状态参数都传 `None`，形成真冷 oracle。
- snapshot/其他 legacy 调用继续走 `assemble_level_view`，不自动共享 p123 store。
- `LevelDerived` lower snapshot/代次发生不兼容变化时，先按 §4 失效 store，再续算。

### 3.6 段一 lib 暴露面：现行能力重锚

历史卡把段一记为“已落”。在本工位，语义证书链仍在，但两个字段名已漂移：

| 历史登记 | 现行符号/落点 | 2026-07-27 判定 |
|---|---|---|
| `tower_confirmed_len(level)` | `TowerCache::tower_confirmed_len`，`classifier/mod.rs:990-1008` | 在案；唯一读入口 |
| `tower0_confirmed_len` | 现名 `TowerCache::l0_confirmed_len`，字段 `:899-901`，写入 `:1552-1553`，clear 归零 `:1038-1041` | 语义在案；旧符号不存在 |
| parser `segments_confirmed_len` | `parser/mod.rs:100-103`，计算 `:274-280`，写入 `:286-290` | 在案 |
| L1+ `prefix_count` sealed | `classifier/mod.rs:1891-1894`；cascade 收缩水线 `:1813-1814`；自然推进 `:1989-2001` | 在案 |
| 历史 `LevelCache.confirmed_len` | 现名 `LevelCache::confirmed_watermark`，字段 `:861-869`，全清归零 `:1769` | 语义在案；旧符号不存在 |

因此 5a 只能调用 `tower_confirmed_len`，不能重新实现
`segments_confirmed_len`/`prefix_count` 公式，也不能复活旧字段名形成第二来源。

## 4. 读域、写域与失效条件

### 4.1 读域

| 读域 | 现行锚 | resident 约束 |
|---|---|---|
| lower legs / segments | `lower_legs_from`，`level_view.rs:418-434` | tower[level-1] 与 legs 1:1 |
| pair/B/seg-a/seg-c | `DivergencePair`，`:451-464`；pair 构造 `:905-977` | 身份变化即 miss |
| T4/T3 前置 | `trend_confirm_time:557-568` | 与冷核同源 |
| T2 envelope | `:579-585,624-631` | 只持久化已封前缀 |
| T5 area/DIF/hist | `:586-622` | 只持久化已封前缀 |
| as-of 扫描上界 | `:570-571` | 仅作增长界，不作身份 |
| 稳定 legs 水线 | `TowerCache::tower_confirmed_len`，`classifier/mod.rs:990-1008` | 单一证书来源 |

### 4.2 写域

允许写：

- bin `LevelDerived.confirm_cursors`；
- 本次构造的 view/sidecar 中 per-pair `ConfirmState`；
- 5a 专用的诊断计数。

禁止写：

- `TowerCache` 的额外 resident 状态或 lineage epoch；
- parser/塔水线的第二份计算；
- `RunEntry.pan_memo`（属于 5b）；
- 冷路径共享的隐式全局状态。

### 4.3 水线推进与回退

对 level `L` 的 lower legs，水线量纲是
`cache.tower_confirmed_len(L - 1)`；`lower_legs_from` 对 `tower[L-1]`
逐项映射，故该值可作为 legs 稳定前缀长度。实施时必须用测试锁住这一量纲，不得把 source 坐标或 upper level 的水线拿来比较。

- 若 `w_now >= cursor.k0` 且 key/结构指纹一致，可从 `k0` 续扫。
- 若 `w_now < cursor.k0`，说明证书回退，清除该 cursor 并从冷核重算；不得倒播累积器。
- `lower_gen/self_gen` 变化不等于必然全清，但凡无法证明 key 与已封读域仍同一，就从首个不可证 pair 起失效；无法定位时清该 level store，宁可 over-shrink。
- cascade/frontier pop 已由塔证书把水线收缩（`classifier/mod.rs:1813-1814,1989-2001`）；5a 不再猜测塔内变更。

### 4.4 三态单调性与等价性

- `Scanning -> Confirmed(t*)` 可发生一次；后续返回保持同一首证钟。
- `Scanning -> TerminalFalse` 只由已封输入上的 `!force_ok` 触发。
- 同一 key 下 `Confirmed` 与 `TerminalFalse` 都是终态；key 或证书失效后必须丢弃，不能跨新结构继承。
- 任意 bar 的热路输出必须等于该 bar 从空 store 跑出的冷路输出；“更快”不构成放宽。
- 历史 50.9%/43.1%/6% 只解释收益面，不是状态机正确性的证明。

## 5. 测试计划（TDD 切片）

下列是按幸存约束重建的 TDD 切片；原卡测试函数名与精确用例表已佚失。

### T0：冷核刻画测试先行

- 固定现行 `trend_confirm_time` 的 T2/T3/T4/T5 首证钟、未决与终假代表例。
- 对公开 assemble/provider 记录事件键、`divergence_confirmed`、`interval_b`、`turn_source` 与 move completion。
- 门关/legacy 入口输出逐字段不变。

### T1：三态拆分

- 覆盖 `Confirmed(t*)`、显式 `!force_ok -> TerminalFalse`、其余 `None -> Scanning`。
- 加反例锁：坐标缺失、T3 未到、无可扫 leg 不得误判终假。
- `ConfirmState` 投影回 `Option<usize>` 与 T0 完全一致。

### T2：cursor 增量核

- 同一 pair 逐步增加 `as_of`，热路每步等于从空 cursor 的冷路。
- 在 T3 前、T3 当 bar、T2 首破、T5 首转假、MACD 边界各切一刀。
- 验证累积器只加入 `(acc_hi, cur_hi]`，不重加、不漏加。

### T3：水线与失效

- 水线单调增长时 `k0` 续进且结果不变。
- frontier pop/cascade 使水线回退时 cursor 清除并冷重算。
- run/pair/B/version 改变时 key miss；旧 key 不污染新 pair。
- 缺级水线为 0 时保守冷算。

### T4：assemble/provider 单源化

- 对同一 view，assemble completion 与 provider trend 事件读取同一个 pair state。
- 用计数探针证明 resident 调用每 pair 每次 view 只进判定核一次；冷/热输出逐字段一致。
- legacy `assemble_level_view` 所有调用保持编译与行为不变。

### T5：p123 接线与真冷 shadow

- dirty 调用传 resident store；forced 调用传 `None`。
- 构造污染 resident store 的测试，forced 必须仍给冷结果并报 mismatch，证明 oracle 未被双染。
- `RunEntry` 的现有 dirty/reuse/pending 过滤顺序不变。

### T6：集成与性能证据

- `cargo test` 全绿。
- 250k 与 1M 对拍 diff=0。
- `P123_SHADOW=1` 全程 0 mismatch。
- 在线 pending/views 计数一致。
- 记录 Confirmed/TerminalFalse/Scanning 新实测计数，但不要求复刻历史近似比例。
- 最终做 wave-1+5a/5b 全量双跑合并封印。

## 6. 验收协议

5a 只有同时满足以下条件才可声明完成：

1. `cargo test` 全绿；具体通过数随代码演进，以实装提交当时输出为准，禁止沿用 2026-07-20 的 1770/0 冒充新跑。
2. 250k、1M 两档基线 diff=0；对比内容须覆盖 stdout 语义行与约定 dump，允许的物理计数字段白名单须在实装报告逐项列明。
3. `P123_SHADOW=1` 全程 `shadow_mismatches=0`，forced 路径明确传 `None`。
4. 在线 pending/views 计数与基线一致；views 的物理装配计数若按既有白名单排除，须同时展示排除前后原值。
5. 全量双跑 dump diff=0，按 scene-ledger 的“wave-1+5a/5b 合并封印”口径结案。

`.chanlun/scene-ledger.md` 的历史 `:16/:17/:18` 在本工位漂移为现行 `:18/:19/:20`：

- `:18`：先解决 5a/5b 超线性，再做一次性合并总重跑；
- `:19`：wave-1 自身只跑到 6/8、在线一致至 3.0M；
- `:20`：编排者仅豁免 wave-1 自身 4.6M 封口。

该豁免**不覆盖 5a/5b**。5a 未落全量双跑封印时，只能写“靶向门已过、全量待验”，不能写“验收完成”。

## 7. 2026-07-27 重锚表

| 符号/缝 | 文件:行 |
|---|---|
| `LevelDerived` | `rust/src/bin/p123_fast_replay.rs:358-370` |
| `RunEntry` | `rust/src/bin/p123_fast_replay.rs:374-383` |
| dirty `evaluate_run` 调用 | `rust/src/bin/p123_fast_replay.rs:897-907` |
| forced shadow 调用/比对 | `rust/src/bin/p123_fast_replay.rs:940-973` |
| `evaluate_run` | `rust/src/bin/p123_fast_replay.rs:1097-1148` |
| run 投影 centers/blocks | `rust/src/bin/p123_fast_replay.rs:1109-1112` |
| `DivergencePairId` / `DivergencePair` | `rust/src/theta_v0/classifier/level_view.rs:451-464` |
| `trend_confirm_time` | `rust/src/theta_v0/classifier/level_view.rs:544-635` |
| provider 内第二次 trend confirm | `rust/src/theta_v0/classifier/level_view.rs:774-789` |
| `LevelAsOfView` | `rust/src/theta_v0/classifier/level_view.rs:1028-1033` |
| `assemble_level_view` | `rust/src/theta_v0/classifier/level_view.rs:1045-1197` |
| `tower_confirmed_len` | `rust/src/theta_v0/classifier/mod.rs:990-1008` |
| `l0_confirmed_len` 字段/写入 | `rust/src/theta_v0/classifier/mod.rs:899-901,1552-1553` |
| `confirmed_watermark` 字段 | `rust/src/theta_v0/classifier/mod.rs:861-869` |
| cascade 水线收缩 | `rust/src/theta_v0/classifier/mod.rs:1813-1814` |
| `prefix_count` sealed | `rust/src/theta_v0/classifier/mod.rs:1891-1894` |
| 自然水线推进 | `rust/src/theta_v0/classifier/mod.rs:1989-2001` |
| parser `segments_confirmed_len` | `rust/src/theta_v0/parser/mod.rs:100-103,274-290` |
| TowerCache lineage 硬契约 | `rust/src/theta_v0/classifier/mod.rs:1481-1490` |

## 8. 复原来源清单

1. `chanlun/review-results/issue93-implementation-card-20260721.md` §1：5a/5b 作用面、缝形、历史分布、禁第二套游标/水线。
2. `chanlun/review-results/wave1-5a5b-blocker-clearance-20260720.md`：阻塞终态、段一暴露面与 5a/5b 验收豁免边界。
3. `chanlun/escalate/r1-r3-ruling-confirmation-checklist-20260720.md`：R1/R2/R3 卡内位置与原裁定口径。
4. `chanlun/escalate/g1-g2-window-ban-ruling-20260721.md` §4：R1-R3 逐项“确认”的终态登记。
5. `.chanlun/scene-ledger.md:18-20`：中止令、wave-1 验证登记、wave-1 自身豁免及合并封印口径。
6. `/Users/silencehan/Projects/NewChanlun/kimi-export-session_-20260721-173955.md:2095-2117`：佚失卡历史节号/行号及 R1-R3 引用。
7. GitHub issue #69、#67、#479 票体。
8. 2026-07-27 `/tmp/wt-69` 现行 Rust 代码，只读重测行号与能力现状。

## 9. 未能复原清单

以下内容没有幸存证据，本文未推测填空：

1. 2026-07-19 原卡逐字正文、原章节完整目录、原图表与原作者措辞。
2. `ConfirmKey`、`ConfirmCursor`、`ConfirmCursorStore` 的原始字段名、精确 Rust 类型、可见性、容器类型和淘汰 API。
3. `assemble_level_view_resident`、三态 enum、view sidecar 与最终 combined `evaluate_run` 的原始精确签名、参数顺序和命名。
4. 原卡如何让 assemble/provider 共享结果的精确载体布局；只复原了“单核单结果、旧入口不改签名”的硬约束。
5. 历史“69 处调用点”的逐文件清单及计数命令；现行树只重测出 42 个调用表达式。
6. 历史 50.9%/43.1%/6% 的样本数、数据集、采样日期、原始整数计数与生成命令。
7. 原卡测试函数名、fixture 名、预期扫描次数和性能预算绝对值。
8. 250k/1M 与全量双跑所用的原脚本、dump 文件名、正规化脚本及当时唯一白名单全文。
9. 原卡对 store 内存上限、陈旧 key 回收频率及诊断计数器命名的精确规定。

这些佚失项不得在实施报告中倒写成“原卡规定”；实施时新增的具体选择必须明确标注“据复原卡约束新定”，并由冷路对拍与验收协议约束。

# #586 T3-in-c 判据工程口径研究

- 日期：2026-07-28
- 票据：#586（地图 #582）
- 仓库快照：`main@c8e87c4a0d056c039bb215347adf6c21e75c408c`
- 性质：纯调研；未修改 Rust、测试、配置或权威文档

**统计口径**：基于上述 `main` 快照的静态只读源码、ADR 与原文核查；函数和调用关系先由 `rust/` 独立代码图索引，再以当前源码逐行复核；数值结果来自 `/tmp/research-586/t3_caliber_demo.py` 的 4 组手构确定性案例（非 wf8 样本、无统计推断）；复用 #565 已归档的 58 例静态观测分桶，仅作既有证据引用；本次未运行重放、未做异质审查。

## 0. 090 证据分层与总裁定

本报告把结论按以下层次分开，避免把实现事实、原文形态和工程建议混成一个“已裁定”：

| 标记 | 含义 | 本报告中的来源 |
|---|---|---|
| 原文事实 | 缠师原文能够直接支持的结构关系 | `037`、`018`、`020`、`017`、`027` |
| 项目裁定 | 仓内已经生效的工程语义 | ADR 0001 补充十三至十五 |
| 实现事实 | 当前 `main` 的真实代码行为 | `signal.rs`、`level_view.rs`、`classifier/mod.rs` 等 |
| 工程建议 | #586 可交给 spec 的候选定义 | 本报告第 5 节 |
| 未决项 | 不能由 #586 单票代替裁定的事项 | #584、#585、#587、#588、#589 |

总裁定：

1. `trend_third_class_in_c` **不能直接**作为 T3-in-c 合取接入 `judge_first_cached`。它与补充十五共用严格价格几何，但候选对选择规则相反：前者“全窗后扫取首中”，后者“旧框右边固定首对，任何失败即终止”。
2. LevelView T3 与 `judge_first_cached` 在理想同级输入下可共享“单对严格几何”原语，但不是同一对象、同一时点或同一窗口身份。推荐两条编排流保留，抽出一个规范的固定首对分级器，并用跨流等价测试防漂移。
3. `λ_C` 与 LevelView `c_start` 都是 `source_index` 坐标，但语义不同。前者是当前离开 episode 的局部起点，后者在现有 provider 中被锚为完整离开窗的较早起点。T3-in-c 不应借用 `λ_C` 作为旧框首对的下界。
4. 连续缺口或单腿 `c` 在尚无紧随回试时不能证明 T3；单中枢盘背被当前趋势门挡在一类流外，但“两中枢、无 T3 的趋势形状”如何命名仍是 #584 的未决项。
5. #586 的判据子节已经足以进入 spec；完整 #589 仍不能直接定稿，因为一类流的命名/分流、影响面和消费者契约尚有前置票。

## 1. 复用性：扫描器与补充十五不是同一谓词

### 1.1 当前代码的真实差异

`judge_first_cached` 当前只接收：

```text
(last_center, trend_dir, seg, anchor_dir, MACD/价格映射,
 a_seg, c_move_start, gauge)
```

它在 `rust/src/theta_v0/classifier/signal.rs:289-404` 完成趋势方向、破最后中枢、A/C 配对、037:20 创新极值、`I(C)=[λ_C,seg.end]` 力度比较以及 `BspPoint` 构造；入参里没有全量 `segments`，也没有可供后扫的 `as_of`。`c_move_start` 只用于 C 力度区间，不验 T3。

现有 `trend_third_class_in_c` 在 `signal.rs:511-544` 的选择过程是：

```rust
lo = first segment with start >= max(c_start, last.end_index)
hi = first segment with end > as_of
for every adjacent pair in segments[lo..hi]:
    wrong direction  => continue
    strict price fail => continue
    first full hit    => return Some(...)
return None
```

因此它的“首个”是**所有合格相邻对中的首个命中**，不是旧框右边的首个相邻对。

补充十五的生产实现 `historical_bound_third_cert` 在
`rust/src/theta_v0/classifier/mod.rs:262-298` 则是：

```rust
leave_idx = first move with start >= frozen_center.end_index
leave  = move[leave_idx]       // 固定
retest = move[leave_idx + 1]   // 固定
if direction is not opposite: return None
return judge_third_cert(center, leave, ..., retest) // 价格失败也是 None
```

ADR 0001 `docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:250-253`
已经把“旧框右边第一条 + 紧随一条；方向或价格失败即 `None`；禁止后扫替代”写成项目裁定。故两者不是可互换实现。

此外，直接把扫描器作为 `judge_first_cached` 的早退门还会碰到一个现有契约：`signal.rs:358-404`
明确保留“结构破位但 D 未背驰”的零六 bit `BspPoint`，并用 `struct_break_dir` 让它进入下游可否证样本。
若 T3 失败就 `return None`，会连这些结构负样本一起删除；这已越过 #586，必须由 #584/#587 明确裁定。

### 1.2 数值案例 A：同向续行挡道

统一设冻结中枢：

```text
B.end=10, B核心=[ZD=100, ZG=120], 趋势方向=Up, candidate/as_of=40
```

中枢右侧完成段：

| 段 | 区间 | 方向 | 终点价 |
|---|---:|---|---:|
| s0 | [10,20] | Up | 130 |
| s1 | [20,30] | Up | 145 |
| s2 | [30,40] | Down | 125 |

结果：

| 口径 | 判定 |
|---|---|
| `trend_third_class_in_c` | 跳过 `(s0,s1)` 的方向失败，后扫 `(s1,s2)`，判 **通过**，证书端点 `(30,40)` |
| 补充十五固定首对 | 固定 `(s0,s1)`，方向元组不是 `(Up,Down)`，判 **`same_direction`**；不得用 `(s1,s2)` 回填 |

这正是“同向续行挡道”：扫描器把第二条同向续行重新解释成一个新 leave；旧框口径认为首对已经失败。

### 1.3 数值案例 B：首对价格失败，远处又出现合法对

统一设：

```text
B.end=10, B核心=[100,120], 趋势方向=Down, candidate/as_of=50
```

| 段 | 区间 | 方向 | 终点价 | 相对核心 |
|---|---:|---|---:|---|
| s0 | [10,20] | Down | 90 | leave 严格下破 ZD |
| s1 | [20,30] | Up | 105 | 回到核心，首对失败 |
| s2 | [30,40] | Down | 80 | 再次严格下破 |
| s3 | [40,50] | Up | 95 | 未回核心 |

结果：

| 口径 | 判定 |
|---|---|
| `trend_third_class_in_c` | 跳过 `(s0,s1)` 的价格失败，后扫 `(s2,s3)`，判 **通过**，证书端点 `(40,50)` |
| 补充十五固定首对 | 固定 `(s0,s1)`，因 `105 >= ZD=100` 判 **`retest_reentered`**；远对不能补证旧框 |

### 1.4 可复用边界

可复用的不是当前扫描器，而是其下方的**单对纯几何叶子**：

```text
grade_pair(B, trend_dir, leave, retest)
    -> pass | same_direction | leave_not_outside | retest_reentered
```

候选对选择必须留在调用者：

- T3-in-c / 补充十五：固定旧框首对，不后扫；
- 若别的诊断确实需要“全窗召回上界”：可继续扫描，但名称和消费者必须显式区别，不能再叫同一规范谓词。

`recursive_tower.rs:473-512,530-664` 已有召回审计原子和“全窗找成功/最深失败”的先例；该路径自己也声明是
`audit_cp_recall_upper_bound`，正好说明“召回审计扫描”与“生产固定首对”应分名分。

**题 1 结论**：不能直接接入；只能共享严格单对价格/方向判定叶子。现有扫描器在两个要求的反例中均会用远对判真，补充十五分别判 `same_direction`、`retest_reentered`。

## 2. LevelView T3 与一类流的对象、级别、时点关系

### 2.1 两条流的真实对象

| 维度 | `judge_first_cached` 一类流 | LevelView T3/趋势确认流 |
|---|---|---|
| 输入对象 | classifier 的 `Center`、`Segment`、A 段预算、当前 `seg` | `ExactThreeProjection`、`MoveBlock`、`LowerLeg`，再投影为 `Segment` |
| 级别 | 调用者当前 classifier 级别；函数本身不带 `level` 字段 | `assemble_level_view`/provider 明示 level，使用本级 block 与下一级 legs |
| B 对象 | `nearest_confirmed_center` 所属趋势块的最后中枢 | projection block 的 `end_center` seed |
| C 下界 | 针对当前候选段重算的 `λ_C` | provider 先找 B 后第一个同向 `c_terminal`，据此求较早 `c_start` |
| 右界/时点 | 当前候选 `seg.end_index` | 查询快照 `as_of`；`trend_confirm_time` 返回全合取首次成立的 `t*` |
| T3 角色 | 当前未验 | T4∧T3∧T2∧T5 中的必要项 |
| 输出 | `BspPoint`，包含六 bit、owner、破位方向、force | `DivergencePair` / `NestCandidateEventExt`，供 LevelView/nest admission |

代码证据：

- `signal.rs:1159-1217`：一类只在 `decompose` 的 `Trend` block 内调用；`c_start_entry` 针对当前 `seg.start_index` 计算。
- `signal.rs:1392-1469`：一类、盘背证书、当前相邻对三类是三个独立分支；一类并不消费 LevelView。
- `level_view.rs:528-635`：`trend_confirm_time` 先验 T4，再调用 T3 扫描器，之后渐进验 T2 与 T5，并返回首个全合取 `t*`。
- `level_view.rs:905-977`：D2 provider 从 projection seed 取 B，先冻结最早同向 `c_terminal`，再生成完整 `seg_c`。
- `level_view.rs:1045-1197`：LevelView 由 exact-three projection、move blocks、lower legs 和 `as_of` 组装。
- `backtest/admission.rs:654-780`：生产 nest gate 逐级组装 LevelView，并只吸收已确认的 nest 事件；它不是 BspPoint 生产入口。

因此，“同为级别 ℓ”只能说明二者**意图上**讨论同级 B/c 关系；不能推出 B 是同一个持久对象，也不能推出候选时点相同。只有在 projection seed 与 classifier center 一一对应、lower legs 与 classifier segments 同构、查询 `as_of==seg.end` 时，数值才可能对齐；当前类型系统没有把这些等式写成契约。

### 2.2 方案比较

#### 方案 A：一类流直接复用 LevelView 的完整判定

耦合代价高：

1. `judge_first_cached` 必须获得 projection 版本、block、lower legs、DIF/HIST、`as_of` 和 level identity；
2. 一类点的确认时点会从当前 `seg.end` 变成 LevelView 的 `t*`；
3. 一类流被迫连带消费 T4、T2、T5，而 #586 只研究 T3-in-c；
4. admission/nest 的版本与完成态策略会进入 Bsp 热路径；
5. 零 bit 结构负样本、resume/full bit-exact、digest 与消费者时序都要重新审计。

优点只是“一个现成函数名”，但实际复用的是错误粒度；它不是低成本合流。

#### 方案 B：两条流各自继续实现完整 T3

短期改动小，长期双轨漂移最高。当前漂移已经发生：

- LevelView 的 `trend_third_class_in_c` 允许后扫替代；
- 补充十五的 `historical_bound_third_cert` 固定首对；
- `recursive_tower` 的 recall audit 又是召回上界扫描；
- 三者都使用相近的“第三类”词，却有不同候选选择语义。

#### 方案 C：保留两条编排流，共享一个规范固定首对分级器（推荐）

```text
classifier/Bsp orchestration ─┐
                              ├─ select_frozen_first_pair(...)
LevelView orchestration ──────┘       └─ grade_pair_strict(...)
```

共享范围只包括：

- B 的四边；
- 趋势方向；
- 固定 leave/retest 身份；
- 五桶分级与通过证书。

各流仍自己负责：

- B 身份如何从其投影取得；
- 何时取快照；
- T2/T4/T5、力度与事件时点；
- 输出 `BspPoint` 还是 nest event。

需要加两类契约测试：

1. 同一手构 B/segments/candidate_end 输入，两流的 T3 grade 与 witness 必须相同；
2. projection 与 classifier 对象无法证明同构时，显式报“对象不可对拍”，不得静默宣称等价。

**题 2 结论**：两流不是同一判定对象/时点的现成同义入口。不要让一类流复用 LevelView 全合取；保留各自编排，只共享固定首对分级器，能以较低耦合消掉最大漂移源。

## 3. 窗口坐标：同坐标系，不同含义

### 3.1 `λ_C` 的含义

`rust/src/theta_v0/classifier/divergence.rs:794-851` 明确定义：

- 坐标：L0 `source_index`；
- 窗口：B 右侧到当前候选段 `until_start`；
- 分界：最后一条反向且回到 B 核心的段；
- `λ_C`：该分界之后第一条同向段的起点。

因此它是**当前局部离开 episode** 的力度积分起点。`judge_first_cached` 用它构造
`I(C)=[λ_C,seg.end_index]`，用途是 MACD/force 比较。

### 3.2 LevelView `c_start` 的含义

`level_view.rs:929-956` 先找 B 右边截至 `as_of` 的**第一条**同向 `c_terminal`，然后用
`departure_move_c_start(..., c_terminal.start_index)` 求 `c_start`。因为上界就是第一条同向段的起点，
后续回中枢不会把这个值推进；其后 `c_end` 再扩展到 `as_of` 前最后一条同向段。

所以两者：

```text
数值类型：都是 usize/source_index
λ_C：      随当前候选及此前最近 reentry 推进的局部 episode 起点
c_start：  以最早同向 departure 锚定的完整/全离开窗口左端
```

`recursive_tower.rs:1167-1210` 已把这一区别写进数据模型：
`c_episode_start/c_episode_interval` 与 `c_interval_full` 分列，并明言兼容字段 `enter_src`
不得与完整 `c_start_full` 建立相等公理。

### 3.3 数值差异案例

设：

```text
B.end=10, 核心=[100,120], 趋势=Up, 当前候选结束=60
```

| 段 | 区间 | 方向 | 终点价 | 作用 |
|---|---:|---|---:|---|
| s0 | [10,20] | Up | 130 | 固定 leave |
| s1 | [20,30] | Down | 125 | 固定 retest，未回核心，T3 成立 |
| s2 | [30,40] | Up | 140 | 续行 |
| s3 | [40,50] | Down | 115 | 回到核心，切断当前 episode |
| s4 | [50,60] | Up | 150 | 当前一类候选 |

此时：

```text
LevelView c_start = 10
当前候选 λ_C     = 50
```

结果：

| 窗口 | 结果 |
|---|---|
| 扫描 `[c_start=10, as_of=60]` | 命中 `(s0,s1)` |
| 扫描 `[λ_C=50, seg.end=60]` | 只有 `s4`，无回试，`None` |
| 补充十五旧框首对，截至候选 60 | 固定 `(s0,s1)`，通过 |

可见把 `λ_C` 当作 T3-in-c 左界会把已经发生且属于完整 c 的第三类证书丢掉；把 LevelView 的全局
`as_of` 直接带进早期 Bsp 候选又可能让未来对给过去候选补证。

### 3.4 给 spec 的窗口定义

对 `judge_first_cached` 候选时点，建议定义为：

```text
冻结对象：B = last_center
因果右界：candidate_end = seg.end_index
候选 leave：第一条 start_index >= B.end_index 且 end_index <= candidate_end 的本级完成走势
候选 retest：leave 后紧随且 end_index <= candidate_end 的本级完成走势
```

换言之，工程上不是在“`λ_C→seg.end`”与“任意快照的 `c_start→as_of`”之间二选一，而是：

```text
旧框/完整 c 左锚（实质为 B.end 后首条走势） → 当前候选 seg.end
```

若调用者已有 `c_start_full`，它必须被证明等于这条固定 leave 的 `start_index`，只能作身份交叉校验；
`λ_C` 继续只服务 episode 力度；LevelView 的 `as_of` 在与 Bsp 对拍时必须收窄为该候选的
`seg.end_index`。这是工程建议，不冒充第 37 课直接给出的数组边界。

**题 3 结论**：两值同属 source-index 坐标，但 `λ_C` 是局部 episode，LevelView `c_start` 是较早完整窗锚。T3-in-c 应按冻结 B 的首对、截至当前 `seg.end` 判，不能从 `λ_C` 起扫，也不能用未来 `as_of` 回填过去。

## 4. 原文形态覆盖：连续缺口、单腿 c 与盘背

### 4.1 原文边界

- `docs/chanlun/text/blog/037-第37课.md:16-22`：标准趋势只讨论同级别 A/B；`b` 可小于次级别；
  `c` 必须是次级别走势并包含对 B 的第三类，且要创新高/低；否则归盘整背驰语境。连续缺口是
  `c` 可能尚未完成、最终延伸为次级别走势的极强形态，不是“免除 T3”的例外。
- `docs/chanlun/text/blog/018-第18课.md:62-66`：中枢破坏需要次级别离开及随后回试不重回。
- `docs/chanlun/text/blog/020-第20课.md:60-62`：回试必须是第一次回试。
- `docs/chanlun/text/blog/017-第17课.md:38-46`：一个中枢为盘整，至少两个同向中枢才构成趋势。
- `docs/chanlun/text/blog/027-第27课.md:64-68`：盘背点在更大观察尺度可有类似一类的位置意义，
  但不是这里的标准趋势一类对象。

### 4.2 连续缺口/单腿 c 的机器结果

数值例：

```text
B.end=10, 核心=[100,120], 趋势=Up, candidate/as_of=20
gap_leg = [10,20], Up, end_price=150
```

| 口径 | 当前快照结果 |
|---|---|
| `trend_third_class_in_c` | 窗内只有一条，`windows(2)` 为空，`None` |
| 固定首对分级 | leave 已有、紧随 retest 尚无，`missing_retest` |

结论是“当前尚不可证”，不是“永久判负”：后续完成紧随回试后，重新在新的候选时点评估同一冻结 B。
但若生产投影把连续缺口拆成多条连续同向走势：

- 固定首对会在第一、第二条处落 `same_direction`，不得后扫；
- 扫描器可能把后面某条同向段当新 leave，再用更远回试判真。

这再次表明投影单位必须作为 spec 输入契约；“缺口很强”不能替代完整走势与第三类证书。

### 4.3 盘背是否误入

当前一类入口的双中枢趋势前提确实挡住**单中枢盘背**：

1. `signal.rs:1159-1217` 先用 `decompose` 产生局部 `center_trend_gate`；
2. 只有 `gate_dir=Some((pos,dir))` 才调用 `judge_first_cached`；
3. 注释和索引关系要求当前 block 为 `Trend` 且 `pos>block 首位`，于是存在同块前一中枢；
4. `signal.rs:1439-1450` 的盘整分支另走 `judge_pan_div`，只产 `PanDivCert`，不产 `BspPoint`、不置六 bit。

因此：

```text
单中枢 Consolidation → 不进 judge_first_cached → 不会误进 T3-in-c 一类合取
```

但必须照实保留边界：

```text
已有两个同向中枢、满足当前 Trend gate，但完整 c 的固定首对 T3 失败
```

这种对象当前仍可进入 `judge_first_cached`。它究竟叫“算法一类/类一”还是继续叫“一类但非标准趋势 c”，
正是 #584 的 A/B 核心裁定；不能用“单中枢已挡”偷换成“所有盘背域都已挡”。

**题 4 结论**：单腿/连续缺口在缺少紧随回试时均不能证明 T3；固定口径给出 `missing_retest` 或投影拆分后的 `same_direction`。单中枢盘背已被 Trend gate 挡住，但两中枢无 T3 的命名/分流仍待 #584。

## 5. 可进 spec 的分档判据

### 5.1 输入契约

```text
T3InCInput {
    level_identity,          // 外置级别身份；用于防止跨级混配
    frozen_b: Center,        // 被 c 离开的最后中枢 B，四边冻结
    trend_direction,         // Up/Down
    level_moves[],           // 同一级、source_index 升序、带完成态的规范走势
    candidate_end,           // 当前一类候选 seg.end_index；因果右界
}
```

前置要求：

1. 调用者已证明当前对象属于双中枢 `Trend(trend_direction)`；
2. `frozen_b` 与当前 `last_center` 是同一 owner；
3. 输入单位是该级别的“走势/段”规范投影，不把 bar、分型混入；
4. 对象序列满足完成前缀：不得越过一条未完成走势去选更远的已完成走势；
5. 只允许使用截至 `candidate_end` 已可见的完成事实，禁止前视。

### 5.2 五桶与通过证书

```text
T3InCGrade =
    Pass {
        leave_interval,
        retest_interval,
        point_source_index,  // retest.end_index
        side
    }
  | Fail {
        bucket,
        evaluated_at,
        leave_interval?,
        retest_interval?,
        actual_directions?
    }

bucket =
    missing_leave
  | missing_retest
  | same_direction
  | leave_not_outside
  | retest_reentered
```

精确定义：

| 桶 | 条件 | 备注 |
|---|---|---|
| `missing_leave` | 截至 `candidate_end`，不存在 B 右边第一条完成走势 | 不得凭未完成腿猜方向/价格 |
| `missing_retest` | leave 存在，但其紧随一条不存在或尚未完成 | 单腿 c 落这里 |
| `same_direction` | 实际方向元组不等于 `(trend_direction, flip(trend_direction))` | 五桶约束下统一承载 leave 错向、retest 同向等方向失败，并保留实际元组 |
| `leave_not_outside` | 方向合格，但 Up 时 `leave.end_price <= B.zg`；Down 时 `leave.end_price >= B.zd` | 严格越界，等号失败 |
| `retest_reentered` | leave 已越界，但 Up 时 `retest.end_price <= B.zg`；Down 时 `retest.end_price >= B.zd` | 严格不回核心，等号失败 |

### 5.3 合取顺序与伪代码

```text
fn grade_t3_in_c(input) -> T3InCGrade:
    B   = input.frozen_b
    end = input.candidate_end
    dir = input.trend_direction

    leave = first move where move.start_index >= B.end_index
    if leave is absent or leave.end_index > end or leave is incomplete:
        return Fail(missing_leave)

    retest = the immediately following move in the same level projection
    if retest is absent or retest.end_index > end or retest is incomplete:
        return Fail(missing_retest, leave)

    if (leave.direction, retest.direction) != (dir, flip(dir)):
        return Fail(same_direction, leave, retest, actual_directions)

    if dir == Up   and leave.end_price <= B.zg:
        return Fail(leave_not_outside, leave, retest)
    if dir == Down and leave.end_price >= B.zd:
        return Fail(leave_not_outside, leave, retest)

    if dir == Up   and retest.end_price <= B.zg:
        return Fail(retest_reentered, leave, retest)
    if dir == Down and retest.end_price >= B.zd:
        return Fail(retest_reentered, leave, retest)

    return Pass(
        leave_interval  = [leave.start_index, leave.end_index],
        retest_interval = [retest.start_index, retest.end_index],
        point_source_index = retest.end_index,
        side = side_from(dir)
    )
```

`missing_leave/missing_retest` 是 `evaluated_at` 时点的“尚不可证”，当那条**原定紧邻对象**后来完成时可以
重评；一旦固定首对已经完成，方向或价格失败就是该冻结 B 的终态失败。无论哪种状态，后续远对都不能
改变旧对象的 bucket；若业务要跟踪新的 B/c 对象，必须显式创建新 owner，而不是在旧对象里后扫。

#565 观测报告
`.chanlun/review-results/reset-before-third-obs-20260728.md:122-132`
已经按同一五类给出 58 例静态分布中的失败数：

```text
missing_leave=8
missing_retest=8
same_direction=5
leave_not_outside=7
retest_reentered=3
```

这些数只证明五桶在既有样本上非空，不证明预测价值或完整召回率。

### 5.4 与现有 `BspPoint` 的承载

现有字段能够承载：

| 字段 | 可承载内容 | 不可偷载内容 |
|---|---|---|
| `source_index` | 当前一类候选确认时点 `candidate_end` | 不能改成 T3 retest 时点，否则点时序变化 |
| `center` | `OwnerRef::Center(B)`，即冻结 B owner | 不能只靠相同四边猜 level/object identity |
| `struct_break_dir` | 一类结构破位侧 | 不能编码五桶 |
| `force` | A/C 力度 proxy | 不能编码 T3 方向/价格失败 |
| `bits.buy1/sell1` | 保持当前六 bit 语义 | 不能因 T3 pass 置 `buy3/sell3` |
| `bits.third_class_entry` | 当前点本身为第三类时的完整身份 | 不能拿来表示“另一个 T3 位于本一类点的 c 内” |

代码锚：

- `bsp.rs:133-188`：`BspPoint` 只有 source、六 bit、pivot、owner、`struct_break_dir`、force；
- `types.rs:191-237,281-294`：`BspBits` 的结构相等、Debug 和 `class_index` 只读六个 bool；
  `third_class_entry` 是 schema-only 归因旁载，不是第七 bit；
- `recursive_tower.rs:1065-1076` 的 `ThirdClassInCp` 已给出 B id、leave/retest identity、区间、
  点位和 side 的证书形状先例，但它依赖 `ElementId` 生命周期，且当前 Bsp 主路径不保证携带该对象。

推荐的最小新增承载是一个**不参与六 bit 分类**的旁载：

```text
BspPoint.type1_caliber: Option<Type1CaliberEvidence>

Type1CaliberEvidence {
    b_owner,
    evaluated_at,
    grade: T3InCGrade,
    first_family: TrendFirst | ClassFirst, // 名称/映射由 #584 裁定
}
```

承载规则：

1. `bits.buy1/sell1/buy2/...` 与 `class_index()` 逐位不变；
2. T3-in-c 通过只写 witness，不置 `buy3/sell3`；
3. T3-in-c 失败仍可旁载五桶，避免为了诊断删除 P2-R2 的结构负样本；
4. `center`、`source_index`、`struct_break_dir`、`force` 保持原义；
5. `PartialEq`、去重、序列化、Debug/digest 是否纳入旁载必须在 spec 明写并版本化：
   - 若作为 schema-only 归因，可仿 `BspBits.third_class_entry` 排除六 bit 相等与 `class_index`；
   - 若 Debug/digest 要显示新证据，应诚实升版本和重算 golden，不能一边变字段一边宣称字节形状未变。

不推荐平行 `HashMap<(source_index,center,side), grade>`：它避免改结构，但跨级同坐标、resume/full 重建和
去重后都可能失联，消费者矩阵还要另建 join 契约。若 #587 证明所有消费者只需离线诊断，才可选择独立
diagnostic event；否则点上旁载更稳。

### 5.5 一类流中的合取位置

在不代替 #584 改六 bit 的前提下，建议执行顺序是：

```text
Trend gate
→ broke last B
→ A/C 可配对
→ 037:20 创新极值
→ grade fixed-pair T3-in-c at candidate_end
→ A/C force / divergence
→ construct existing BspPoint
→ attach Type1CaliberEvidence
```

理由：

- 前四项失败时，本来就不是现有一类结构候选，无须制造 T3 五桶；
- 在结构候选域内分级，五桶的分母稳定；
- T3 与力度是并列证据，T3 失败不应隐式阻止现有力学负样本构造；
- #584 再决定 `TrendFirst/ClassFirst` 如何消费 grade，而 #586 不抢改六 bit。

**题 5 结论**：固定 B 首对、候选时点封顶、严格方向和价格，按五桶终止且禁止后扫。现有字段可复用 owner/时点/side，但不能承载 witness 与失败桶；应加 schema-only 一类口径旁载，六 bit 与 `class_index` 不动。

## 6. Spec 就绪度与前置关系

可直接进入 spec 的 #586 子节：

1. 输入对象与同级完成走势约束；
2. 冻结 B、固定首对、候选时点右界；
3. 五桶的顺序、严格不等式和“不后扫”规则；
4. 通过证书的 leave/retest 区间；
5. 不改六 bit、不得复用 `third_class_entry` 的承载铁律；
6. 两条编排流共享单对分级器，而不是一类流复用 LevelView 全合取。

尚不能由本报告直接封进完整 #589 的部分：

- #584：T3 失败的一类候选最终叫 `ClassFirst`、旧混合名还是其他名；
- #585：多窗口/信号集合影响；
- #587：每个消费者需要 pass witness、failure bucket 还是只要 subtype；
- #588：与 `pan_div_diag` 的所有权和互斥/并列关系；
- Debug/digest/serialization 的版本策略。

截至本报告收口时，`gh issue view` 复核 #584、#585、#586、#587、#588、#589 均为 `OPEN`；
下述“不可定稿”是依赖状态事实，不是对这些票的完成度作臆测。

因此准确状态是：

```text
#586 判据子章节：可直接进 spec
#589 完整实施 spec：暂不可直接定稿
```

## 7. 复现

演示脚本（仓外）：

```text
/tmp/research-586/t3_caliber_demo.py
```

运行：

```bash
python3 /tmp/research-586/t3_caliber_demo.py
```

脚本含 4 组断言：

1. 同向续行挡道；
2. 首对价格失败后有远处合法对；
3. LevelView `c_start` 与当前 `λ_C` 分叉；
4. 连续缺口/单腿只有 leave、没有 retest。

它只验证本文的确定性选择语义，不替代 Rust 单测、真实 replay、wf8 统计或异质审查。

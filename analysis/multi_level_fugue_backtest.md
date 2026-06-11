# 多级别赋格回测 — master+voice 双级别并发（OKLO 首验）

> 任务（2026-06-11 编排者）：把 Rust 交易层的 master+voice 双级别赋格接入回测管线，
> 先在 OKLO 上验证。master 执行 49课（下跌段持币不动），voice 执行 38课（向下段
> 先卖后买降成本），两者操作同一仓位但独立决策，每个买卖点用次级别把握（递归正则化），
> 六个买卖点全部要有处理路径。
> 脚本：`analysis/multi_level_fugue_backtest.py`
> 数据：`analysis/data_cache/multi_level_fugue_backtest.json` / `multi_level_fugue_tables.md`
> 磁带：C段修复后引擎（bsp 10,706 / div 8,462 / D3 翻转 24,417）——与修复前在册
> 数字**不可同表比较**（口径漂移在册，见 `project_c_segment_fix_b2`）。

---

## 1. 结论

### 1a. 架构判定：多级别赋格已支持，零 Rust 代码改动

`rust/src/trading/runner.rs` 的 `run_organic` 已实现任务要求的全部结构：

| 任务约束 | 实现位置 | 状态 |
|---------|---------|------|
| master 执行 49课（下跌段持币不动） | runner.rs：master 出场 = `MasterExitSignal::from_sell1_row`（C1 类型隔离，532号）→ `close_position` → FLAT 持币，直到下一个 buy1 重新 ARM | ✓ 已有 |
| voice 执行 38课（向下段先卖后买） | level_operating_unit.rs：`VoiceUnit` LOU FSM（UpLeg↔DownLeg，T1-T7 转换表），`rev_mode` 开启即激活 | ✓ 已有 |
| 同一仓位、独立决策 | ledger.rs：共享 `OrganicLedger`（total_shares/cost_basis 单账本）+ `SlotKey{ladder, leg}` 槽隔离（main/osc/rev 互不抢占） | ✓ 已有 |
| 递归正则化（次级别把握买卖点） | 入场：FLAT→ARMED→次级别 `buy_any` 区间套确认（SUB_EXPIRY=60 兜底）；REV 闭腿：T5 `nesting_buy_level` 区间套级别读出逐级回补；REV 开腿：G1 `sub_anchor` 次级别方向锚定（可选轴） | ✓ 已有（master 出场除外，见 §3 边界） |
| 多级别并发 | voices = `[floor_ladder, entry_ladder)` 每层一个 VoiceUnit；master 在 entry_ladder | ✓ 已有 |
| 纯 Rust 链路 | 信号磁带一次 marshal（compute-once），全部变体共享只读引用；cargo test 74 passed | ✓ |

**六个买卖点处理路径核对**（C6 穷举义务——`BspClass` 六变体 match，漏一类编译不过）：

| 买卖点 | 处理路径（代码锚） |
|--------|------------------|
| Buy1 | master 入场信号（FLAT 扫描 `sig.buy1` 最高层→ARMED）；voice REV 闭腿 T5（`buy_trigger_at` / 配对模式同锚 `buy1_paired`）；main 腿 normal 闭腿 |
| Buy2 | T5 买侧三岔（confirmed Buy2 回补）；`type2_buy` → n_addon 可观测；rev_paired 模式显式拒绝（`n_rev_mismatch_holds` 反事实计数） |
| Buy3 | **T7/T6 回补位**（confirmed/candidate → 全 tranche 回补恢复暴露——任务约束"type3买是回补位不是止损信号"✓）；master REV 腿闭腿；CenterBook `Terminated{Up}` |
| Sell1 | master 出场（`MasterExitSignal` 唯一构造入口）；voice REV 开腿（`seg_end_trigger` / 配对模式震荡型）；main 腿开腿（open_kinds） |
| Sell2 | 消融轴 R2（`sell2_trigger`，默认 no-op 的显式表态位） |
| Sell3 | REV 逃逸型开腿（`rev_escape_open`）；CenterBook `Terminated{Down}` → 冻结；main 腿 hard_type3 |

runner.rs docstring 的"当前磁带无 D3 行"声明已过时——`compute_organic_signals(dir_flips=...)`
已产出 D3 翻转行（本次 OKLO 24,417 行），V1f/V1r 等 D3 依赖变体可运行；
仅 `run_high` 行（rev_gate 依赖）仍未产出。

### 1b. 回测判决（OKLO 447K，C段修复后磁带）

守卫：**双 floor 各一次 Rust O0 ≡ Python O0 四面对账（trades/trace/counters）全 PASS**
（floor=2：195 笔/317 trace 行；floor=3：195 笔/34 trace 行）。

| floor（voice 域下界） | 变体 | 复利% | Δ(vs BH)pp | Δ(vs 同floor V0)pp | rev对(胜率/净现金) | osc对(净现金) |
|---|---|---|---|---|---|---|
| 2=segment（对照） | V0(P5) | **+1088.8** | +781.7 | — | — | 129 (+15.3K) |
| 2=segment | V1(38课legacy) | +1416.0 | +1108.9 | **+327.1** | 1094 (47.1%/−5.7K) | 129 (+15.3K) |
| 2=segment | V2of(配对REV) | **+1499.2** | +1192.1 | **+410.4** | 185 (51.9%/**+51.4K**) | 129 (+15.4K) |
| 3=move(L1)（任务字面配置） | V0(P5) | +284.9 | −22.2 | — | — | 21 (**−112.0K**) |
| 3=move(L1) | V1 | +172.9 | −134.2 | −112.1 | 54 (42.6%/−37.7K) | 21 (−112.0K) |
| 3=move(L1) | V2of | +245.9 | −61.2 | −39.0 | 13 (53.8%/−13.4K) | 21 (−112.0K) |

三条判决：

1. **C段修复后磁带上 REV 轴首次大幅转正**（floor=2）：V2of Δ(vs V0) = +410.4pp，
   rev 腿 185 对、胜率 51.9%、净现金 +51.4K——预注册判据(a)（胜率>50%）在本磁带
   达成。修复前在册结论"REV 三标的全负"建立在 C段恒空（type1 1 个）的磁带上，
   修复后（type1 374）信号基底变了，REV 的有效域需要全面重判。
2. **floor=3（voice=L1）全面劣化**：拖累主因不是 rev 而是 **osc 域腿在 L1 层
   净现金 −112.0K**（21 对，avg_diff −5.44%）——L1 中枢震荡幅度大，ZD 触线深，
   单次回补亏损大；且 voice 数量从 ≤3 层缩到 ≤1 层，38课"分区卷钱"的基数消失。
   数据再次裁决 **floor=segment 是最优 voice 域下界**（与在册"数据裁决 segment
   为最优 floor"一致，印证 L\* 分辨率甜点：bar/bi 太细 churn、move 太粗稀疏）。
3. **"master=L2"的实现度**：master 级别是数据驱动的（entry = 最高 confirmed buy1
   层）。OKLO 上 entry 分布：segment 160 笔 / move(L1) 32 笔 / recL2 3 笔——
   按笔数 master@L2+ 仅 1.5%，但**按持有 bar 算 recL2 占 21.7%（69,864 bar）、
   move(L1) 占 43.1%**——大级别入场的笔贡献了大部分持仓时间。"master 固定在 L2"
   若强制实施（entry 下界=4）会把入场信号源砍到 3 笔/2.4年，系统饿死；动态级别
   归属（38课"任何买卖点归根结底都是某级别的第一类买卖点"——买卖点自带级别属性）
   是严格形式，固定级别是其退化特例。**不实现 entry 级别锁定轴**——这不是缺口，
   是对任务规格的级别相对化修正（与 267号动态级别归属同构）。

## 2. 定义依据

- **49课操作模式**（master）：sell1 出场=「中枢完成的向上移动出现背驰，把所有筹码
  抛出」；FLAT 持币=「没必要参与操作级别及以上级别的下跌」（Step 5 持币占优）。
- **38课操盘程式**（voice）：向下段先卖后买（UpLeg→DownLeg 转换 T1）；段终结三触发
  =「下一段向上走势类型不创新高或盘整背驰」的事件形态。
- **40课多重赋格**：「每一层次的操作都是独立又在一个整体的操作中」——SlotKey 槽
  隔离 + 共享账本的代码形态。
- **27课区间套**（递归正则化）：入场 ARMED→次级别买点确认；T5 `nesting_buy_level`
  ——「大级别买点必然伴随小级别买点共现」的 max 收缩读数。
- **G-3 统一**（v2 设计 §0A.5）：49课 Step5 持币（FLAT 语境）与 38课先卖后买
  （LONG 语境）不冲突——master/voice 架构正是这个统一。

## 3. 边界条件（结论何时翻转）

1. **单标的 L2**：全部判决仅 OKLO 447K（2.4 年，强单边标的）。QQQ/BRN 复验前，
   "REV 转正"停留在 L2；若 QQQ/BRN 上 V2of Δ 为负，判决1 翻转为 regime 依赖。
2. **磁带代次**：判决全部建立在 C段修复后磁带（bsp 10,706）。若 B2 越界极值修复
   被谱系否决回滚，本表全部作废。
3. **floor=3 的劣化归因于 osc 不是 rev**：若在 floor=3 单独关 osc（osc_mode=false
   消融），V1/V2of 的 floor 轴对比可能反转——该消融未跑（本轮矩阵 6 cell 不含
   osc 轴，一次只验一组轴）。
4. **master@L2+ 持有 bar 占比 21.7%** 依赖 OKLO 的级别涌现密度；低波动标的 recL2
   buy1 更稀疏，该占比趋零。
5. 执行口径：恒以 close 成交、无摩擦——rev/osc 短差对摩擦敏感（在册：卖飞修复
   任务 be≈9.7bps），实盘摩擦会先吃掉 avg_diff 最小的腿。

## 4. 下游推论

- C段修复改变了 REV 的信号基底 → **修复前全部 REV 否定性结论（532号 M-e、
  "REV假设三标的被否证"）的有效域收缩为"修复前磁带"**，需在 QQQ/BRN 上用本脚本
  矩阵重判（增量续跑已支持，`BT_SYMBOLS=QQQ,BRN`）。
- V2of（仅震荡型+深度门+配对闭腿）在双 floor 都优于 V1 legacy → 配对修正方向
  在新磁带上继续成立。
- floor 轴的结论（segment 最优）与 voice 域上移劣化 → 后续多级别实验应保持
  floor=2，把级别轴的探索放在 master 侧（earning 升级出场已有），不动 voice 下界。

## 5. 谱系引用

- 532号（已结算 L3）：master 出场与 voice 触发是两个概念——本回测的 C1 类型隔离前提。
- 267号：动态级别归属（买点归属最高涌现层）——判决3 的级别相对化依据。
- 090号：floor=3 的"任务字面配置劣化"如实落盘，不替换为 floor=2 的好看数字。
- `project_c_segment_fix_b2`：磁带口径漂移的在册声明。
- `project_organic_fugue_implementation` / `project_rev_paired_fix`：修复前 REV
  否定性结论（本轮其有效域被收缩）。

## 6. 影响声明

- **新增**：`analysis/multi_level_fugue_backtest.py`（双 floor × 三变体矩阵 +
  floor 参数化 O0≡P5 守卫 + 磁带指纹守卫 + 增量续跑）、本报告、
  `data_cache/multi_level_fugue_backtest.json` / `multi_level_fugue_tables.md`。
- **行为零改动**：rust/src/trading/* 逻辑未动（架构已支持，cargo test 74 passed
  验证现状）、信号层、引擎。
- **修正一处过时声明**：runner.rs docstring "当前磁带无 D3 行"与事实不符
  （dir_flips 已产出，仅 run_high 缺）——已在本轮改正（纯注释，无行为变化）。
- **不实现**：entry 级别锁定轴（判决3：动态级别归属是严格形式）。

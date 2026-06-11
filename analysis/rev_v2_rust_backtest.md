# REV 配对修正回测——C 段修复后引擎（enginefix 代次）

2026-06-11 · 接续任务第四步。数据基底 = 修复A+B2 后引擎产出的新磁带
（OKLO tape_fp: bsp=10706/div=8462/flips=24417——此前"12209→10706 混表事故"中的
10706 即本代次磁带，事故根因是修复代码并发进入工作树，本次为该代次的正式在册）。

结果文件：`analysis/data_cache/rev_v2_paired_backtest_enginefix.json`（BT_TAG 通道，
与修复前在册结果物理隔离，不混表）。

## 1. 结论

**O0≡P5 bit-exact 守卫：OKLO/QQQ 双标的 PASS**（修复后引擎上 Rust 交易层与
Python 路径逐位一致，配对修正零侵入 legacy 路径的声明在新磁带上继续成立）。

变体对比（复利收益，floor=segment 级）：

| 变体 | OKLO 447K | Δ vs P5 | QQQ 728K | Δ vs P5 |
|---|---|---|---|---|
| BH 基准 | +307.08% | — | +174.64% | — |
| P5（基线） | **+1088.83%** | 0 | **+100.62%** | 0 |
| V1f（legacy REV+41课门） | +1146.45% | +57.6pp | +84.54% | −16.1pp |
| V2p（配对，开全 kind） | +670.44% | −418.4pp | +66.28% | −34.3pp |
| V2f（配对+θ_depth=1%） | +515.67% | −573.2pp | +94.36% | −6.3pp |
| **V2o（仅震荡型）** | **+1625.81%** | **+537.0pp** | +89.34% | −11.3pp |
| V2of（仅震荡型+θ） | +1499.20% | +410.4pp | +97.08% | −3.5pp |

腿级统计（修复后引擎）：

- OKLO 震荡型：n=376-381 对，胜率 52.4-52.5%，净现金 **+5.5万~+6.5万**（正贡献）
- OKLO 逃逸型：n=244-449 对，胜率 44.7-47.2%，净现金 **−14.2万~−14.3万**（强负）
- QQQ θ=1% 后配对几乎清空（n=3-4，量级失配），与修复前结论一致

## 2. 定义依据

- O0≡P5 四面对账：trades/metrics/legs/diag 逐字段一致（_bitexact_o0，
  organic_fugue_rust_check 管线）。
- V2 配对语义：kind-aware 开腿（type1/盘背卖开，不盲接 type3）+ 同锚闭腿
  （同锚中枢 type1 买 / ZD 触线 / type3 回补）+ θ_depth 深度门槛
  （`rust/src/trading/level_operating_unit.rs` rev_paired 路径，
  commit bd1e439914 实装）。
- 磁带：compute_organic_signals（Python 信号层）驱动修复后 Rust 引擎
  （maturin 重建于本任务）。

## 3. 边界条件

1. **修复前后结论的稳健性**：修复前在册（rev_v2_paired_backtest.json，
   bsp=12209 代次）与修复后（10706 代次）的变体排序一致——
   V2o 仅震荡型聚合最优、逃逸型全负、QQQ θ 失配清空。若 BRN（后台补跑中）
   出现排序反转，"排序稳健"声明翻转。
2. **θ_depth=1% 是单点**：未扫参。QQQ 清空说明 θ 与标的波动率量级耦合；
   换相对化 θ（ATR 归一）结论可能变。
3. **强趋势标的依赖**：OKLO V2o 的 +537pp 超额发生在 BH+307% 的强单边段；
   震荡市占比更高的标的上 osc 腿净现金可能转负（QQQ 已是负）。
4. 认识论等级 **L2→L3 过渡**：双标的（OKLO/QQQ）已复验，BRN 在跑；
   三标的齐后达 L3（多标的交叉验证）。

## 4. 下游推论

1. V2o（仅震荡型配对）是当前唯一在强趋势标的上稳定超越 P5 的 REV 变体——
   与修复前结论一致，修复后引擎放大了其优势（OKLO +537pp）。
2. 逃逸型开腿在两代次磁带上均为强负（OKLO −14万级）——
   逃逸型假设的否证在新引擎语义下继续成立，框架收缩方向不变。
3. 修复后引擎的递归层 type1/type2 激活（见 engine_c_segment_fix_verified.md）
   尚未被 REV 配对消费——同锚 Buy1 闭腿目前只在笔中枢/走势级取锚；
   高层锚（ladder4+ type1）的配对是新的可探索空位。

## 5. 谱系引用

- project_rev_paired_fix（配对成立、逃逸型否证、θ 失配、tape_fp 守卫）
- project_organic_fugue_v2_rust（O0≡P5 双标的 bit-exact 先例）
- project_bsp_gap_root_cause → engine_c_segment_fix_verified.md（本代次磁带的
  引擎语义来源）
- 090号（严格性）：BT_TAG 代次隔离 = 拒绝混表的结构化形式

## 6. 影响声明

- 在册新增/确认：`rev_v2_paired_backtest_enginefix.json`（OKLO/QQQ 全变体 +
  o0_guard PASS + tape_fp）；BRN 后台补跑中（pid 43171，
  日志 `analysis/data_cache/_enginefix_bt_brn.log`，完成后自动入册同文件）。
- maturin 重建了 .venv 中的 newchan_rust 扩展（修复后引擎对 Python 侧生效）。
- 不改动任何引擎/交易层源码——本步骤纯回测落盘。
- 旧在册结果（无 TAG 文件）保持原样：它们是修复前引擎语义的历史记录，
  与本代次物理隔离。

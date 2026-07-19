# p106 证书绑定组 × 前向收益分组实验（task #106，策略层级别标定）

日期：2026-07-17　数据：btc_1m_full.json（4,613,599 bar，2017-08-17..2026-05-31）
探针：`rust/src/bin/p106_cert_forward_pnl.rs`（新写，只读；自动发现 bin，未动 Cargo.toml——
与 p102/p104 同款先例，探针不依赖 `backtest_bin` feature）
输出全文：`/tmp/p106_full.txt`（P106_* 报表行）；进度：`/tmp/p106_progress.txt`（全量重放约 22 分钟）
复跑命令：

```text
cd rust && cargo run --release --bin p106_cert_forward_pnl -- \
  /Users/silencehan/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json \
  /tmp/p92_ckpt_dump.txt p7_inputs/trades.jsonl
```

## 0. 性质声明（090：声明与能力一致）

- 这是**策略层级别标定实验**：只测量「入场落在 nest 证书绑定窗口内的交易」与
  「其余交易」的前向收益分布差异（mfe / fin_ok / 派生 fin_ret）。
- **不碰判据**：未改 divergence/bsp/signal/level_view/nest.rs 任何一字；绑定规则逐字镜像
  p101 探针（`nearest_dt_tie` p106:236 ↔ p101_cert_bsp_tag.rs:102-121；
  `strength` p106:258 ↔ p101:123-129；judge_max 取钟 p106:182-233 ↔ p101:76-82）。
- **不做信号升格结论**：本报告全部分组差异是描述性统计，不构成入场过滤/加权规则；
  绑定组样本极小（见 §2），所有差异须带小样本与多重比较谨慎读。
- 派生字段说明：`fin_ret = dir × (exit − entry)/entry`（由 trades.jsonl 既有 entry/exit
  tick 价直接计算，探针 p106:305-310；trades 原生字段只有 mfe/fin_ok/dir_match）。

## 1. 验证门（全部 PASS，否则本报告不成立）

| 门 | 期望（锚） | 实测（/tmp/p106_full.txt） | 判 |
|---|---|---|---|
| BSP 事件总量 | 28,417 / 14,762 / 13,655（p100-cert-bsp-recon-20260717.md:74） | `P106_BSP events_total=28417 buys=14762 sells=13655` | PASS |
| L0–L4 分层 | L0 10298/9478 … L4 388/160（p100 报告 :75-79） | `P106_BSP_LVL` 5 行逐字一致 | PASS |
| 绑定强度分布 | A strong=29/weak=12、B strong=17/weak=8（p100 报告 :121,147 的 w240/w1440） | `P106_BIND_COUNT` 4 行逐字一致，ties=0、unbound=0 | PASS |
| 交易基线 | 40,001 笔、零长度 2,955（strict_nest_check.rs:23） | `P106_TRADES n=40001 mfe_null=2955` | PASS |

口径链：BSP 侧 = 生产因果塔 `ParseLayerIncr::append` + `classify_with_tower_incremental`
（p106:416-435，与 p101:157-178 同一提取循环；内联 IncrementalClassifier 先例
strict_nest_check.rs:53-86）；bar 加载镜像 `backtest::data::load_symbol`
（NaN→null、缺 OHLC 前收填充、untradable 不删除，data.rs:207-278 → p106:69-130）。

**附带产物（填 p101 缺口分析待填项）**：#105 口径 66 张证书的绑定物化——
`P106_BIND_SUMMARY bound=66 unbound=0 ties=0 distinct_bsp=41 multi_bsp=24`
（p101-ruling-gap-analysis-20260717.md:101 的「distinct_bsp/multi_bsp 待填」行就此回填：
66 张证书绑到 41 个去重 (bsp_bar, side) 键，24 键有多证书，最大单键 2 张——
无 91 口径那种 39 张末端簇，与 p103 的簇不存在结论一致）。

## 2. 核心数据事实 A：字面 V1 分组退化（n=1）——两套事件源不重合

任务字面口径：「交易入场 BSP 落在某张证书的绑定窗口内」，绑定规则镜像 p101
（证书 → 最近同侧 BSP，|dt|≤1440）。严格执行为：交易 `(entry_bar, dir)` 命中
41 个绑定键之一 ⟹ 绑定组。

- **实测：40,001 笔交易中仅 1 笔命中**（`P106_KEY_SUMMARY keys_with_trades=1 keys_total=41
  trades_tagged=1`）；该笔 = seg_idx=22557，2022-07-03 Long，落在 exec=3 top=3 trend
  Long 证书对（A/B 各一张，judge_max=2556965，dt=+344 weak）的绑定 bar 2557309
  （`P106_KEY bsp_bar=2557309`）。mfe=5.6e-4、fin_ret=−3.36e-3——**n=1 无分布可言**。
- 根因不是固定时移：全量 BSP (bar,side) 集（28,417 事件去重后）也只覆盖 343/40,001
  笔交易的入场（`P106_TRADES bsp_member=343 rate=0.008575`）；41 个绑定键 ±5 bar 内
  同向交易仅 4 笔（offset −5/0/+5 各 1/1/2 笔，无 +1 系统性偏移）；绑定键到最近同向
  交易距离 min=0 / median=38 / max=196 bar。
- 结论：**p7 交易的 entry_bar 与 p101 语义的 BSP 确认 bar 是两个不同事件源**。
  p7 交易是段翻转链（exit_k = entry_{k+1}、entry==close 40,001/40,001，
  strict_nest_check.rs:23），不是因果塔 seen-set 首见确认事件。「入场 BSP 落在证书
  绑定 BSP 上」这个分组谓词在当前数据上**结构性接近空集**——这不是证书稀少
  （66 张）所致，而是入场锚定义不兼容。若未来要按级别用证书标定交易，入场锚到
  BSP 确认 bar 的映射规则需要先成裁定（超出本任务范围，不越权定义）。

## 3. 核心数据事实 B：fin_ok / dir_match 恒真，零区分度

40,001 笔全部 `fin_ok=true`、`dir_match=true`（trades.jsonl:1 起全量；
`P106_GROUP` 各行 `fin_ok_rate=1.000000 dir_match_rate=1.000000`）。
两字段在本文件是常量，任何分组比较对它们都无信息——本实验实际比较维度只有
**mfe（前向最大有利振幅）** 与派生 **fin_ret（终值收益）**。
（mfe=null 的 2,955 笔 = 零长度交易，仅计入 null 计数不进分布。）

## 4. V2 窗口成员口径（敏感性分组）：521 vs 39,480

字面 V1 退化后，可操作的窗口口径：**存在同侧证书使 |entry_bar − judge_max| ≤ 1440**
（证书绑定窗口的成员资格，p101 条款 2 的窗口本身，p106:585-589）。同义变体
「绑定 BSP ±1440」给出几乎相同的 521 笔集合（窗口高度重叠），不重复列。

| 组 | n | mfe mean | p25 | p50 | p75 | p90 | fin_ret mean |
|---|---:|---:|---:|---:|---:|---:|---:|
| 窗内 in | 521 | 7.74e-3 | 1.65e-3 | 4.15e-3 | 8.62e-3 | 1.86e-2 | −7.79e-4 |
| 窗外 out | 39,480 | 6.24e-3 | 1.44e-3 | 3.39e-3 | 7.32e-3 | 1.44e-2 | +4.11e-5 |

- 窗内 mfe 系统性更高：mean +24%、p50 +22%、p90 +29%。两个方向各自成立
  （Long in/out mean 7.22e-3/6.04e-3；Short 8.10e-3/6.43e-3）。
- **终值收益无一致方向**：全样本窗内 fin_ret mean 为负（−7.8e-4）而窗外为正
  （+4.1e-5）；fin_ret 全样本中位数恰为 0（7.4% 交易 fin_ret=0，负值占非零 46.5%）。
- 覆盖形态：66 张证书窗口内交易数均 ≥1（0 空窗），最多 16 笔/窗（judge_max=3115054
  A Short）；521 笔散落于各窗而非单一簇驱动。

## 5. 样本内/外分段（机械切分，声明无预注册）

切分：seg_idx ≤ 28,000 为样本内（70%）、其余样本外——边界日 2023-09-22
（`P106_SPLIT`；本数据集无预注册 IS/OOS 切分，此为时间序机械切分）。

| 段 | 组 | n | mfe mean | mfe p50 | mfe p90 | fin_ret mean |
|---|---|---:|---:|---:|---:|---:|
| IS | in | 413 | 8.42e-3 | 4.46e-3 | 2.01e-2 | −1.11e-3 |
| IS | out | 27,587 | 7.01e-3 | 3.85e-3 | 1.61e-2 | +8.31e-5 |
| OOS | in | 108 | 5.07e-3 | 2.51e-3 | 1.28e-2 | +5.04e-4 |
| OOS | out | 11,893 | 4.44e-3 | 2.58e-3 | 1.02e-2 | −5.62e-5 |

- **regime 效应是最大混杂**：IS 全体 mfe mean 7.03e-3 vs OOS 4.44e-3（−37%），
  窗内交易 IS 占比 79.3%（高于全体 70%）——部分窗内 uplift 来自时段倾斜。
- 但分段后 uplift 仍在：IS 内 +20%（8.42 vs 7.01）、OOS 内 +14%（5.07 vs 4.44），
  OOS 内 mfe p50 反而略低（2.51 vs 2.58e-3）——uplift 集中在右尾（p90），
  中位层面 OOS 不支持。
- fin_ret 方向在两段间翻转（IS 窗内负 / OOS 窗内正）——终值维度无任何稳定结论。

## 6. 结论（全部描述性，不升格）

1. **字面实验不可执行**：p7 入场锚 ≠ BSP 确认 bar，「入场 BSP 命中证书绑定键」
   全样本仅 1 笔（§2）。按级别用证书做交易标定，前置缺一条「入场锚 → BSP 确认 bar」
   的映射裁定；在裁定落地前，任何「证书绑定交易」的收益结论都无样本基础。
2. **窗口口径下证书邻域 = 高振幅邻域**：judge_max ±1440 内的交易 mfe 右尾显著更厚
   （mean +24%、p90 +29%，IS/OOS 分段后 +20%/+14% 仍在），与「证书形成于级别完成的
   高波动区域」的结构直觉一致；但**终值收益无方向性**（fin_ret 符号跨段翻转）。
3. **不支持任何信号升格**：窗内 mfe uplift 是振幅事实而非方向事实；n=521（OOS 仅 108）、
   无多重比较校正、regime 混杂未完全排除。这些数字只能作为后续按级别标定实验的
   先验量级参考（「证书邻域振幅约为全体 1.2×」），不能作为入场/加权依据。
4. fin_ok/dir_match 常量事实（§3）提示：p7 trades 文件对分组实验的有效信息列
   只有 mfe 与 entry/exit——后续实验设计应直接声明用哪几列。

## 7. 复现性与纪律核对

- 探针 `rust/src/bin/p106_cert_forward_pnl.rs`（自动发现 bin；无需 Cargo.toml 注册。
  若主 agent 想显式登记，可选片段：`[[bin]] name = "p106_cert_forward_pnl"
  path = "src/bin/p106_cert_forward_pnl.rs"`——**不要**加 `required-features`，
  探针不依赖 backtest_bin）。
- 全量输出 `/tmp/p106_full.txt`；Python 独立复算（dump+trades 双侧，不经 Rust）
  与 P106_GROUP 的 full/v2/is/oos 行逐值一致（±末位舍入）。
- 主仓零写入（K 线只读）；生产源码零改动；未做任何 git mutation；
  新增文件仅本探针与本报告。
- 异常不静默：unbound/ties 若出现会逐张打 `P106_BIND ... bsp_bar=NONE`（本次 0 例）；
  多证书键全部 `P106_MULTI` 可见（24 键）。

—— #106 收口。

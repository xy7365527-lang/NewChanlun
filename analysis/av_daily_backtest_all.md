# AV 真实日线：数据就绪 + 引擎递归涌现（回测信号待 TV Replay 零前视序列）

**编排者最终裁定（2026-05-31）**：TV 静态标注**不能**做回测信号源——买卖点标注在
分型极点，但**成立时刻**在右侧确认之后；静态数据无法恢复成立时刻，lag=1/lag=3 任何
静态近似都错。唯一可接受方案：**TV Replay 的 first_seen_step + lag=1 + 真实 OHLCV**
（first_seen_step = 逐步回放中信号无前视首现 = 成立时刻），由独立 Replay session 产出。

本报告只产出**无前视争议的确定性内容**：AV 数据就绪状态 + 引擎递归涌现结构。

---

## 1. AV 真实日线 OHLCV（已保存，待 Replay 信号对齐）

| 标的 | AV 代码 | 性质 | 根数 | 起 | 止 |
|------|--------|------|------|----|----|
| 纳指 QQQ | QQQ | 真实本体 | 6684 | 1999-11-01 | 2026-05-29 |
| 恒生 HSI | EWH | EWH（港股 ETF，非 HSI 本体） | 6684 | 1999-11-01 | 2026-05-29 |
| 上证 SHCOMP | FXI | FXI（中资大盘 ETF，非 SHCOMP 本体） | 5444 | 2004-10-08 | 2026-05-29 |
| Brent 原油 BRN | USO | USO（原油 ETF，非 BRN 本体） | 5066 | 2006-04-10 | 2026-05-29 |
| Brent 真实原油 | BRENT | AV commodity（真实布伦特，退化日收盘） | 9898 | 1987-05-20 | 2026-05-26 |

> QQQ 为真实本体；HSI/SHCOMP/BRN 用代理 ETF（AV/Polygon 实测不提供指数本体，Polygon I:HSI/000001 返回 n=0）。BRENT commodity endpoint 为真实原油但仅日收盘单值。

## 2. 引擎递归涌现结构（L0 确定性，无前视）

| 标的(代理) | 日线 | 笔 | 线段 | 中枢 | L1走势 | L1买卖点 | 递归层数 | L2线段 | L2买卖点 |
|-----------|------|----|----|------|-------|---------|---------|-------|---------|
| 纳指 QQQ(QQQ) | 6684 | 482 | 50 | 12 | 2 | 9 | 2 | 0 | 0 |
| 恒生 HSI(EWH) | 6684 | 584 | 63 | 11 | 3 | 8 | 2 | 0 | 0 |
| 上证 SHCOMP(FXI) | 5444 | 499 | 51 | 11 | 3 | 7 | 2 | 0 | 0 |
| Brent 原油 BRN(USO) | 5066 | 417 | 53 | 9 | 3 | 6 | 2 | 0 | 0 |

### 走势分组涌现边界（用户指出的 bug 现象）

引擎在真实日线上的递归涌现呈现一致的**走势分组瓶颈**：
- **纳指 QQQ**：12 中枢 → 仅 **2 个 L1 走势** → L1 买卖点 9 个（{'type3-sell': 1, 'type3-buy': 8}），递归到 L2。
- **恒生 HSI**：11 中枢 → 仅 **3 个 L1 走势** → L1 买卖点 8 个（{'type3-sell': 1, 'type3-buy': 7}），递归到 L2。
- **上证 SHCOMP**：11 中枢 → 仅 **3 个 L1 走势** → L1 买卖点 7 个（{'type3-buy': 5, 'type3-sell': 2}），递归到 L2。
- **Brent 原油 BRN**：9 中枢 → 仅 **3 个 L1 走势** → L1 买卖点 6 个（{'type3-sell': 4, 'type3-buy': 2}），递归到 L2。

> **诊断**：`moves_from_zhongshus` 把十余个中枢压缩为 2-3 个走势（走势分组定义过严或有 bug），导致 L1 买卖点稀疏（个位数）且类型单一（多为 type3），递归在 L2 即终止（走势 <3 无法构建上层中枢）。**这正是不能用引擎自生买卖点做回测信号的原因**（信号太少、买卖不平衡），也是 TV 标注曾被用作左侧候选的根由。走势分组修复是独立的上游工作（与本回测数据准备解耦）。

## 3. 回测信号对齐接口（待 TV Replay 零前视序列）

回测**未产出**——按编排者裁定，需 Replay first_seen_step 避免前视。接入规格：

1. **信号**：Replay session 逐步回放，记录每个买卖点的 `first_seen_step`（无前视首现步）。
2. **执行价**：`first_seen_step + lag=1` 对应交易日的真实 OHLCV 收盘价（QQQ 用 av_QQQ_daily.json；其余待原标的数据或代理）。
3. **走势方向**：真实日线线段方向（本报告 §2 已确定性产出，线段层无走势分组 bug）。
4. **FSM**：cost_reduction_fsm 三阶段降成本（A 组）+ PH settle 门控（B 组），lag=1。
5. **数据就绪**：QQQ 真实 OHLCV 已存 `analysis/data_cache/av_QQQ_daily.json`，待对齐。

---

## 结果包（六要素）

1. **结论**：AV 四标的真实日线 OHLCV 已保存待用；引擎递归涌现结构确定性产出（§2）。回测信号源不用 TV 静态标注（前视偏差不可消除），待 TV Replay first_seen_step 零前视序列。
2. **定义依据**：递归涌现 = brn_level_analysis.analyze_levels_batch（包含→分型→笔→线段→中枢→走势→买卖点，缠论原文§5/§17）；前视偏差 = 买卖点成立需右侧确认（缠论第27/37课「对象否定对象」，a_online_persistence §10 因果 settle 定理）。
3. **边界条件**：若 Replay 产出 first_seen_step 序列 → 可接入 §3 接口产出零前视回测；若走势分组 bug 修复 → 引擎自生买卖点可能足量，届时可对照 TV 信号。
4. **下游推论**：引擎走势分组瓶颈（中枢多→走势少→买卖点稀疏）跨四标的一致，说明这是 moves_from_zhongshus 的结构性问题，非数据特异——修复它是恢复引擎自生信号（摆脱 TV 标注依赖）的关键路径。
5. **谱系引用**：前视偏差承接 a_online_persistence §10（settle = 顶/底分型右侧确认的因果形式化）；走势分组边界承接 recursive_backtest/unified docstring（引擎 confirmed 买卖点≈0 的结构性声明）；267/338号（cost_reduction_fsm，待 Replay 信号接入）。
6. **影响声明**：av_daily_backtest_all.py 由 TV 静态回测改为数据准备 + 涌现分析（移除被否定的静态信号回测）；新增/保留 av_{QQQ,EWH,FXI,USO,BRENT}_daily.json 缓存；复用 brn_level_analysis/av_fetch（未改动）；未改动引擎与 FSM。

## 缺口与诚实声明（no-patch-mentality）

1. **回测未产出**：TV 静态标注前视偏差不可消除，本脚本不伪造回测数字。等待 Replay first_seen_step——这是唯一零前视途径（编排者裁定）。
2. **代理 ETF ≠ 原标的**：HSI/SHCOMP/BRN 用 EWH/FXI/USO，AV/Polygon 不提供指数本体。§2 涌现结构是代理 ETF 自身的真实结构，不可外推到原指数/原油。
3. **走势分组 bug 未修**：中枢→走势压缩过度（§2），引擎自生买卖点稀疏。本报告诊断现象，不在此修复（独立上游工作）。
4. **BRENT 退化 OHLC**：commodity endpoint 仅日收盘，未纳入 §2 涌现（需 OHLC）；作真实原油数据保存待用。
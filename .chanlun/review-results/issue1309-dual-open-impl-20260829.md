# #1309 多空双开对称扩展：实现 + 预注册对照 + 数据卡点（2026-08-29）

> **状态**：实现已落地并验证（纯 Python 交易层）；回测跑批**卡在外部数据依赖**（沙盒无 DATABENTO_API_KEY，7 标的 1min 数据文件不在仓内）。
> **本票「正确」口径**：忠实执行 #1282 预注册，不是「实验必须过」——过与不过都原样报。
> **上游**：#1279（M1 实验线）；预注册判据 = #1282 冻结文本（跑前不改一字）。

## 0. 结论摘要（TL;DR）

1. **操作层对称扩展已实现**（#1309 要做 ①）：`fugue_alpha_diagnosis.run_swing_trading_dual`——长腿（次级别底背驰进多 + L2 顶背驰离场）逐位等价于既有 `run_swing_trading(MODE_NONE)`；空腿 = 联立门方向镜像（次级别顶背驰进空 `up_move_settled∧exit_div_ok`、L2 底背驰回补 `l2_flip_long∧entry_div_ok`）。双腿各自独立满仓、可同时在场（多空双开）。分解机制本就双向（`up_move_settled`/`down_move_settled`、`exit_div_ok`/`entry_div_ok`、`l2_flip_long`/`l2_flip_short` 信号字段已备），本票只接操作面，未改信号/引擎/分解层。
2. **双开百分位基线已实现**（#1309 要做 ②的随机基线）：`p3_random_gate_control.large_bootstrap_percentile_dual`——双腿各 N 笔 × 平均持有、entry 均匀随机、5000 次秩统计，组合口径与 `compute_dual_metrics` 一致（长+空）/2。
3. **回测 driver 已写好**（#1309 要做 ②③）：`analysis/m1_e_futures_dual_backtest.py`——7 标的一次引擎跑批，产出「双开百分位 + 7/7 判定 + 前后对照表」，结果原样写入 `.chanlun/review-results/issue1309-dual-open-backtest.{json,md}`。
4. **跑批卡点**（外部依赖）：沙盒无 `DATABENTO_API_KEY`/`DATABENTO_KEY`（环境变量核验 0 命中），7 个数据文件 `analysis/data_cache/*_1m_databento_10y.json` 不在仓内（被 gitignore），`newchan_rust`（Rust 扩展）未构建。回测**无法在沙盒执行**。解除条件 + 续跑命令见 §4。

## 1. 实现面（#1309 要做 ①）

### 1.1 联立门方向镜像（不另立判据）

| 腿 | 进场门（次级别背驰，操作级入场） | 出场/回补门（L2 背驰，趋势级） |
|----|--------------------------------|-------------------------------|
| 长腿（既有，逐位等价） | `down_move_settled ∧ entry_div_ok`（次级别底背驰进多） | `l2_flip_short ∧ exit_div_ok`（L2 顶背驰离场） |
| 空腿（#1309 新增，方向镜像） | `up_move_settled ∧ exit_div_ok`（次级别顶背驰进空） | `l2_flip_long ∧ entry_div_ok`（L2 底背驰回补） |

镜像规则：同一套判据，`down↔up`、`entry↔exit`、`l2_flip_short↔l2_flip_long` 三处翻转，合取结构（联立门）不变——与 #1282 Q2「同级别分解 + 联立门镜像，不引入『腿』概念」一致（空腿是方向镜像，不是另立一套判据；无宽严两档、无兜底，不触 #799/#804）。

### 1.2 P&L 口径

- 长腿逐笔 `pnl% = 出场/进场 − 1`；空腿逐笔 `pnl% = 进场/出场 − 1`（做空方向倒数），与既有长腿「无降成本」口径同构。
- 双腿各自独立满仓（各一份 INITIAL_CAPITAL），组合复利% =（长腿复利% + 空腿复利%）/ 2（按 2× 本金归一）。随机双开基线同口径合并。

### 1.3 代码落点

- `analysis/fugue_alpha_diagnosis.py`：`_run_swing_leg` + `run_swing_trading_dual` + `compute_dual_metrics`。
- `analysis/p3_random_gate_control.py`：`simulate_one_random_dual_gate` + `large_bootstrap_percentile_dual`（种子偏移 `DUAL_SEED_OFFSET=2000`，与单边 `BASE_SEED+1000` 空间隔离）。
- `analysis/m1_e_futures_dual_backtest.py`：新 driver（7 标的、增量落盘、报告生成）。

## 2. 验证面（沙盒可做部分）

- `python3 -m py_compile` 三文件全绿。
- 合成 BarSignal 磁带测试：`_run_swing_leg(signals, "long")` 与 `run_swing_trading(signals, MODE_NONE)` **逐位等价**（进出场点、pnl、reason 全同）；空腿方向正确（上涨序列空腿负收益、下跌序列空腿正收益）。
- `simulate_one_random_dual_gate` 与手工重算逐位一致（组合复利% 精确相等）；`large_bootstrap_percentile_dual` 结构同 `large_bootstrap_percentile`。
- driver 缺数据路径实测：打印 7 个缺失文件 + 解除条件，退出码 2。

**未做**：真实 7 标的跑批（缺数据，见 §4）。

## 3. 预注册冻结文本逐条对照（#1309 要做 ④）

对照 #1282 票面冻结文本（2026-08-29，编排者拍板 Q1/Q2/Q3）：

| 冻结项 | #1282 票面 | 本票执行 | 核对 |
|--------|-----------|----------|------|
| H | 操作层对称扩展至下跌侧（次级别顶背驰进空、L2 底背驰回补，联立门方向镜像） | `run_swing_trading_dual` 空腿三处镜像 | ✅ |
| 过判据（唯一） | 7/7 标的百分位全部越过各自随机中位（符号检验单侧 p=0.0078）；不过 = M1 落档「期货对称扩展不可证」同为有效结论 | driver 按 `分位>50` 计数 + 7/7 判定 + p=0.0078125；过与不过都原样报 | ✅ |
| 前后对照（Q3.1） | 原 E 单边百分位 vs 多空双开百分位，同标的同窗配对，逐标记录变化量 | 同 signals 同窗重算两套百分位，逐标 Δ | ✅ |
| 随机基线（Q3.2） | prereg-windows-v0 百分位法（含 #1281 补 ZN/6E 窗）= 真策略在随机进出分布中的排位 | `large_bootstrap_percentile_dual`（双腿各 N×H 随机进出 5000 次） | ✅ |
| 数据与窗口 | 7 标的 10 年 1min，窗口照 prereg-windows-v0 + §9 补录 | `SYMBOL_FILES` 全量 1min（prereg 表冻结值；与归档单边 `analysis/p3_futures_random_gate.md` 同窗） | ✅ |
| 纪律附款与排除项 | 跑前不搜索、不调参、不挑窗口；跑完结果原样入档；不做参数网格搜索、不加新对照品种、不中途改判据 | 判据/窗口逐字取自冻结文本，零改动；结果原样入档 | ✅ |

> 注：「数据与窗口」的「窗口照 prereg-windows-v0」在本票语境 = prereg 表 §1 冻结的数据范围（10y 快照 + ZN/6E §9 补录），与归档单边回测（`analysis/p3_futures_random_gate.md`，全量 1min）同窗——前后对照要求「同标的同窗配对」，故不另切 OOS 子窗；若编排层裁定改为 OOS 窗（2023-01-01→2025-06-30），走 §9 并告知，本 driver 的窗接口可一处改。

## 4. 卡点报告（外部依赖，复现证据 + 解除条件 + 续跑命令）

### 4.1 复现证据

```text
$ env | grep -iE "databento|api_key"   # 无 DATABENTO_API_KEY / DATABENTO_KEY（其余 key 与 Databento 无关）
$ ls analysis/data_cache/               # 仅 venue_fee_* 5 个文件，无 *_1m_databento_10y.json
$ git check-ignore analysis/data_cache/es_1m_databento_10y.json   # 命中（该目录被 gitignore）
$ PYTHONPATH=src:analysis uv run python analysis/m1_e_futures_dual_backtest.py
卡点：缺少期货 1min 数据文件（Databento 外部依赖，沙盒无 DATABENTO_API_KEY）：
  - analysis/data_cache/es_1m_databento_10y.json
  ...（共 7 个）
exit=2
```

`newchan_rust`（Rust 扩展）亦未构建：`uv run python -c "import newchan_rust"` → ModuleNotFoundError。

### 4.2 解除条件

1. Databento 订阅 key 注入沙盒环境（`DATABENTO_API_KEY` 或 `DATABENTO_KEY`，订阅档已覆盖 GLBX/IFEU/IFUS 免费 1min 档，见 `scripts/fetch_1m_databento_10y.py` 头注）；或
2. 编排层把 7 个数据文件（`es/gc/cl/zn/usd6e/brn/dx_1m_databento_10y.json`）放入 `analysis/data_cache/`（gitignored，不入仓）。

### 4.3 续跑命令（数据就位后）

```bash
cd /home/agent/workspace
# 1) 构建 Rust 扩展（若未构建）
cd rust && uv run maturin develop --release && cd ..
# 2) 数据就位（有 key 时拉取；或直接放文件）
DATABENTO_API_KEY=... PYTHONPATH=src uv run python scripts/fetch_1m_databento_10y.py all
# 3) 跑多空双开回测（7 标的，增量落盘可 resume）
PYTHONPATH=src:analysis uv run python analysis/m1_e_futures_dual_backtest.py
# 4) 产物
#    .chanlun/review-results/issue1309-dual-open-backtest.json
#    .chanlun/review-results/issue1309-dual-open-backtest.md
```

## 5. 交付物清单

- [x] 操作层对称扩展代码（长腿逐位等价 + 空腿方向镜像 + 双开 metrics）
- [x] 双开随机基线百分位函数
- [x] 7 标的回测 driver（按冻结判据，结果原样入档路径就绪）
- [x] 预注册冻结文本逐条对照
- [ ] 7 标的真实跑批结果（**卡点**：外部数据依赖，见 §4）

**待解除卡点后**：跑批 → `#1282` 票面贴结果与判定（过与不过都原样报）。

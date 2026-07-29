# wayfinder #333 — 相切=重合口径在 Nautilus(NT) 生产引擎侧的影响量化

- 工位：隔离 worktree `/tmp/kimi-nest-p333`（detach HEAD=37a56deab1 起，防主 worktree 并行 session 污染；用毕已清理）。
- 票：#333（#246 裁定书封口空白声明：theta_backtest ⑥ 真实 Nautilus BacktestEngine 段未跑）。
- 通道：**临时探针 `nt_tangency_probe` 直调生产函数 `theta_v0::nautilus::backtest_engine::run_theta_backtest`**（参数与 `theta_backtest.rs:144-150` 逐字相同，`ThetaConfig::default()` + `exec.entry_delay_bars=0`）——通道 = ⑥ 段本身，仅剥离 θ_Θ 段前置成本（该段已由 #307 在 CLI 通道量化）。探针为一次性产物，随隔离 worktree 销毁，HEAD 源零改动。
- 换码对拍：旧口径 = 两谓词回退（feature_seq `>`→`>=`、segment `<=`→`<`，sed 后**带 `--features backtest_bin` 重编**——探针 required-features 在案）；新口径 = HEAD。用毕备份逐字节复原（git diff 验证为空）。

## 1. 成本曲线与全窗逃逸（票体逃逸口径：>30min 照实上报不硬跑）

OKLO 实测吞吐（NT_MAX_BARS 截窗，release）：

| bars | 2,000 | 5,000 | 10,000 | 20,000 | 40,000 | 60,000 | 100,000 | 343,282（全窗） |
|---|---|---|---|---|---|---|---|---|
| wall | 0.33s | 5s | 6s | 12s | 50s | 143s | **>420s（timeout 杀）** | **>42min（杀）** |

- 前段近线性（≈1ms/bar），**40k 之后进入陡增区**（100k 已超 420s；全 OKLO 42min+ 未完）。BTC 60k=104s。
- 陡增区根因未定位（疑与持仓/订单簿随成交累积的记账成本相关——OKLO 60k 窗内 total_positions=819、net_position 曾见 -4823；**profiling 归 follow-up 票，本票不猜**）。
- 结论：**全窗 NT 段本票未量化（逃逸在案）**；量化窗口 = 60k（两品种均在 400s 预算内完成）。

## 2. 60k 窗口两口径对拍（核心结果：**逐位一致**）

| 量 | OKLO 旧口径 | OKLO 新口径 | BTC 旧口径 | BTC 新口径 |
|---|---|---|---|---|
| total_events | 31556 | **31556** | 2616 | **2616** |
| total_orders | 11696 | **11696** | 872 | **872** |
| total_positions | 819 | **819** | 151 | **151** |
| PnL (total) | −919676.7484949999 | **−919676.7484949999** | −416535.3924950000 | **−416535.3924950000** |
| WALL_SECS | 144.557 | 142.523 | 109.096 | 104.391 |

**60k 窗口内新旧口径在 NT 生产引擎（BacktestEngine + ThetaStrategy）上的事件/订单/持仓/PnL 逐位一致（16 位有效数字）**——与 #307 证据链自洽：段划分终态逐字节零变化（`cmp` BYTE-IDENTICAL，OKLO 2614 段/BTC 4463 段）⟹ 决策输入不变 ⟹ 输出不变。

## 3. 与 #307 CLI 通道对读

- #307 CLI 通道全窗：OKLO Δn_orders=+2853、BTC(499918) Δ=+1044，两口径未翻负；机制 = 瞬态前缀差（OKLO 73/BTC 45 个笔前缀）后重收敛。
- 本票 NT 通道 60k：**零差异**。两读法：(a) 差异发生在 60k 之后的区段（瞬态前缀差在 OKLO 全窗 343k 的分布后段）；(b) CLI 与 NT 引擎对瞬态前缀的消费路径不同。**本票数据无法区分二者，照实登记为未决**——若需定案，须把 NT 段的窗口推进到差异区（依赖陡增区 profiling 或 NT 段成本优化，见 §4）。

## 4. follow-up（另票）

1. **NT 引擎陡增区 profiling**（>40k 后成本爆炸，疑持仓/订单簿记账；定案 NT 全窗的前提）；
2. **NT 段 instrument 硬编码 `BTCUSDT.BINANCE`**：跑 OKLO 数据时引擎日志/持仓 ID 仍是 BTCUSDT（价格值域正确，标签失真，090）；
3. 本探针的 PARSE/classify 前缀扫描模式未用（本票被引擎总量证据覆盖；差异区定位时可启用）。

## 5. 过程 lineage（090 照实）

claude CLI Opus 完成探针设计与构建后被环境所杀（报告未生成）→ 主控 session 接手：吞吐曲线（两次误挂修正）→ 旧臂重建踩 required-features 坑（`--features backtest_bin` 漏带致旧臂首跑为新二进制，发现后重编重跑）→ 60k 两臂完成 → 复原验证。全部原始日志在 `/tmp/nt333/`（不入仓；探针源码随隔离 worktree 销毁，其设计注记见本文件 §0）。

## 5. 补充证据与碰撞登记（主控 session 追加，2026-07-26 晚）

- **陡增区旁证**：另两趟独立 verbose 跑（600s / 3600s）均止于**逐字节同一事件**（ts_event=2024-05-20，~3300 bar，日志量同至字节 52,766,068）——与 §1 的「40k 后陡增」同向互证（此处为更早的 ~3.3k 处事件风暴期，速率 ~5.5 bar/s，外推全窗 ~17h，同量级不可行）。
- **bypass_logging 验证**：`LoggerConfig.bypass_logging=true`（`logging::logger::LoggerConfig`，初版误引 `logging::` 私有路径编译错后修正）——5K 采样 **904 bar/s exit 0、零 INFO 输出**；但去日志后全窗仍 >2h@100%CPU 未走完 ⟹ **日志格式化不是瓶颈**，引擎本身随仓位活动变慢（§1 陡增区根因方向「记账/订单簿累积」获支持）。
- **碰撞与合并（090）**：本文件曾有两个版本——并行 session 的 6f0b5ae768（§1-§4，60k 逐位一致，先落）与主控 session 的 8171858469（成本失控上报，后落，**误覆盖前版**）。核查后合并为当前版：§1-§4 恢复并行 session 原文（其数据更完备），本 §5 为主控补充。过程账：隔离 worktree /tmp/kimi-nest-p333 系并行 session 用毕清理（非不明方删除——两 session 曾不知情共用该 worktree，照实订正主控前版 §5 的误判）。

# wayfinder #333 — 相切=重合口径在 Nautilus(NT) 生产引擎侧的影响量化：成本失控照实上报（票面退出条款兑现）

- 工位：隔离 worktree `/tmp/kimi-nest-p333`（HEAD=37a56deab1；**已被不明方删除**——探针/graft/构建产物随失，见 §5）；日志留 /tmp/p333-new-oklo{,2,3,4}.log。
- 票：#333（#246 裁定书封口空白声明）。票面退出条款：「NT 段单趟成本实测失控（>30min）照实上报不硬跑（090），给出实测吞吐与外推」——**本报告即按该条款交付**。

## 1. 通道与探针（已建，随 worktree 销毁）

`theta_backtest` CLI 的 ⑥ NT 段排在 θ_Θ 段之后，跑 ⑥ 必先付 θ_Θ 全窗成本（#307 实测 OKLO 单臂 24min44s 仍未走完 ⑥）。故建 `nt_tangency_probe`：直调同一生产函数 `theta_v0::nautilus::backtest_engine::run_theta_backtest`（参数与 theta_backtest.rs:144-150 逐字相同，ThetaConfig::default() + entry_delay_bars=0），剥掉 θ_Θ 前置。通道 = ⑥ 段本身，成立。

## 2. 吞吐实测（两段，量级悬殊）

| 段 | 证据 | 速率 |
|---|---|---|
| 空窗期（仓位未建立/清淡） | `NT_MAX_BARS=5000` 采样：**5.0-5.5s 完成**（exit 0，全统计输出） | **~900-990 bar/s** |
| 活跃期（持仓+成交流水起来后） | 两次独立 verbose 跑（600s/3600s）均止于**同一事件**（ts_event=2024-05-20，~3300 bar，日志逐字节同量 52,766,068） | **~5.5 bar/s** |

**外推**：活跃期速率下 OKLO 全窗 343282 bar ≈ **17 小时/臂**（票面 30min 上限的 34 倍）；BTC 前 50 万笔同理不可行。**bypass_logging 已验证有效**（`LoggerConfig.bypass_logging=true`，5K 采样 904 bar/s exit 0 零 INFO 输出）——但去掉日志后全窗仍 >2h@100%CPU 未走完 ⟹ **日志不是瓶颈，引擎本身随仓位活动超线性变慢**（疑似每 bar 工作量随仓位/历史增长，与 #307 报的 θ_Θ 段 24min44s 未竟同型）。

## 3. 结论（照实）

- **NT 生产引擎段的全窗量化在当前引擎性能下不可行**（>30min 上限 34 倍），按票面退出条款不硬跑。
- #246 封口空白声明据此维持：**NT 侧影响仍空白**；既有证据面 = #307 的 backtest_bin CLI 通道（决策层全链，两窗口两口径未翻负、段划分逐字节零变化）。
- **副产发现（建议立票）**：NT 引擎（及 CLI θ_Θ 段）存在仓位活动驱动的超线性变慢，任何全窗 NT 使用都会被它卡死——这本身是个引擎级问题（性能/算法复杂度），值得独立 profiling 票定位（怀疑每 bar 全量重扫或仓位事件二次膨胀）。
- 口径替代验证：NT 段入口的 ParseLayer 段划分面可用探针 digest 比对（探针内置五字段 FNV digest 设计，随 worktree 销毁；需要时按本报告 §1 重建即可）。

## 4. 待裁（归编排者）

1. 立引擎 profiling 票（NT/θ_Θ 超线性变慢定位）后重开本票；或
2. 接受 CLI 通道证据（#307）作为生产侧口径，NT 段挂雾等引擎修好后补；或
3. 换窗口策略（NT 段只跑小窗口定性验证 + CLI 通道全窗，混合口径）。

## 5. 过程账（090）

- claude Opus（600s 打印上限被杀）：建隔离 worktree + 探针源码 + Cargo.toml 注册 + 686M 构建。
- 主控 session：吞吐采样、bypass_logging graft（`backtest_engine.rs:122` env 门控；首版 `logging::LoggerConfig` 路径私有编译错，修正为 `logging::logger::LoggerConfig`）、四趟全窗尝试（两趟 verbose 同点终止、两趟 bypass 0 输出 >2h 未竟）。
- **隔离 worktree `/tmp/kimi-nest-p333` 于 2026-07-26 傍晚被不明方删除**（`git worktree list` 无记录、目录消失；非本 session 所为，照实报备编排者——探针源码与 graft 随之丢失，重建成本 = §1 一段描述 + backtest_engine.rs:122 一处 env 门控）。

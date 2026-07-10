# 严格区间套证书产量漏斗归因（2026-07-09）

## 运行范围与语义护栏

- 数据：`/tmp/codex-work-p7/analysis/data_cache/btc_1m_full.json`，**4613599** bar（2017-08-17 04:00:00 .. 2026-05-31 23:59:00）；`ThetaConfig::default()`，`l_max=6`，`min_parts_per_level=3`。
- 命令：`cargo build --release --bin strict_nest_check && ./target/release/strict_nest_check`；全量因果重放 1492.9s。
- 插桩只读终态 `Classification`、`CandDeltaEvent` 与装配结果；未修改 `divergence.rs`、`bsp.rs`、`signal.rs`、`recursive_tower.rs` 或 `nest.rs` 的任何判据/控制流。
- 列口径：背驰段谓词命中 = 终态分类 buy1/sell1 bit；`Cand^δ` 候选 = 塔上 `cand_delta=true` 事件（两列应因 P1 bit-exact 相等）；递降链配对成功 = 能把已从 L0 可达的 partial chain 延长到本级的相邻父子边；最终证书 = `assemble_certificates(events, 0, ℓ, terminal)` 产量。

## ① 各段 × 各级计数

| 级别 ℓ | 结构事件（辅助） | 背驰段谓词命中 | Cand^δ 候选 | 递降链配对成功 | 可达本级 Cand | 最终证书 N^δ_{ℓ↓0} |
|---:|---:|---:|---:|---:|---:|---:|
| 0 | 579 | 452 | 452 | — | 452 | —（基例 terminal=452） |
| 1 | 20 | 7 | 7 | 0 | 0 | 0 |
| 2 | 20 | 13 | 13 | 0 | 0 | 0 |
| 3 | 16 | 9 | 9 | 0 | 0 | 0 |
| 4 | 4 | 2 | 2 | 0 | 0 | 0 |
| 5 | 0 | 0 | 0 | 0 | 0 | 0 |

核对：P1 mismatch bar = **0**；L0 terminal 查无 = **0**；跨级证书合计 = **0**。

## ② 首个归零的段

首个归零发生在 **L0 → L1 的递降链配对**：L0 有 452 个可达 Cand 基例/partial-chain，L1 有 7 个 `Cand^δ=true` 候选，但满足 `同方向 ∧ parent.confirm_src ≤ child.confirm_src ∧ J_child ⊆ J_parent` 的可达相邻配对为 **0**。因此从该段开始所有 `N^δ_{ℓ↓0}` 完整证书均为 0；损失不发生在 P1 谓词→Cand 映射，也不发生在 terminal 查找。

## ③ 首个归零段最接近通过的 3 个样本

排序冻结为：失败原子条件数升序 → 三个数值门缺口 bar 总和升序 → `(完成时缺口, Sub 左界缺口, Sub 右界缺口, parent_idx)` 字典序；未调判据、未用价格/收益挑样本。

| 排名 | 父级 Cand（side, confirm, I(C)） | 子级可达 Cand（side, confirm, I(C)） | 通过条件 | 具体缺口 |
|---:|---|---|---|---|
| 1 | `L1 Short, t=345518, [344830,345518]` | `L0 Short, t=345578, [345518,345578]` | 方向、完成时递降、Sub左界 | **Sub 右界失败（子终点晚 60 bar）** |
| 2 | `L1 Long, t=2365194, [2364887,2365194]` | `L0 Long, t=2365281, [2365264,2365281]` | 方向、完成时递降、Sub左界 | **Sub 右界失败（子终点晚 87 bar）** |
| 3 | `L1 Short, t=2168349, [2167920,2168349]` | `L0 Short, t=2168532, [2168471,2168532]` | 方向、完成时递降、Sub左界 | **Sub 右界失败（子终点晚 183 bar）** |

## 结论

当前 BTC 1m / 默认 Θ / 完成时 / 趋势背驰-only adopted-default 有效域内，证书产量 0 的首因是 **L0→L1 相邻级递降+Sub 合取无一通过**。上游背驰谓词并非零产量，P1→Cand 也无损；最终证书阶段只是传播该首个零。该结论不外推到其他数据、Θ、盘整背驰入链或进入时口径。

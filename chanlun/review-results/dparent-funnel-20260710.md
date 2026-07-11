# D_parent 严格区间套复跑原始漏斗（2026-07-10）

## 运行范围与语义护栏

- 数据：`/tmp/codex-work-p7/analysis/data_cache/btc_1m_full.json`，**4613599** bar（2017-08-17 04:00:00 .. 2026-05-31 23:59:00）；`ThetaConfig::default()`，`l_max=6`，`min_parts_per_level=3`。
- 命令：`cargo build --release --bin strict_nest_check && ./target/release/strict_nest_check`；全量因果重放 1483.8s。
- 语义：`J_parent := D_parent = parent.interval`；闭包含只判 `child.a_interval ⊆ D_parent`；父子均显式过滤 `cand_delta=true` 且方向一致。
- `child.confirm_src - right(D_parent)` 只登记有符号分布；`ε_conf` 未进入任何控制流、排序或否决门。
- 列口径：背驰段谓词命中 = 终态分类 buy1/sell1 bit；`Cand^δ` 候选 = 塔上 `cand_delta=true` 事件；相邻边成功 = 能把已从 L0 可达的 partial chain 以 `I(A_child)⊆D_parent` 延长一级；最终证书 = `assemble_certificates(events, 0, ℓ, terminal)` 产量。

## ① 各段 × 各级计数

| 级别 ℓ | 结构事件（辅助） | 背驰段谓词命中 | Cand^δ 候选 | D_parent 相邻边成功 | 可达本级 Cand | 最终证书 N^δ_{ℓ↓0} |
|---:|---:|---:|---:|---:|---:|---:|
| 0 | 1874 | 452 | 452 | — | 452 | —（基例 terminal=452） |
| 1 | 302 | 7 | 7 | 3 | 3 | 3 |
| 2 | 113 | 13 | 13 | 0 | 0 | 0 |
| 3 | 26 | 9 | 9 | 0 | 0 | 0 |
| 4 | 4 | 2 | 2 | 0 | 0 | 0 |
| 5 | 0 | 0 | 0 | 0 | 0 | 0 |

## ② λ_conf(child) − right(D_parent) 延迟分布（纯诊断）

| 相邻级 | 同向可达候选对（结构门前） | 通过 I(A)⊆D_parent 的边 |
|---|---|---|
| L0→L1 | n=1589；负/零/正=698/0/891；min/p25/p50/p75/p90/p95/max=-4221801/-1060996/233011/1622481/2837779/3368213/4257324 | n=3；负/零/正=0/0/3；min/p25/p50/p75/p90/p95/max=60/60/87/87/87/87/183 |
| L1→L2 | n=22；负/零/正=12/0/10；min/p25/p50/p75/p90/p95/max=-3091118/-1415792/-14211/397509/1814313/1854358/2035579 | n=0 |
| L2→L3 | n=0 | n=0 |
| L3→L4 | n=0 | n=0 |
| L4→L5 | n=0 | n=0 |

`ε_conf`：**仅诊断标签，未设置数值，未作闸门**。

核对：P1 mismatch bar = **0**；L0 terminal 查无（唯一基例数）= **0**；跨级证书合计 = **3**。

## ③ 首个归零的段

首个归零发生在 **L1 → L2 的 D_parent 相邻配对**：L1 有 3 个可达 Cand 基例/partial-chain，L2 有 13 个 `Cand^δ=true` 候选，但满足 `同方向 ∧ I(A_child) ⊆ D_parent` 的可达相邻边为 **0**。`confirm_src` 延迟不参与该结论。因此从该段开始所有 `N^δ_{ℓ↓0}` 完整证书均为 0；损失不发生在 P1 谓词→Cand 映射，也不发生在 terminal 查找。

## ④ 首个归零段最接近通过的 3 个样本

排序冻结为：失败原子条件数升序 → 两个 Sub 边界缺口 bar 总和升序 → `(Sub 左界缺口, Sub 右界缺口, parent_idx)` 字典序；确认延迟不参与排序，未用价格/收益挑样本。

| 排名 | 父级 Cand（side, confirm, D_parent） | 子级可达 Cand（side, confirm, I(A)） | 通过条件 | 具体缺口；确认延迟仅诊断 |
|---:|---|---|---|---|
| 1 | `L2 Short, t=354036, D=[352003,354036]` | `L1 Short, t=345518, I(A)=[340499,341236]` | 方向、Sub右界 | **Sub 左界失败（子起点早 11504 bar）；lag_conf=-8518 bar（仅诊断）** |
| 2 | `L2 Short, t=2182560, D=[2180264,2182560]` | `L1 Short, t=2168349, I(A)=[2160411,2161133]` | 方向、Sub右界 | **Sub 左界失败（子起点早 19853 bar）；lag_conf=-14211 bar（仅诊断）** |
| 3 | `L2 Short, t=2197213, D=[2194856,2197213]` | `L1 Short, t=2168349, I(A)=[2160411,2161133]` | 方向、Sub右界 | **Sub 左界失败（子起点早 34445 bar）；lag_conf=-28864 bar（仅诊断）** |

## 结论

当前 BTC 1m / 默认 Θ / 趋势背驰-only D_parent 有效域内，证书产量 0 的首因是 **L1→L2 的 I(A_child)⊆D_parent 相邻边为 0**。上游背驰谓词并非零产量，P1→Cand 也无损；确认延迟与 ε_conf 均未作闸门。该结论不外推到其他数据、Θ 或盘整背驰入链。

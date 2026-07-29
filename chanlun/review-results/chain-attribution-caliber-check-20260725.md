# 链归因装置口径核实 + 7-24 旧读数（#251 输入材料）

- 日期：2026-07-25
- 执行：主控 session（只读核实）
- 票：map #250 子票 #251（卡点定位图）的前置口径核实

## 一、为什么做这次核实

#251 原提问是「身份桥三条件逐档放松」。核查 `admission.rs:248` 的注释后发现该提问**问错对象两次**：

1. **三条件已不存在**：T4 (#173) 起进场侧 fixed ℓ=lvl+1 comparison 与 multi 扫描桥均退役；T5a (#207) 起身份判据改为**同点递归**（两元锚在 [L0, 链顶] 逐级键域查询 `by_triple_anchor` + 层间联动 `chain_lookup`）；T5b (#208) 出场侧旧桥四件已删。
2. **生产已内建归因装置**：`chain_verdict` 三态 + `chain_first_gap`（最高非闭合级 + 缺/断极性）+ `chain_genealogy`（逐级闭合，[L0, 链顶] 升序全落账）。卡点定位不需要新实验，读出分布即可。

## 二、口径核实结论

### 2.1 测量装置本身不改判定 ✓

`endorsement-instrument-{baseline,after}-wf7-chain-dump-20260724.jsonl` 两文件 **md5 完全相同**（`8d6ffda9afea`，各 1724 行，verdict 分布逐格一致）。

对照 #214 读数报告的口径声明（`endorsement-failure-instrument-readings-20260724.md:9`）：「基线 = 本装置合入前同一未提交树（改码前二进制重放留档）」，以及 :100「baseline 三窗 stderr 无 NEST_GATE_FAIL/LEVEL 行（纯增量新行）」。

⟹ 背书失败测量装置是**纯增量观测**，链判定 bit-exact 不变。这部分可信。

### 2.2 但 dump 早于 admission.rs 大改 ✗

| 事项 | 时间 |
|---|---|
| wf7 chain dump 产出 | 2026-07-24 **03:04:37** |
| `admission.rs` knot 合版（commit `d2312352f9`，+959/−275） | 2026-07-24 **19:24:17** |

**dump 早于改动 16 小时。** 该 commit 新增 `chain_driven_level_projection`——在 `nest_cert_gate_enabled() && !config.level_projection.enabled` 时派生 `level_projection` 配置，直接影响链的级别投影，进而可能改变 verdict 与 first_gap 分布。

⟹ **7-24 读数已过期，不得直接用于裁定。** 必须在当前 HEAD 口径下重跑。

（本次是同一天内第二次撞上过期基线：先是 2026-07-01 的区间套 N^δ 实测记忆已被 T3/T4/T5a/T5b 系列改动作废，再是本条。）

## 三、7-24 旧读数（过期，仅作重跑后的对照基准）

wf7 全部 **1724** 个候选：

### 3.1 链裁决三态

| verdict | 数量 | 占比 |
|---|---:|---:|
| `no_chain`（压根没链） | 1591 | 92.3% |
| `reject`（有链但 n_δ 判假） | 100 | 5.8% |
| `pass` | **33** | **1.9%** |

### 3.2 首位缺口（verdict × kind × level）

| verdict | kind | level | 数量 |
|---|---|---:|---:|
| no_chain | missing | **0** | **883** |
| no_chain | missing | 1 | 261 |
| no_chain | missing | 2 | 206 |
| no_chain | missing | 4 | 151 |
| no_chain | missing | 3 | 73 |
| no_chain | —(None) | — | 17 |
| reject | missing | 1 | 26 |
| reject | missing | 2 | 17 |
| reject | **broken** | 0 | 17 |
| reject | missing | 3 | 17 |
| reject | missing | 4 | 15 |
| reject | **broken** | 1 | 8 |
| pass | —(None) | — | 33 |

`first_gap` 落在 level 0 的合计 900（883 + 17），**占全体 52%**。

### 3.3 逐级 status（同一候选跨多级，合计 > 1724）

| 级别 | `missing_cert` | `missing_existence` | `missing_causal` | `closed` |
|---|---:|---:|---:|---:|
| L0 | **1217** | 377 | — | 86 |
| L1 | 389 | 340 | — | — |
| L2 | 265 | 194 | — | — |
| L3 | 100 | — | 83 | — |
| L4 | 166 | — | — | — |

## 四、旧读数指向的判断（待重跑确认）

三种缺口的语义：

- `missing_cert` —— **该级根本没有证书**（样例行：`certs: 0, rungs: 0, status: "missing_cert"`）
- `missing_existence` —— 恰好存在性不成立（层间联动缺）
- `missing_causal` —— 因果前缀不满足

按旧读数，**卡点压倒性是 `missing_cert`**（L0 单级就 1217 次），而真正由桥的条件挡掉的（`missing_existence` + `missing_causal`）是次要项，被 n_δ 判假的（`broken`）仅 25 个。

⟹ 若该构成比在当前口径下稳健，则「放松桥的条件」最多能捞回 `missing_existence` 那几百个；`missing_cert` 那一千多个**放松桥一个也捞不回来，因为根本没有证书可连**。真问题会从「桥太严」变成「证书为何产得这么少」。

**这只是旧读数的指向，不是结论。** 重跑后须与本文逐格对照，差异显著则单独归因。

## 五、后续

本文件为 #251 的输入材料。#251 已按此重写为「重跑归因装置、读出分布」，并已解除对[真链判据](https://github.com/xy7365527-lang/NewChanlun/issues/254)的阻塞——读既有归因分布不需要独立真值来源（归因由引擎自报），该判据仍是终裁的输入，但不再前置。

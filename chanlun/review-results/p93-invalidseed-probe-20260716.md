# task #93 调研：215 个 D1 InvalidSeed 的携带核覆盖率与证书产量域交集（只读探针）

日期：2026-07-16。状态：调研结论，**不含语义裁决、不提出收复方案**。

## 任务界定

#84 已定格分型（全部 A3，`ZD>ZG` 核空；179 窗无其他合法三段 / 36 窗有滑窗候选）。本探针补齐
路线选择前缺失的两个测量：

1. **携带核覆盖率**：生产顺序上 `InvalidSeed` 在 `rust/src/theta_v0/classifier/level_view.rs:305`
   先于携带核检查（`:311`）抛出，塔 compose 携带核对这 215 窗**从未被读取过**（#84 亦未测）。
2. **证书产量域交集**：215 窗坐标区间与最终 as_of 快照下全部 `NestCandidateEvent`
   （`seg_a`/`interval_a`/`interval_b`/`turn_source`，p92 同一 run 切分 + provider 路径）的交集。
3. **相邻 run 拓扑**（附带）：收复单窗是否会触发左右合法 run 合并（即 provider 视图是否必然改变）。

## 复算命令

```
cargo run --release --bin p93_invalidseed_probe -- analysis/data_cache/btc_1m_full.json
```

守恒锚：`P93_CONSERVATION status=PASS bars=4613599 as_of=4613598 d1_invalid_seed=215
per_level=[165, 43, 7, 0, 0] events_total=3807`（与 #83/#84 逐级一致）。探针源：
`rust/src/bin/p93_invalidseed_probe.rs`（只读消费生产塔与 provider seam）。

## 结论

### 1. 携带核覆盖率 = 215/215，且全部合法

| 测量 | 计数 |
|---|---|
| `rmove` 为 Compose 且 `centers` 非空 | **215/215** |
| 携带核 `zd<=zg` 合法 | **215/215** |
| `centers_len=1` | 215/215 |
| 携带核缺失 | 0 |

每一个 InvalidSeed 窗口都携带恰一个合法塔核。**若走"携带核作 seed"的版本化迁移，几何前提
100% 覆盖，无残留死窗。** #84 的 36 个滑窗候选窗亦全部在内（滑窗改锚不再是必需路线）。

### 2. 证书产量域交集：214/215 非空——"域外封存"路线不可行

| 测量 | 计数 |
|---|---|
| 与同级事件坐标区间有交集 | 181/215 |
| 与跨级事件坐标区间有交集 | 214/215 |
| 任意交集（overlap_any） | **214/215** |
| 全零交集 | **1**（`L1 id=1:1182 span=553754:553954`） |

证书候选事件的消费域几乎处处覆盖这 215 窗。把它们按"与证书产量域无交集"封存、以有效域
complete 收口 R7(a) 的路线**只能覆盖 1/215，作废**。

### 3. 相邻 run 拓扑：194 窗收复会触发 run 合并——收复不是机械修复

| 拓扑 | 计数 |
|---|---|
| 左右均有合法 run（收复 ⟹ 两 run 合并） | **194** |
| 仅一侧有合法 run（收复 ⟹ run 延长） | 20 |
| 两侧均无（连片失败内部窗，`L2 id=2:1115`，左右邻窗均 InvalidSeed） | 1 |
| 与其他 InvalidSeed 连片 | 21 |

run 合并/延长会改变 provider 视图的 run 分区，进而可能改变
`provide_nest_candidate_events` 输出与下游证书。**任何收复都是语义变更**，须走 #90 条款 3
的独立立项 + 版本化迁移 + A/B 重放对账，不存在"零影响补洞"。

## 路线含义（供裁决，不是裁决）

- 域外封存：作废（结论 2）。
- 滑窗改锚：不必需（结论 1 全覆盖；且改锚同样触发 run 变更，无优势）。
- **唯一存活路线**：V2 先例式版本化迁移（参照 `EXTENDED_TO_EXACT_THREE_V2` +
  `SeedCoreProvenance` 的静默双核缝合），新 provenance 类（如 CarriedOnly，区别于
  SelfConsistent/InheritedRecut），seed 取携带核；因结论 3，迁移必须附带全量重放 A/B 对账
  （4,374 窗 bit-exact 零 diff + 215 窗显式 provenance + 下游证书差异清单）后交裁决。

## 边界声明

- 交集测量基于最终 as_of 单快照的候选事件全集；prefix 重放各时点的事件集是其子集来源，
  单快照口径对"无交集"判定是保守的（足以否证封存路线），对逐窗差异清单不足，迁移立项时须补。
- 本探针不修改生产对象；`P93_D1` 全部 215 行逐窗明细见探针 stdout。

## 后记（#94 原文回查补测：逐子段与携带核重叠，0010:29 延伸判据）

日期：2026-07-16。探针 `P93_D1` 行新增 `subs_touch=` 字段（该窗 sub_moves 中与携带核
`[zd,zg]` 时间区间重叠的子段计数），源改动仅限
`rust/src/bin/p93_invalidseed_probe.rs` 的 D1 打印，只读性质不变。

| 测量 | 计数 |
|---|---|
| `subs_touch == sub_moves`（每个子段均与携带核重叠） | **215/215** |
| 存在不重叠子段的窗 | 0 |

子段数分布：`sub_moves=3` 174 窗、`sub_moves=4` 26 窗、`sub_moves=5` 15 窗；`subs_touch`
分布逐一相同（174/26/15），零错配。守恒锚不变：`P93_CONSERVATION status=PASS bars=4613599
as_of=4613598 d1_invalid_seed=215 per_level=[165, 43, 7, 0, 0] events_total=3807`。

**含义**：0010:29 的延伸判据（延伸段须与中枢重叠）在"携带核作 seed"口径下对全部 215 窗
逐子段成立——不存在任何一个子段游离于携带核区间之外。这进一步支撑结论 1 的几何前提：
CarriedOnly 迁移不会制造"子段不触核"的新语义违例。裁决口径与边界声明不变。

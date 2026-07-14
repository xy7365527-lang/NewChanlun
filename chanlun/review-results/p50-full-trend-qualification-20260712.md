# P50 FullTrendCQualified 实装与全量重放报告（2026-07-12）

## 1. 结论

任务 #50 已按裁决 R3/R7 实装第 37 课第 20/22 行机器证书，并把完整趋势资格固定为以下不可放宽的合取：

```text
FullTrendCQualified(p) :=
  TrendContext(p)
  AND ThirdClassInsideC(p)
  AND NewExtremeInDirection(p)
  AND InternalSublevelCenters(c_p) >= 2
  AND CompletedTrendDecomposition(p)
```

BTC 1m 全量 4,613,599 bar 批式重放得到 31 个 R7 目标事件：event-time 31、third-closed 11、full-qualified 0、classification-review 20。11 个 third-closed 对象的第 20 行与第 22 行证书均成立；0 个对象取得 CompletedTrendDecomposition，因此 0 个对象取得完整合取证书。20 个未闭合对象保持 classification-review，未自动降类。

## 2. 实装口径

### 2.1 第 20 行：NewExtremeInDirection

- 先由相邻同级中枢关系确定原趋势方向；扩张关系不冒充趋势。
- 上涨要求 `c_p` 内至少一个次级别走势的外缘高点严格大于 `B_p.gg`；下跌镜像要求外缘低点严格小于 `B_p.dd`。
- 证书保存 `B_p` ID、方向、基准价、创新高/低价格、产生极值的走势 ID，以及该走势右端 `confirm_src`。
- `confirm_src` 是首次机器可证时点，不从背驰或后来的终态标签回填。

### 2.2 第 22 行：InternalSublevelCenters

- 证书保存 `c_p` 内实际次级别中枢的连续确定性 `ElementId` 链；`len >= 2` 只是对 ID 链的校验，不是计数替身。
- L0 `Segment` 没有内部中枢载荷，不能把段 ID 冒充中枢 ID。
- ID 链与 `c_p` 的 departure/terminal 逐一对齐，要求同级、严格连续并覆盖结构首尾。

### 2.3 完成分解与双时点

- CompletedTrendDecomposition 要求内部中枢链全部沿原趋势方向，并且是一个完整的最大同标签块。
- 完成态只能在 terminal 的后继次级别走势出现、证明该趋势块结束时登记；证书同时保存该后继走势 ID，`confirm_src` 取其右端。
- 事件快照只消费 `event_seg_idx` 当时可见的走势切片；越界即拒绝产证，不回退全序列。
- 增量对象在第三类成立时单调 Closed；后继走势到达时做完成性复核，不回填事件快照。复核证据同时保存后继 ID（阴性裁定也保存）。frontier pop/recompose 时，tail 重继承对象与 retained-prefix 对象统一执行 dirty 依赖门：terminal 落入 dirty 后缀则退回 Pending 并从 departure 重判；只有完成性后继落入 dirty 后缀则保留 third-closed 与第 20/22 行证据、清除旧 review/full，并从该后继重算。

## 3. 31 例四桶与全级别统计

四桶为四个明确视图，其中 classification-review 是 event-time 中尚未 third-closed 的 20 例；不是对 20 例作自动降类。

| 口径 | 总计 | L1 | L2 | L3 | L4 |
|---|---:|---:|---:|---:|---:|
| event-time | 31 | 7 | 13 | 9 | 2 |
| third-closed | 11 | 1 | 6 | 3 | 1 |
| full-qualified | 0 | 0 | 0 | 0 | 0 |
| classification-review | 20 | 6 | 7 | 6 | 1 |

R3 分量分布：

| 分量 | 总计 | L1 | L2 | L3 | L4 |
|---|---:|---:|---:|---:|---:|
| third-closed | 11 | 1 | 6 | 3 | 1 |
| NewExtremeInDirection | 11 | 1 | 6 | 3 | 1 |
| InternalSublevelCenters ID chain | 11 | 1 | 6 | 3 | 1 |
| CompletedTrendDecomposition | 0 | 0 | 0 | 0 | 0 |

全级别终态统计：

| 级别 | event-time | third-closed | full-qualified | classification-review |
|---|---:|---:|---:|---:|
| L0 | 275 | 191 | 0 | 84 |
| L1 | 7 | 1 | 0 | 6 |
| L2 | 13 | 6 | 0 | 7 |
| L3 | 9 | 3 | 0 | 6 |
| L4 | 2 | 1 | 0 | 1 |
| L5 | 0 | 0 | 0 | 0 |

旁路对象总量诊断保持：parent events 483、stable edges 483、snapshot objects 108、terminal objects 202（terminal delta 94）。

## 4. 11 条第 20/22 行终态证书

| 级别 | event confirm | third confirm | row20 confirm | row22 中枢 ID 链 |
|---|---:|---:|---:|---|
| L1 | 3306324 | 3306426 | 3306324 | `L1#6703 -> L1#6704` |
| L2 | 354036 | 355240 | 354036 | `L2#151 -> L2#152` |
| L2 | 510836 | 512973 | 510836 | `L2#229 -> L2#230` |
| L2 | 1761310 | 1762852 | 1760063 | `L2#834 -> L2#835 -> L2#836` |
| L2 | 1967685 | 1969685 | 1967685 | `L2#915 -> L2#916` |
| L2 | 2182560 | 2184459 | 2182560 | `L2#993 -> L2#994` |
| L2 | 3427468 | 3431710 | 3427468 | `L2#1563 -> L2#1564 -> L2#1565` |
| L3 | 693716 | 706773 | 693716 | `L3#71 -> L3#72` |
| L3 | 960644 | 988650 | 960644 | `L3#99 -> L3#100 -> L3#101` |
| L3 | 1870550 | 1884023 | 1870550 | `L3#181 -> L3#182` |
| L4 | 4071883 | 4175633 | 4139257 | `L4#70 -> L4#71 -> L4#72 -> L4#73 -> L4#74 -> L4#75` |

以上 11 条的 decomposition confirm 与 full confirm 均为 `None`，与 full-qualified=0 一致。

## 5. 消费者迁移（GATED-3）

- `nest` 提供并区分 `d_parent_interval_snapshot` / `d_parent_interval_terminal`、单个与批量 snapshot/terminal 装配 API。
- 旧 `d_parent_interval_full`、`assemble_certificate`、`assemble_certificates` 仅留兼容入口并标记 deprecated；不再允许新消费者静默选默认语义。
- `strict_nest_check` 的正式装配沿稳定边读取 terminal 对象证书，同时保留原事件对象用于 event-time/P1 诊断。
- runner sidecar 显式选择 terminal；不接入订单、候选、风控或账本。
- 裁决文档 GATED-3 已核销。

## 6. 禁区核对

- 未改 `judge_third_cert` 条件分支。
- 未放宽任何闭合条件。
- 未回填历史事件；snapshot 与 terminal 分离。
- 686 fallback 方向禁令、#37 双时点、#43 episode/完整 `c_p` 能力边界均保留。
- 20 个 classification-review 对象未自动降类。

## 7. 验证

批式全量命令：

```bash
CP_SMOKE_MODE=batch CP_SMOKE_MAX_BARS=4613599 CP_SMOKE_VERBOSE=0 \
  CP_SMOKE_P50_CERTS=1 cargo run --release --bin cp_capability_smoke
```

strict-nest 消费迁移回归由该二进制的 9 个单测覆盖。另做逐 bar 前缀诊断至 1,400,000 bar，P1 mismatch bars 始终为 0；该前缀诊断不冒充全量结果，任务要求的 4,613,599 bar 正式统计来自上面的批式全量命令。

最终验收：

| 命令 | 结果 |
|---|---|
| `cargo check --all-targets` | PASS |
| `cargo test` | PASS：1560 passed / 127 ignored / 0 failed（基线 1554 + 本任务 6 个结构回归） |
| `strict_nest_check` 单测 | PASS：9 passed / 0 failed |
| `git diff --check` | PASS |

6 个结构回归覆盖：下跌创新低正例、上涨创新高正例、第 20 行无新极值反例、第 22 行仅有段 ID/计数但没有中枢 ID 链反例、扩张内部中枢链反例、terminal 后继到达/变异（dirty 失效）后的增量完成性复核。

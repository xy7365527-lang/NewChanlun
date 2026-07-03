# Codex decide 裁决：Q7-#1 跨裁决张力（盘整块 ownership 单元 endpoint fallback 方向消费合法性）

**工位**：codex-challenger（代码层异质审查工位） · **日期**：2026-07-03 · **触发**：Task #146
描述——ac4-impl-20260703.md §8.4 遗留上浮项，编排者全权授权 codex decide 作为裁决渠道。
**裁决渠道**：`.venv/bin/python -m newchan.codex decide`（codex-cli 0.142.5，ChatGPT 订阅认证）。
**原始 transcript**：`.chanlun/review-results/codex-decide-20260703-222813-c563.md`（CLI 自动持久化，
含完整 prompt + response，本文档 §2 为其摘录）。

## 1. 结论

**裁定 C（分级处置）**：fallback 单元（`center_own_dir_at(blocks,i)==None`——落在 Consolidation
块 ownership 区间或 `i==0`）保留作为序列/区间/面积累计的**成员**，但**不得**作为上级一/三类
判据（`extract_first_third_for_level` → `locate_departure_move_a`/`judge_first_cached`/
`departure_move_c_start`）中的 A 段候选、C 段/破中枢段、离开段**方向锚**。

**要求实装改动**：是。需新增区分 provenance 的判据字段（codex 建议命名 `anchor_direction:
Option<Direction>`，`Some(center_own_dir_at 结果)`/`None`=fallback），并在三处消费点把
`s.direction == trend_dir` 换成 `s.anchor_direction == Some(trend_dir)`。

**与 #121 裁定A 的关系**：**收窄，非推翻**。#121"级别-N 直接判定架构"（units 承担线段角色、
复用 L0 判据链）继续成立；被废止的仅是"`UnitRange.direction` 不论来源都可当真实趋势方向消费"
这一隐含前提——该前提在 #121 裁决时尚不成立冲突（`center_own_dir_at`/`blocks` 参数是 Q7 本轮
晚于 #121 约 12 小时新增，#121 裁决时 direction 只有 `fold_direction` 一种来源，无 provenance
区分的必要性）。

## 2. codex 裁决摘录（完整推理见 transcript）

> **决策**：选 **C：分级处置**
>
> `UnitRange.direction` 继续保留为结构/几何方向，但不得再单独代表"可用于趋势判定的方向"。
> 新增或派生一个不可变的方向资格字段：`anchor_direction: Option<Direction>`。
>
> 判据：`anchor_direction = center_own_dir_at(blocks, i)`；`Some(dir)` ⟹ 该单元可作为上级
> 一/三类判据的方向锚；`None` ⟹ 该单元只能作为序列成员/区间成员/面积累计成员，不能作为 A 段
> 候选、C 段/破中枢段、离开段方向锚。
>
> 具体替换：`s.direction == trend_dir` → `s.anchor_direction == Some(trend_dir)`，适用于
> `locate_departure_move_a`（A 段候选筛选）、`judge_first_cached`（C 段/破中枢段触发一类）、
> `departure_move_c_start` 及三类相关离开/突破段判断。
>
> **推理链**：(1) #121 的"高级别 units+centers+trend 原生判定"核心是对的；(2) 但 #121 里
> "direction 已存在故过期"的论证在 Q7 后需收窄——direction 现有两种来源（趋势 ownership 方向
> vs endpoint fallback），概念不同；(3) 缠论定义里一类买卖点的定义域是"上涨/下跌趋势中"，
> 盘整只有 1 中枢无趋势方向，盘整背驰是独立范畴非趋势背驰弱化版；(4) `fold_direction` 只是
> endpoint 比较，不能把盘整 ownership 单元提升成趋势方向锚；(5) 但 fallback 单元仍是完成的
> 次级别走势单元，完全移出序列会破坏时间连续性/区间划分/面积累计/中枢关系，故不选纯删除（B）；
> (6) 最小正确建模 = 结构方向与判定资格分离。
>
> **信号计数影响**：预期减少，但不保证单调——过滤掉某 fallback A 段候选后，episode 定位可能
> 改选更早的合法 A 段候选，个别情况可能反而新增信号；总体按 golden 变更处理。
>
> **边界条件（裁决翻转条件）**：领域定义正式改为"盘整 ownership 单元 endpoint 方向可作为上级
> 趋势方向锚" / `fold_direction` 被严格证明等价于某级别趋势 ownership 方向 / 后续算法不再用
> `Segment.direction` 判一三类改用 `MoveBlock::Trend(dir)` 边界直接生成候选 / 实测发现保留
> fallback 作为成员仍间接制造方向泄漏且无法局部守卫（则升级到 B）。

## 3. 证据包摘要（完整版见 `/tmp/codex-q7-fallback-ctx.md`，已随 prompt 转入 transcript）

- **裁决一**（#121 codex-t1，`codex-decide-20260703-025537-8d3c.md`）：选 A"级别-N 直接判定"——
  units 整体承担线段角色，复用 L0 判据链；裁决时 `project_to_units` 尚无 `blocks`/
  `center_own_dir_at`，direction 只有 endpoint 一种来源。
- **裁决二**（Q7 审计，`codex-review-20260703-221031-3801.md` #1，致命/reject）：fallback 方向
  无 provenance 流入 `extract_first_third_for_level` 被当真实方向消费——判定跨裁决张力上浮。
- **代码事实**：`recursive_tower.rs:477-493` `project_to_units`（`center_own_dir_at(blocks,idx)
  .unwrap_or_else(|| m.fold_direction(prev))`）→ `decompose.rs:449-457` `center_own_dir_at`
  （Consolidation 块/`i==0` ⟹ `None`）→ `mod.rs:129-141` `unit_to_segment`（无条件写入
  `Segment.direction`，不分辨来源）→ `signal.rs` `locate_departure_move_a`/`judge_first_cached`
  按 `Segment.direction == trend_dir` 匹配——provenance 判据（`center_own_dir_at` 本身）已存在
  但未接入消费点。
- **缠论定义**（`缠论知识库.md` §7.1/§6.7/§9.1/§10）：盘整=恰1中枢+无方向；趋势=2+同向中枢+
  方向；"没有趋势就没有背驰，盘整里只谈盘整背驰"（独立范畴非弱化版）；第一类买卖点定义域显式
  为"某级别下跌/上涨**趋势中**"。
- **残余泄漏面**（裁决时无现成计数标注待测；实装后 BTC 全历史实测补录，`level_signal_census_btc`
  @ 裁定C 实装 commit）：Q7 前泄漏面=100%（全部 endpoint 占位，`ac4-impl-20260703.md` §8.3-1
  确认）；Q7 后残余 = `i==0`（每级恰1个，结构必然）+ Consolidation ownership 单元。**实际排除
  锚面（一类 buy1+sell1 差分，旧→新）**：L0 575→575（豁免路径逐位不变，锚门 L0 恒等的实证）；
  L1 349→18（排除 331，94.8%）；L2 102→14（排除 88，86.3%）；L3 31→7（排除 24，77.4%）；
  L4 0→0。一类总计 1057→614。块级参照（census 同跑）：L0 分解 782 块中 trend=393（盘整块
  389，49.7%）；L1 320 块 trend=165；L2 96 块 trend=50；L3 20 块 trend=12。逐单元 fallback
  占比未单独插桩（可由 `center_own_dir_at` 逐点统计派生，签名信息量低于上述信号差分——排除面
  已由差分直接量化）。

## 4. 边界条件（本裁决何时翻转）

同 §2 codex 给出的四条：(1) 领域定义正式授权 endpoint fallback 可作趋势锚；(2)
`fold_direction` 被证明等价于某级别趋势 ownership 方向；(3) 判据链改为直接消费
`MoveBlock::Trend(dir)` 边界而非 `Segment.direction`；(4) 实测显示保留 fallback 作序列成员
仍间接制造方向泄漏且无法局部守卫——则升级为 B（严格排除，不作序列成员）。

## 5. 下游推论（若裁决被 Task #146/rerun5 采纳并实装）

- 需改动 `unit_to_segment`/`extract_first_third_for_level`（`mod.rs`）签名以传入 `blocks`
  并派生 `anchor_direction`；`locate_departure_move_a`/`judge_first_cached`/
  `departure_move_c_start`（`divergence.rs`/`signal.rs`）的方向匹配条件需从 `s.direction`
  改为 `s.anchor_direction`。
- 级别≥1 一/三类信号计数（当前 L≥1 部分：#145 ac4-impl §2 表 L1 349/L2 102/L3 31）将再变——
  方向预期减少但非保证单调，须重新冻结 GOLDEN，纳入 #146 rerun5 同批重跑清单。
- Q4 盘整背驰承接路由（`pan_div_gate_pass`）与本裁决对象不同（PanDivCert 消费 Consolidation
  块本身而非其 ownership 单元的 fallback 方向），预期不受影响，但需在实装时核对 `PanDivCert`
  是否间接依赖 `UnitRange.direction`（当前证据未发现依赖，需实装时复核）。

## 6. 影响声明

- 本文档为纯裁决记录，未修改任何生产代码。
- 涉及模块（待实装）：`rust/src/theta_v0/classifier/mod.rs`（`unit_to_segment`/
  `extract_first_third_for_level`）、`rust/src/theta_v0/classifier/divergence.rs`
  （`locate_departure_move_a`/`departure_move_c_start`）、`rust/src/theta_v0/classifier/
  signal.rs`（`judge_first_cached`/`extract_signals_with_hist` 调用链）、
  `rust/src/theta_v0/classifier/decompose.rs`（`center_own_dir_at` 消费方需扩展调用点）。
- 谱系引用：本裁决闭合 `ac4-impl-20260703.md` §8.4 遗留上浮项（Q7-#1）；#121（codex-t1）
  与本次 Q7 审计（`codex-review-20260703-221031-3801.md`）均为已结算记录，本裁决在两者之间
  裁定收窄关系，不改写任一方原文。

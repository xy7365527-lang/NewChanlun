# owner 归属判据裁定：归属语义不动 + 二类载体修填 + 判同机制换两元锚（路线 B）

- 日期：2026-07-24 ｜ 票据：#211（wayfinder map #126「塔侧跨级链增产」子票）｜ 裁定：编排者（grilling 在场）
- 依据链：#210 事实账（`p1-resupercede-fact-ledger-20260724.md`）→ #214 测量读数（`endorsement-failure-instrument-readings-20260724.md`）→ #217 orphan 归属核查（`orphan-ownership-survey-20260724.md`）
- 前置裁定：P1 重议（`p1-resupercede-trend-all-three-types-20260721.md`）、#206 身份教义（身份=同点递归、锚=（极值价, 合并组锚）、禁序号判同）

## 背景

P1 放开 Trend 域二三类后实测产量仅 +8/窗。#214 实测：窗内同向二三类点 `center.start_index == b_center_start` 相等率 14.0%（23/164）；wf7 Trend 失败桶 = 异向 59% / owner 不等 36% / 窗口外 4.5%；Trend 事件仅占全部事件 4.3%。

烤制中编排者指出并经查证坐实的双错配：

1. **级别错配**：二类点判定中枢 c1 是**次级别**结构（生产实参 = parent Compose `centers.first()`），事件 B 是**同级别**结构——拿次级别 start_index 比同级别 B，是范畴错误（编排者教义表述：买卖点首先也是同级别的——在同级别有定义条件，在递归区间套里有次级别确认条件；错配根因 = 归属载体语义不统一：一/三类载同级别中枢、二类载次级别 c1，而判定式按「载体必为同级别 B 同型对象」写）。
2. **锚错配**：`start_index` 序号判同撞 #206 身份教义（K 序号在包含合并下漂移，不作身份依据；判据 = 同点递归，锚 =（极值价, 合并组锚））。反面先例：XZD 双序号判同已被 #44 证伪「同中枢序号 ≠ 同归属链」。

#217 核查：结构派生归属在点构造层已全部实装（一类 center=被破 last_center、三类 center=所离开回抽中枢，均教义对齐；二类 center=次级别 c1 是级别错配构造根源）；projection.rs 两元锚索引（#206 裁定锚原文形态）已接 admission（5 处调用）、未接 nest 背书；点侧两元锚无需新字段（source_index→(极值价,组锚) 查法已在 projection.rs:143-145），事件侧 NestCandidateEventExt 已带锚。

## 裁定（路线 B）

1. **owner 归属语义不动**：owner = 点属于本事件所确认的走势（同走势归属）。不换同点谓词——同点（点↔事件同一拐点）是值桥层语义，两层不重叠接管。
2. **二类载体修填**：`make_second_point` 的 center 从 c1（次级别中枢）改载**该走势的同级别结构**（parent 走势的最后中枢，与事件 B 同型同级）。一/三类载体不动（已教义对齐）。修填后 `center == B` 判定式语义自动成立（同级别同型判同）。
3. **判同机制换锚**：owner 判同从 `start_index` 序号判同换成**身份锚（极值价, 合并组锚）判同**——接 projection.rs 两元锚索引（`cross_level_query` 模式，复用已接 admission 的既有实装，不重写）。禁序号判同。
4. **GOLDEN 翻转**：判定集合会变 ⟹ digest/GOLDEN 翻转按 #110 线先例诚实重算并登记，不静默。

## 影响清单

- 执行票：spec → ticket → /implement（TDD 主接缝 + cargo check + 全量测试 + wf7 重放 + 红线对照 + 两轴 code-review，同 #214 管线）。
- 验收读数：wf7 重放的新相等率（一/二/三类分层）与产量 delta（对照 #214 基线：二三类 23/164 = 14.0%）。
- map #126 Destination 验收口径不变：typed_found 显著上升 ✓ 或如实判定。
- 090：核化触及处 doc 如实改写；未确证项照实标。

## 不采纳记录

- 路线 A（owner 换同点谓词接 projection 锚）：归属语义丢失且与值桥层重叠——否。
- 路线 C（另写结构派生归属关系）：构造层原料已算出同类条件，重写是重复——否。
- 维持 owner=B 不动：级别错配+锚错配双在案，且与 #206 身份教义冲突——否。

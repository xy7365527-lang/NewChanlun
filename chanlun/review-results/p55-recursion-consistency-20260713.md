# P55 递归实装一致性核查 + 两例强张力复核

- 日期：2026-07-13
- 角色：只读核查代理（OpenAI Codex）
- 范围：生产定义与生产 Rust 只读；唯一仓库写入为本报告
- 结论纪律：仅列证据与倾向，不替 P0 下最终裁决
- 证据标签：`【已验证】` 表示可由当前文件、当前代码或本轮定向重放直接复核；`【推断】` 表示由这些事实导出的语义判断

## 0. 核查口径与摘要

【已验证】用户给出的定义路径在仓库中实际为 `.chanlun/definitions/level_recursion.md`。本报告先服从已经落地的 C 裁决：要求“完成的走势类型”的生产消费者，法定对象是消费层 as-of 组装出的 `CompletedMove[k]`；递归塔产物是 `WindowUnit[k]`；“窗口封闭”不得替代“走势类型完成”。塔本身不改，二者双轨分层：`chanlun/escalate/c-ruling-decision-20260713.md:11-14,23-24`。

核查摘要：

1. 【推断】`K线 → 分型 → 笔 → parser Segment → Move[0]` 的对象粒度总体一致。parser 只把 confirmed Segment 送入分类器、把未完成尾部单列；映射保留方向、起止坐标和价格区间。未发现把笔、尾段或任意三段误命名为 `Move[0]` 的证据。
2. 【已验证】当前 `theta_v0` 递归塔从 L1 起向上消费的不是 `CompletedMove[k-1]`，而是 exact-three 几何窗口 `WindowUnit[k-1]`。上级种子只查三窗口价格重叠，不查盘整被后继终结、趋势末段背驰或 `settled/completion` 状态；每个上级对象固定只含一个中枢。生产树中也不存在已接线的 `move_view.rs`/`CompletedMove` 组装器。
3. 【推断】因此，当前 `theta_v0` 对“不可变窗口塔”的实装与 C 裁决一致，但对定义要求的“完成走势作为更高级递归组件”尚未实装一致。`MoveKind` 的盘整/趋势标签不能弥补这一点：它没有完成状态，且实际向上传递的是逐中枢 exact-three `upper_moves`，不是该标签所描述的完整走势实例。
4. 【已验证】仓库另有 `orchestrator.rs → LevelEngine → level.rs` 的 settled-Move 递归核；它在构造高级中枢时过滤 `m.settled`，与定义形状一致。但它与 `theta_v0::classify_with_tower` 是不同对象/入口，不能把前者的合规性自动转借给 #54 的 WindowUnit 宇宙。
5. 【推断】第三类点构件粒度仅在当前 theta_v0 的 L0 路径上局部一致：相邻 parser Segment 正是完成的 `Move[0]`。它没有机器化 `firstRetrace`，也没有 theta_v0 的高层 `CompletedMove` 第三类点生产入口。#54 审计分支枚举的是相邻 WindowUnit pair，按 C 裁决不等于完成走势 pair。
6. 【推断】两例在各自 `judge_at` 时点的首回抽与后续成功回抽，按现有 C2 原型的四种确定性方向适配均为 `Pending`；故本报告的倾向均为：**不能把后续几何 SUCCESS 当作由完成 `Move[k-1]` 构成的合格后续回抽，原“首对失败、后对成功”的强张力尚未进入严格 C2 的 firstRetrace 比较域**。

## 1. 定义条目 ↔ 实装位置映射

| 定义/裁决条目 | 实装位置（文件:行号） | 一致性 | 证据与倾向 |
|---|---|---|---|
| `Move[0] = Segment`；K线经包含、分型、笔、特征序列形成线段 | `.chanlun/definitions/level_recursion.md:19-45`；`rust/src/theta_v0/parser/mod.rs:258-283`；`rust/src/theta_v0/parser/segment.rs:305-393` | **一致** | 【已验证】parser 按链构造 fractals、strokes、confirmed segments，并把活动尾部另存为 `tail`；分类器不消费 tail。 |
| parser Segment 映射为 Move[0] 时应保持对象身份与结构语义 | `rust/src/theta_v0/classifier/mod.rs:96-113,186-205`；`rust/src/theta_v0/classifier/recursive_tower.rs:109-128`；`rust/src/theta_v0/types.rs:97-105` | **一致；有证据字段收窄** | 【已验证】方向、source 起止坐标、端点价格规约区间及稳定 ID 均被保留。`segments_confirmed_len` 是性能前缀证书而非完成语义：`parser/mod.rs:95-110`。【推断】映射未保留线段终结规则的 provenance，但定义中的 Move[0] 本体并未要求该字段，故目前不构成语义落差。 |
| `Center[1]` 由三个连续 `Move[0]=Segment` 重叠构成 | `.chanlun/definitions/level_recursion.md:24-26,70-84`；`rust/src/theta_v0/classifier/center.rs:136-172`；`rust/src/theta_v0/classifier/mod.rs:115-125,227-238` | **一致** | 【已验证】L0 使用 confirmed Segment，判方向交替与三段共同重叠；成功窗口对应三个连续 Move[0]。 |
| `Center[k]`（k≥2）的组件必须是三个已完成的 `Move[k-1]`，而非仅几何重叠 | `.chanlun/definitions/level_recursion.md:70-80,143-180,231-242`；C 裁决 `c-ruling-decision-20260713.md:11-14` | **偏差（theta_v0）** | 【已验证】`LeveledMove` 无 completion/settled 字段：`recursive_tower.rs:74-107`；高层 `center_from_window` 只做三窗口几何重叠：`center.rs:174-201`；扫描成功 `i+=3`、失败 `i+=1`：`recursive_tower.rs:180-209`。【推断】这只能证明 WindowUnit 种子成立，不能证明组件是 CompletedMove。 |
| `Move[k]` 是含 1 个中枢的盘整或含 ≥2 同向中枢的趋势实例 | `.chanlun/definitions/level_recursion.md:24-27,76-80`；`rust/src/theta_v0/classifier/level.rs:55-90`；`rust/src/theta_v0/classifier/mod.rs:227-266` | **分类形状一致；实例递归偏差** | 【已验证】`classify_move` 按 1 中枢/≥2 同向中枢给出 label；但 `outcome_to_kind` 只产一个可选 `MoveKind`，实际向上传递的 `upper_moves` 则由每个中枢各自 exact-three compose，一一对应且各含单中枢：`recursive_tower.rs:130-156,211-249`。【推断】label 不是完成走势实例，也未成为下一层组件。 |
| 盘整须被后续走势终结；趋势须有末段背驰/合法终结证据；as-of 不得 hindsight | `.chanlun/definitions/level_recursion.md:76-80,231-242`；`chanlun/review-results/assembler-spec-20260712.md:123-164`；C 裁决 `:11-14` | **未接入生产（theta_v0）** | 【已验证】规范原型定义了 Completed/Pending 与盘整后继、趋势 A/C+MACD 门，但其报告明确“生产接入：无”：`assembler-spec-20260712.md:7-18`；当前 `rust/src/theta_v0/classifier/` 下无 `move_view.rs`，无 `CompletedMove/CompletionStatus/assemble_move_view` 符号。【推断】C2 是既定目标语义，不是当前生产事实。 |
| 高级中枢只消费 settled 下级 Move | `rust/src/moves.rs:30-48,174-215`；`rust/src/level.rs:1-7,47-56,101-162,290-335`；`rust/src/orchestrator.rs:474-529` | **一致（另一条递归核）** | 【已验证】`LevelEngine::process` 在 `orchestrator.rs:509-524` 先过滤 `m.settled`，再构造 Center/Move；最后一个新 Move 被显式置 `settled=false`。【推断】这是定义形状正确的独立路径，但没有证据表明其 CompletedMove 身份已桥接到 theta_v0/#54 的 ElementId 与消费者。 |
| 第三类点：第一次离开后的回试；离开、回试均是完成的次级别走势类型 | `.chanlun/definitions/maimai.md:132-147` | **theta_v0 仅 L0 局部一致；高层偏差/缺失** | 【已验证】当前 theta_v0 只在 L0 调 `extract_signals`：`classifier/mod.rs:240-253`；它逐相邻 Segment `(leave,retest)` 判几何：`classifier/signal.rs:303-351,582-627`。`EndpointSituation` 没有 `first_retrace` 字段，`is_third` 只合取 `left_center && retrace_not_reenter`：`classifier/bsp.rs:27-65`。【推断】Segment 粒度在 L0 合法，但“第一次”未被结构证书约束，高层 CompletedMove pair 尚无生产入口。 |
| #54 高层 pair 审计是否枚举完成 Move pair | `/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:507-671`；同分支 `signal.rs:371-447` | **偏差（诊断分支）** | 【已验证】审计对 `units/unit_moves` 的每个相邻窗口作 pair，把 UnitRange 临时转成 Segment 后跑几何证书；没有 completion 门，也不是 first pair 后即停止（成功前继续枚举）。【推断】这些是 WindowUnit pair 的上界诊断，不能按 C 裁决直接解释为 CompletedMove pair。 |
| 另一生产递归核的高层第三类点是否强制回抽 Move settled | `rust/src/orchestrator.rs:420-471`；`rust/src/buysellpoint.rs:337-369,655-669,799-846` | **构件类型一致；完成门偏差** | 【已验证】它把下级 `Move` 投影为 SegView，`build_type3_bsp` 能用 `require_settled` 合取回抽 settled，并选突破后的第一个反向 Move；但 `level_buysellpoints` 实际调用明确传 `false`（`orchestrator.rs:470-471`）。【推断】“严格走势类型”这一类型名不能替代对象的 `settled=false` 状态，故该入口也不能为“完成回抽”提供严格证书。 |

## 2. (a) Move[0] 构造链专项核查

### 2.1 已验证事实

- 【已验证】`ParseLayer` 明确把 confirmed 结构与 incomplete tail 分开：`rust/src/theta_v0/parser/mod.rs:71-106`。
- 【已验证】全量链依次调用分型、笔、`divide_segments_with_tail`，只把返回的 confirmed `segments` 放入 `ParseLayer.segments`：`rust/src/theta_v0/parser/mod.rs:258-283`；段划分函数的 confirmed/pending 契约见 `parser/segment.rs:305-393`。
- 【已验证】`segment_to_unit` 与 `LeveledMove::from_unit` 保留方向、起止 source index、价格区间，并生成 L0 `RMove::Segment`：`classifier/mod.rs:96-113,186-205`；`classifier/recursive_tower.rs:109-128`。

### 2.2 倾向

【推断】parser Segment → Move[0] 没有发现实质语义落差。可见的收窄是“线段为何确认”的 provenance 未进入 `LeveledMove`，但 Move[0] 的递归消费者只需稳定的已完成线段身份、方向、区间和坐标；这些均在。若后续消费者要审计线段终结法，则需另加 provenance，这属于可审计性缺口，不是本轮已证的对象粒度错误。

## 3. (b) Center[k] 与 Move[k] 专项核查

### 3.1 exact-three 实际语义

- 【已验证】代码自身将 compose 定义为连续三段窗口，每个 `RMove::Compose` 的 `subs` 恰为三段、`centers` 恰为一个：`rust/src/theta_v0/classifier/recursive_tower.rs:26-33,130-156`。
- 【已验证】窗口扫描成功消费 3、失败滑 1；`compose_level` 对每个成功中枢各生成一个上级对象：同文件 `:180-249`。
- 【已验证】L0 seed 要求方向交替 + 三段重叠；高层 seed 只要求三段区间重叠：`rust/src/theta_v0/classifier/center.rs:136-201`。
- 【已验证】上级方向字段只是外缘占位，不是已结算走势方向：`recursive_tower.rs:159-177`。
- 【已验证】`classify_move` 的盘整/趋势 label 没有完成状态；主循环随后仍把 exact-three `upper_moves` 投影并传给下一层：`classifier/level.rs:55-90`；`classifier/mod.rs:227-266`。

### 3.2 倾向

【推断】若对象名按 C 裁决改读为 `WindowUnit`，当前塔的结构实现自洽；若把它读成定义中的完成 `Move[k]`，则存在两重偏差：

1. 组件资格偏差：Center[k≥2] 的三个组件没有 `CompletedMove[k-1]` 证书。
2. 实例边界偏差：一个 exact-three/单中枢窗口被直接当作下一层走势单元，而定义的 Move[k] 需要按中枢序列形成盘整/趋势并取得终结证据。

【推断】另一条 settled-Move 递归核证明仓库已有可复用的定义形状，但 C 裁决要求的是消费层 as-of 组装且不动塔；在生产桥接完成前，不能宣称 theta_v0 已按该结算口径实装。

## 4. (c) 第三类点 pair 构件粒度专项核查

### 4.1 当前主树

【已验证】当前 theta_v0 的第三类点生产只在 L0，pair 是相邻 confirmed Segment。按 `Move[0]=Segment`，其构件粒度正确；但判据只检查最近中枢、离开方向、回试反向与不重入几何，不携带 `firstRetrace`/completion 字段：`rust/src/theta_v0/classifier/signal.rs:303-351,582-627`；`classifier/bsp.rs:27-65`。

【推断】因此它能证明“某个相邻 Move[0] 对满足第三类几何”，不能完整证明“该对是第一次完成走势回试”。对 k≥1，当前 theta_v0 根本没有 CompletedMove pair 的同类生产枚举。

### 4.2 #54 审计分支与另一递归核

【已验证】#54 审计分支在每个对象内从 `cp_start` 后逐个枚举相邻 `WindowUnit`，首个成功前会继续尝试后续 pair：`/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:563-671`。这正是两例 “首 pair RETEST_REENTERS、后 pair SUCCESS” 的来源。

【推断】此枚举适合作为几何召回上界与 firstness 张力探针，不适合作为 C2 第三类点事实表；应先把 lower ledger 组装为相邻、不同、已完成的 `CompletedMove[k-1]`，再在其中只认第一次合法回试。

【已验证】独立 `LevelEngine` 路径的 pair 类型确实是下级 Move，并找突破后的第一个反向 Move；然而生产调用把 `require_settled=false` 传入：`rust/src/orchestrator.rs:420-471`；`rust/src/buysellpoint.rs:799-846`。

【推断】这一事实说明“pair 的 Rust 类型是 Move”与“pair 是完成 Move”必须分开核查；当前该路径只能给前者正证，不能给后者正证。

## 5. 两例强张力定向复核

### 5.1 方法、边界与基线

- 【已验证】#54 两例及 pair 时点来自 #54 审计工作树：例1见 `/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:355,430,512`；例2见同文件 `:422,430`。
- 【已验证】#54 分支是可变长窗口/ownership 基线，而当前主树是 exact-three 基线；两者对象宇宙不能混算：`chanlun/review-results/c-ruling-evidence-replay-20260712.md:20-31`。
- 【已验证】本轮在 #54 对象身份上做定向重放，并用现有 C2 as-of 原型分别在每个 pair 的 `judge_at` 截断；方向未有最终裁决，因此并列运行 `ownership_fallback / first_leaf / first_last_envelope / sequence_envelope` 四个确定性适配。原始输出：`/private/tmp/p55-probe.log:1-38`。
- 【已验证】四种适配均只能用于敏感性核查；趋势 A/C hook 尚未生产化，原型自己也声明 hook 缺失时趋势只能 Pending：`chanlun/review-results/assembler-spec-20260712.md:156-164`。
- 【推断】C 裁决要求 as-of，故 `history_end` 后来变成 Completed 不能倒灌为 `judge_at` 时已经完成；下表以 `judge_at` 为判定时点，history-end 只用于展示敏感性。

### 5.2 例1：level=1，B=L2#777，cp_departure=L1#3456

原始 pair：

| pair | leave | retest | #54 原子 | judge_at |
|---|---|---|---|---:|
| 首对 | `L1#3456 [1625292..1626098]` | `L1#3457 [1626098..1626497]` | `RETEST_REENTERS` | 1626497 |
| 后续成功对 | `L1#3458 [1626547..1626953]` | `L1#3459 [1626961..1627583]` | `SUCCESS` | 1627583 |

【已验证】定向重放原始锚为 `/private/tmp/p55-probe.log:1-19`：

- 首对在 `judge_at=1626497`：四适配中的 retest 均为 `Consolidation/Undetermined + Pending`；leave 则为 Unassigned 或与 retest 折叠到同一 Pending 组。
- 后续成功对在 `judge_at=1627583`：四适配中的 retest 全为 Pending；leave 也全为 Pending，且 `first_leaf` 下 leave/retest 折叠为同一 Pending 组。
- 到全历史末端，后续 retest 仅 `first_last_envelope` 变为 Completed Consolidation；另三适配仍 Pending。首 retest 在四适配下始终不是 Completed。

**例1 倾向：**【推断】**否——在两个各自判定时点，回抽段均不能认作已完成的 `Move[1]`；后续成功对连“两个不同的 CompletedMove”资格也未取得。** 因此 #54 的 `RETEST_REENTERS → later SUCCESS` 是 WindowUnit 几何序列上的强张力，但尚不是严格 C2 firstRetrace 规则内部的反例。该倾向不裁定未来唯一方向/A-C hook 接线后的最终身份。

### 5.3 例2：level=3，B=L4#47，cp_departure=L3#273

原始 pair：

| pair | leave | retest | #54 原子 | judge_at |
|---|---|---|---|---:|
| 首对 | `L3#273 [2884261..2896661]` | `L3#274 [2896743..2906047]` | `RETEST_REENTERS` | 2906047 |
| 后续成功对 | `L3#276 [2927357..2934387]` | `L3#277 [2935555..2943378]` | `SUCCESS` | 2943378 |

【已验证】定向重放原始锚为 `/private/tmp/p55-probe.log:20-38`：

- 首对在 `judge_at=2906047`：四适配中的 retest 全为 `Consolidation + Pending`。leave 在 ownership/first_leaf/sequence 下为 Completed Consolidation，但在 first_last 下仍为 Pending；无一适配给出 Completed retest。
- 后续成功对在 `judge_at=2943378`：四适配中的 retest 仍全部 Pending。leave 仅 ownership/sequence 下为 Completed，first_leaf 折叠为同一 Pending 组，first_last 仍 Pending。
- 到全历史末端，首 retest 仅 first_leaf 变 Completed；后续 retest 仅 ownership/sequence 变 Completed，另两适配仍 Pending。该分歧进一步说明不能用 history-end hindsight 反写判定时点。

**例2 倾向：**【推断】**否——在两个各自判定时点，回抽段均不能认作已完成的 `Move[3]`。** 后续几何 SUCCESS 没有进入严格 C2 的合格 pair 域，所以它不能推翻“第一次完成回抽”口径；它只证明 WindowUnit 枚举若允许跳过首个几何回抽，会找到后续几何成功。

## 6. P0 可直接采用的证据边界

### 已验证

1. 当前 theta_v0 塔是 exact-three `WindowUnit` 递归；上级中心只查几何，塔对象无 completion 状态。
2. 当前生产 theta_v0 没有 C2 `CompletedMove` 组装器接线。
3. 当前 theta_v0 第三类点生产只覆盖 L0 相邻 Segment；#54 高层审计覆盖的是相邻 WindowUnit。
4. 独立 LevelEngine 构中心时消费 settled Move，但其高层第三类点入口显式关闭回抽 settled 门。
5. 两例的首/后续回抽在各自 judge_at、四种现有确定性方向适配下均为 Pending。

### 推断/倾向（非最终裁决）

1. Move[0] parser 映射可判为一致；缺的是终结 provenance，而不是已证的对象语义错配。
2. theta_v0 从 Center[2] 起不符合“由完成次级别走势构造”的定义结算口径；它符合的是 C 裁决保留的 WindowUnit 构造层。
3. #54 两例不能作为“第一次完成回抽失败、第二次完成回抽成功”的严格反例；在现有 C2 证据下，两例更接近“尚未进入 CompletedMove firstRetrace 判定域”。
4. 最终生产裁决仍需以唯一方向规则、唯一 A/C hook、`Assigned / Unassigned / Tombstoned` 账本及生产消费者接线后的 as-of 重放为准；本报告不为这些尚未落地的选择代填结果。

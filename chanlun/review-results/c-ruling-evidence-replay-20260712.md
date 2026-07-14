# C 裁决证据重放：塔窗口语义 vs 走势类型完成语义（task #60）

- 日期：2026-07-12
- 角色：只读调研员
- 数据：BTC 1m 全历史 4,613,599 bars，末端 `source bar=4,613,598`
- 生产修改：无；本报告是本任务唯一写入的仓库文件
- 结论纪律：本文只提供证据，不替 P0 在 C1/C2 间裁决

## 0. 结论摘要与证据等级

1. **[已验证 E01] 当前/#58 的恰三段塔在 BTC 全历史最高产到 `WindowUnit[6]`。** `WindowUnit[k]` 数依次为 `12,065 / 3,556 / 989 / 243 / 59 / 6`；L0 输入为 40,003 条线段。重放锚：`/tmp/c60-window-replay.log:1-2,6,10,14,18,22`。代码契约是“成功 +3、失败 +1、每个上级对象只含一个中枢”：`rust/src/theta_v0/classifier/recursive_tower.rs:26-33,142-156,189-249`。
2. **[已验证 E02] #58 原型尚不给出唯一的 C2 计数。** 塔对象没有已结算方向，适配器强制调用方显式提供方向；趋势完成又依赖尚未生成的 A/C 配对 hook：`chanlun/review-results/assembler-spec-20260712.md:52,156-164`，`/tmp/c58-assembler-work/rust/src/theta_v0/classifier/move_view.rs:23-45,221-268`。因此本报告给三种确定性方向适配的敏感性重放，不把其中任何一种冒充定义裁决。
3. **[已验证 E03] 三种适配下，C2 候选总数分别为 `12,904 / 8,712 / 8,752`；Completed/Pending 分别为 `10,072/2,832`、`4,507/4,205`、`4,460/4,292`。** 逐级数据见 §2，原始锚 `/tmp/c60-window-replay.log:3-25`。
4. **[已验证 E04] #54 的 `213/93/38/0` 不能原数搬到 C2。** 在与 #54 分支对象最接近的 ownership/fallback 映射中，213 个成功 pair 仅 22 个映射为两个不同的 Completed 视图，146 个至少一侧 Pending，45 个至少一侧无归属；93 个 late-success 相应为 `7/69/17`。重放锚 `/tmp/c60-evidence-replay.log:28-29`。这只是对象映射，不是完成版 C2 对 third 的重新裁决。
5. **[已验证 E05] “塔游标 +1 漏段”共 `6,155/56,915=10.814372%`，另有未进入完整三元组的尾段 6 个。** 三种适配下，最终前缀可归入任一 AssembledMove（含 Pending）的漏段为 `6,054/5,881/5,842`，可归入 Completed 的为 `1,714/1,433/1,481`，最终仍无归属为 `101/274/313`。重放锚 `/tmp/c60-window-replay.log:2-26`。
6. **[已验证 E06] C2 原型会产生结构上的 Trend 候选，不是 0。** 三种适配分别出现 `98/115/152` 个、均为 Pending；Completed Trend 仍是 0，因为未提供 A/C hook，而不是因为组装器不能形成 ≥2 中枢的 Trend kind。代码锚 `/tmp/c58-assembler-work/rust/src/theta_v0/classifier/move_view.rs:368-395`；重放锚 `/tmp/c60-window-replay.log:3-25`。
7. **[已验证 E07] 原文反对“固定恰三段封闭即完成”作为一般定义。** 三个重叠次级别走势只使盘整“随时可以完成”，原文紧接着说它也“可以不结束”并无限延伸；趋势则至少两个中枢且也可继续延伸：`docs/chanlun/text/blog/018-第18课.md:40-44`。第45课的三段即结束是一个“最弱情况”示例，且同文明确“事先是不可能知道的”：`docs/chanlun/text/blog/045-第45课.md:16-30`。
8. **[推断 I01] 原文的“当下临时划分→新材料后修改→完成后不可修改”，在认识论结构上与 C2 的 `as_of + Pending/Completed + supersede` 同构；原文没有规定实现字段、版本 ID 或 ledger API。** 原文锚：`docs/chanlun/text/blog/069-第69课.md:24-26`、`docs/chanlun/text/blog/070-第70课.md:20-40`；C2 契约锚：`chanlun/review-results/assembler-spec-20260712.md:54-73,150-164`。

## 1. 重放口径、基线分叉与可复现性

### 1.1 两条基线必须分开

- **[已验证] 当前主工作树/#58 基线**是恰三段、成功 `i+=3`、失败 `i+=1`、不吸收中枢延伸的窗口塔：`rust/src/theta_v0/classifier/recursive_tower.rs:189-249,278-281`。§2、§5、§6 的计数来自此基线。
- **[已验证] #54 审计工作树**已经是后来的可变长窗口：seed 三段后把延伸段一并放进 `subs`，并带 `CpScanOwnership`：`/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:290-329`。#54 的 2,630 个对象和 `213/93/38/0` 在该基线上产生：`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:3-18,48-64`。
- **[已验证]** 因这两个对象宇宙不同，§3 对 #54 所做的是“身份映射/可迁移性审计”，不是把 #58 原型接入 #54 生命周期后的最终 C2 重跑。最终 C2 数在生产入口、方向和 A/C hook 都完成前是**未定义**，不是 0。#58 自己也声明生产调用方为 0：`chanlun/review-results/assembler-spec-20260712.md:7-18,203-209`。

### 1.2 输入与输出锚

- **[已验证]** exact-three 重放：`bars=4,613,599, l0_segments=40,003, levels=7, highest_window_level=6`：`/tmp/c60-window-replay.log:1`。
- **[已验证]** #54 映射重放：同一 bars/L0 输入，#54 分支 `levels=6`：`/tmp/c60-evidence-replay.log:1-2`；#54 原报告数据锚相同：`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:3-8`。
- 三个方向适配仅用于敏感性枚举：
  - `first_leaf`：取构成链首个叶单元方向；
  - `first_last_envelope`：以组首尾外缘方向；
  - `sequence_envelope`：以相邻窗口外缘序列方向。

  **[推断边界]** 三者都是确定性代码枚举，适配定义见 `/tmp/c58-assembler-work/rust/src/bin/c60_window_replay.rs:14-56`；但没有一个得到 #58 规范授权为“真实方向”。这一缺口由原型显式拒绝默认 `fold_direction` 证明：`chanlun/review-results/assembler-spec-20260712.md:52`，`rust/src/theta_v0/classifier/recursive_tower.rs:159-177,387-407`。

## 2. 问题 1：WindowUnit[k] 与 CompletedMove[k] 计数

### 2.1 窗口计数与视图状态

表中 `C/P` 是 `Completed/Pending`；“视图数”实际类型名是原型的 `AssembledMove`，仅 `C` 行可称完成视图。

| k | 下级单元 | WindowUnit[k] | first_leaf 视图 C/P | first_last 视图 C/P | sequence 视图 C/P | 输出锚 |
|---:|---:|---:|---:|---:|---:|---|
| 1 | 40,003 | 12,065 | 10,039 = 8,356/1,683 | 6,286 = 3,245/3,041 | 6,302 = 3,245/3,057 | `/tmp/c60-window-replay.log:2-5` |
| 2 | 12,065 | 3,556 | 2,219 = 1,398/821 | 1,771 = 915/856 | 1,798 = 883/915 | `/tmp/c60-window-replay.log:6-9` |
| 3 | 3,556 | 989 | 489 = 245/244 | 495 = 267/228 | 496 = 253/243 | `/tmp/c60-window-replay.log:10-13` |
| 4 | 989 | 243 | 128 = 61/67 | 125 = 63/62 | 122 = 63/59 | `/tmp/c60-window-replay.log:14-17` |
| 5 | 243 | 59 | 28 = 12/16 | 33 = 17/16 | 31 = 14/17 | `/tmp/c60-window-replay.log:18-21` |
| 6 | 59 | 6 | 1 = 0/1 | 2 = 0/2 | 3 = 2/1 | `/tmp/c60-window-replay.log:22-25` |
| **总计** | 56,915 | **16,918** | **12,904 = 10,072/2,832** | **8,712 = 4,507/4,205** | **8,752 = 4,460/4,292** | 同上 |

### 2.2 类型分布

| 适配 | Consolidation | Trend | Undetermined | Completed | Pending |
|---|---:|---:|---:|---:|---:|
| first_leaf | 10,076 | 98 | 2,730 | 10,072 | 2,832 |
| first_last_envelope | 4,512 | 115 | 4,085 | 4,507 | 4,205 |
| sequence_envelope | 4,462 | 152 | 4,138 | 4,460 | 4,292 |

**[已验证]** 逐级 kind/status 原数见 `/tmp/c60-window-replay.log:3-25`。盘整若有后继异向 group 则 Completed；末 group 等后继而 Pending；Trend 走 A/C 背驰完成分支：`/tmp/c58-assembler-work/rust/src/theta_v0/classifier/move_view.rs:305-395`。

**[未决]** 表不是“合法方向已裁后的唯一 C2 数”。方向一旦裁定，才可从三列敏感性结果收敛为单列；趋势 A/C 配对接入后 Completed/Pending 还会再变。

## 3. 问题 2：#54 的 213/93/38/0 在 C2 上会怎样

### 3.1 原计数

**[已验证 E08]** #54 原数为：稳定对象 2,630；`SUCCESS=213`；其中首个被评估 pair 失败、后续成功 93；`trend_context=38`；`completed_trend_decomposition=0/213`。失败原子总分布是 `660/1,335/326/10/86/213`：`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:13-18,48-64,430,473-503,557-562`。

### 3.2 最近可比的对象映射

| #54 集合 | CompletedDistinct | 至少一侧 Pending | 至少一侧无归属 | 合计 | 锚 |
|---|---:|---:|---:|---:|---|
| SUCCESS | 22 | 146 | 45（leave 26 + retest 19） | 213 | `/tmp/c60-evidence-replay.log:28-29` |
| late SUCCESS | 7 | 69 | 17（leave 8 + retest 9） | 93 | `/tmp/c60-evidence-replay.log:28-29` |

方向敏感性很大：first_leaf 映射 213 为 `collapsed=62 / completed-distinct=34 / pending=69 / unassigned=48`；first_last 为 `81/17/87/28`；sequence/ownership 为 `0/22/146/45`：`/tmp/c60-evidence-replay.log:40-42`。

### 3.3 哪些结论会翻、哪些仍未定

- **[已验证] `213` 会翻为“不能沿用”。** 213 是窗口对象上的 third 成功数；最近映射下只有 22 对同时落在两个不同 Completed 视图，无法把 213 直接当 C2 third 数。C2 需要重新定义离开/回试在 CompletedMove 序列中的相邻关系后重跑。
- **[已验证] `93` 会翻为“不能沿用”。** 最近映射仅 7 对达到 CompletedDistinct；其余 86 对在 C2 当前前缀不是两个完成走势。#54 原报告本来也只证明 firstRetrace 等价证书缺失，而非 93 个假阳性：`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:13-16,430`。
- **[已验证] `38` 不能解释成 38 个 C2 Trend。** #54 的 `trend_context` 是相邻中心几何关系旁路：`/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:1458-1469`；C2 Trend 是同向 group 内多个中心且方向一致：`/tmp/c58-assembler-work/rust/src/theta_v0/classifier/move_view.rs:356-386`。两者对象类型不同。
- **[已验证] `0/213` 的“结果”在无 hook 原型仍是 0 Completed Trend，但“根因”会翻。** C2 已形成 98–152 个结构 Trend 候选；它们为 Pending 是因为缺 A/C hook：`chanlun/review-results/assembler-spec-20260712.md:158-164`。因此“塔永远组不出 Trend kind”被否证；“真实 BTC 上有多少完成趋势”仍未决。
- **[未决]** `2,630` 的 C2 分母、六桶失败原子及 full-qualified 数均没有合法新值。原因是 C2 视图尚未接入 `classify/judge_third_cert`，而且 higher center 也尚未从 CompletedMove 递归重建：`chanlun/review-results/assembler-spec-20260712.md:7-18`；C 裁决文件也明确要求全部重放而非假定维持：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:198-210`。

## 4. 问题 3：firstRetrace 首对顺序的前 10 个分歧样本

以下是按 `(level, B id, pair order)` 排序的前十例。C1 列是 #54 对相邻 WindowUnit pair 的原子；C2 列是 ownership/fallback 视图中的结构状态。`PENDING_PAIR` 不是判负，而是截至末端至少一个走势未完成。

| # | level / B | C1 leave → retest（end bar） | C1 首对 | C2 映射 | 锚 |
|---:|---|---|---|---|---|
| 1 | L1 / `B=L2#31` | `L1#140@77211 → L1#141@77473` | SUCCESS | retest 无归属 | `/tmp/c60-evidence-replay.log:30` |
| 2 | L1 / `B=L2#32` | `L1#147@80880 → L1#148@81489` | DIR_MISMATCH | 同属 `g74`，pair 折叠 | `/tmp/c60-evidence-replay.log:31` |
| 3 | L1 / `B=L2#43` | `L1#203@105904 → L1#204@106148` | SUCCESS | `g102 Trend/Pending → g103 Consolidation/Completed` | `/tmp/c60-evidence-replay.log:32` |
| 4 | L1 / `B=L2#76` | `L1#367@188242 → L1#368@188741` | ANCHOR_NONE | `g176 Undetermined/Pending → g177 Consolidation/Completed` | `/tmp/c60-evidence-replay.log:33` |
| 5 | L1 / `B=L2#81` | `L1#391@199362 → L1#392@199604` | SUCCESS | `g190 Trend/Pending → g191 Consolidation/Completed` | `/tmp/c60-evidence-replay.log:34` |
| 6 | L1 / `B=L2#94` | `L1#445@222212 → L1#446@222948` | ANCHOR_NONE | 同属 `g219`，pair 折叠 | `/tmp/c60-evidence-replay.log:35` |
| 7 | L1 / `B=L2#105` | `L1#496@249030 → L1#497@249487` | SUCCESS | `g245 Pending → g246 Pending` | `/tmp/c60-evidence-replay.log:36` |
| 8 | L1 / `B=L2#143` | `L1#675@333222 → L1#676@333505` | ANCHOR_NONE | `g332 Undetermined/Pending → g333 Consolidation/Completed` | `/tmp/c60-evidence-replay.log:37` |
| 9 | L1 / `B=L2#199` | `L1#954@453811 → L1#955@454304` | SUCCESS | retest 无归属 | `/tmp/c60-evidence-replay.log:38` |
| 10 | L1 / `B=L2#223` | `L1#1053@498567 → L1#1054@498976` | ANCHOR_NONE | 同属 `g537`，pair 折叠 | `/tmp/c60-evidence-replay.log:39` |

**[已验证 E09]** 分歧不是单一 bool 的差别：C2 会把两个窗口折叠进同一走势、让一侧 Pending，或由 lower ledger 归属失败。#58 的 grouping 与 lower-ledger 取料代码见 `/tmp/c58-assembler-work/rust/src/theta_v0/classifier/move_view.rs:305-350`。

**[推断 I02]** 严格 firstRetrace 的 C2 定义至少应先要求“离开走势和回试走势是两个不同、已完成、相邻的 CompletedMove”，再跑离开/回试几何；否则 `collapsed/pending/unassigned` 会被误当作普通判负。该要求与定义“离开段、回试段都是完成的次级别走势类型”一致：`.chanlun/definitions/maimai.md:132-145`，Lean 明示 `firstRetrace=true`：`formal/Origin/BspClassification.lean:107-121`。

## 5. 问题 4：孤儿段规模、可归属率与残余

### 5.1 塔游标漏段

| k | 下级单元 | WindowUnit | 成功窗覆盖 | 失败 +1 漏段 | 漏段率 | 尾段 | 锚 |
|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | 40,003 | 12,065 | 36,195 | 3,806 | 9.514286% | 2 | `/tmp/c60-window-replay.log:2` |
| 2 | 12,065 | 3,556 | 10,668 | 1,397 | 11.578947% | 0 | `/tmp/c60-window-replay.log:6` |
| 3 | 3,556 | 989 | 2,967 | 587 | 16.507312% | 2 | `/tmp/c60-window-replay.log:10` |
| 4 | 989 | 243 | 729 | 259 | 26.188069% | 1 | `/tmp/c60-window-replay.log:14` |
| 5 | 243 | 59 | 177 | 65 | 26.748971% | 1 | `/tmp/c60-window-replay.log:18` |
| 6 | 59 | 6 | 18 | 41 | 69.491525% | 0 | `/tmp/c60-window-replay.log:22` |
| **总计** | **56,915** | **16,918** | **50,754** | **6,155** | **10.814372%** | **6** | `/tmp/c60-window-replay.log:26` |

“失败漏段”只数 `i+=1` 走过而未被后来成功三元组覆盖的单元；尾部不足三元组的 6 个另列。游标语义锚：`rust/src/theta_v0/classifier/recursive_tower.rs:189-207`。

### 5.2 C2 归属

| 方向适配 | 归入任一视图（含 Pending） | 占 6,155 | 归入 Completed | 占 6,155 | 最终无归属残余 |
|---|---:|---:|---:|---:|---:|
| first_leaf | 6,054 | 98.359058% | 1,714 | 27.847279% | 101 |
| first_last_envelope | 5,881 | 95.548335% | 1,433 | 23.281885% | 274 |
| sequence_envelope | 5,842 | 94.914703% | 1,481 | 24.061738% | 313 |

**[已验证 E10]** 原数来自 `/tmp/c60-window-replay.log:3-25`；#58 明确要求从完整 lower ledger 按坐标补回孤儿，而不是拼窗口 subs：`chanlun/review-results/assembler-spec-20260712.md:88-106`，实现见 `/tmp/c58-assembler-work/rust/src/theta_v0/classifier/move_view.rs:319-350`。

**[边界]** “最终无归属”只表示在全历史末端、给定该适配和该原型 grouping 后没有 host；它不是数学上证明未来永远无归属。用户问题中的“永久”若指流式终局，只能在对象被后继走势封闭且坐标域最终化后裁定；当前原型无永久墓碑类型。

## 6. 问题 5：C2 是否产生趋势型 Move（≥2 中枢）

| k | first_leaf Trend | first_last Trend | sequence Trend | Completed Trend |
|---:|---:|---:|---:|---:|
| 1 | 53 | 60 | 84 | 0 |
| 2 | 23 | 30 | 40 | 0 |
| 3 | 18 | 13 | 15 | 0 |
| 4 | 3 | 8 | 6 | 0 |
| 5 | 1 | 4 | 7 | 0 |
| 6 | 0 | 0 | 0 | 0 |
| **总计** | **98** | **115** | **152** | **0** |

- **[已验证 E11]** Trend kind 判据是组内至少两个延伸后中心，且中心方向一致：`/tmp/c58-assembler-work/rust/src/theta_v0/classifier/move_view.rs:186-218,356-386`。逐级数见 `/tmp/c60-window-replay.log:3-25`。
- **[已验证]** 所有 Trend 为 Pending，是因为本次没有合法 A/C pair hook；代码在 hook 缺失时直接 `MissingDivergencePair`：`/tmp/c58-assembler-work/rust/src/theta_v0/classifier/move_view.rs:221-241`。
- **[裁决证据]** “塔窗口对象自身只有一个中枢，因而自身不可能是趋势型完成走势”仍成立：`rust/src/theta_v0/classifier/recursive_tower.rs:142-150`。但“以窗口为种子组装的 C2 视图也不会产生 Trend”已被 BTC 重放否证。
- **[未决]** 真实 Completed Trend 数。必须先裁方向、从构成链生成唯一 A/C 配对并重放 MACD；#54 的 `0/213` 不能替代这个实验。#54 的 38 个 context 只是中心几何旁路：`/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:1458-1469`。

## 7. 问题 6：迁移成本清单（只读）

### 7.1 若选 C1：重命名/重基线清单

#### Rust 消费者

| 文件:行号 | 当前消费 | C1 所需动作 |
|---|---|---|
| `rust/src/theta_v0/classifier/recursive_tower.rs:26-33,94-106,130-177,211-249,387-407` | `LeveledMove/上级走势` 实为单中心三段窗口，方向含占位 | 类型/注释统一改称 WindowUnit；禁止把 fold direction、完成走势、趋势语义外推 |
| `rust/src/theta_v0/classifier/mod.rs:180-205,217-266` | 建塔、`upper_moves`、下一层投影 | `moves_tower/upper_moves` 及 `LevelState.moves` 的口径文档重命名；重基线各级计数 |
| `rust/src/theta_v0/classifier/mod.rs:1114-1165,1203-1219` | 在窗口 `subs` 内抽 B2/S2；Compose 方向为占位 | 将“完整次级别走势”声明降为窗口内结构判据；B2/S2 资格/数量重基线 |
| `rust/src/theta_v0/classifier/descend.rs:60-106` | `RMove::Compose` 与 `descend` 称走势/次级走势 | 明示 descend 返回窗口构件，不证明已完成走势 |
| `rust/src/theta_v0/classifier/rmove_compose.rs:63-73,113-165` | 第二类在 `RMove::Compose` 内识别 | `SecondTypeStructure` 改为 window-structure 语义；firstRetrace/完成性不再暗含 |
| `rust/src/theta_v0/classifier/voice_eat.rs:61-81` | 全塔元素与父子 containment | 元素名改 WindowUnit；几何 containment 基线可保留 |
| `rust/src/theta_v0/strategy/coverage.rs:214-264,305-338,522-541,1755,2097` | `LeveledMove` 塔作为语法元素/父容器 | E、carrier、方向、身份声明改成窗口口径；coverage 与 stale 基线重跑 |
| `rust/src/theta_v0/strategy/interp.rs:238-393,479,638,728,747` | 提取/缓存/解释 `LeveledMove` 塔 | TreeKey、parent、active-leg 的“走势”命名与行为基线重跑 |
| `rust/src/theta_v0/backtest/incremental.rs:54-86` | 跨 bar `LeveledMove` 身份稳定 | 改称 WindowUnit 身份；bit-exact 基线不等于走势完成稳定 |
| `rust/src/theta_v0/backtest/runner.rs:212-216,338-358,592-602,663-670` | 每 bar 因果塔进入 πΘ | 报告、订单、stale/coverage 基线全部标注窗口语义并重跑 |
| `rust/src/theta_v0/backtest/econ_positive.rs:102-107,460,537` | tower 参数及方向/收益解释 | 改窗口类型名并重跑经济性基线 |
| `rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:140`；`l3_fullwindow.rs:440-452,734-747`；`l3_pi_probe.rs:164-205,524-536` | 诊断/回测直接取塔 | 输出字段从 moves 改 window units；所有阈值和零/非零结论重基线 |

#### 定义、形式化与报告

| 文件:行号 | C1 重命名/范围约束 |
|---|---|
| `.chanlun/definitions/level_recursion.md:19-26,70-80,150-178,209` | 把生产 WindowUnit 与定义 `Move[k]` 分栏，禁止声称实现了“已完成次级走势” |
| `.chanlun/definitions/maimai.md:132-145,245-289,319-321` | firstRetrace、1/2/3 类的“完成走势”消费者降格为窗口派生，并列出缺失等价证明 |
| `.chanlun/definitions/beichi.md:28,107-113,345,377` | 趋势背驰/完成资格改成窗口代理名，不可继续叫 completed trend decomposition |
| `docs/formal_axioms.md:82-125`；`docs/reference-theta-v0.md:29-34` | 区分分类实体 TrendTypeInstance 与生产窗口对象 |
| `formal/Foundation/ChanlunInstantiation.lean:105-166,329-350`；`formal/Origin/RecursiveLevelSystem.lean:16-39,75-130` | `composeStep` 产物改注“规范窗口编码”，撤销其等同完成 Move 的自然语言解释；定理本身可不变 |
| `formal/Origin/RMoveCompose.lean:13-19,63-68,104-112,194,391-469`；`formal/Origin/BspClassification.lean:107-121` | 第二/第三类依赖完成走势的桥标为窗口代理/待证，不用同名掩盖 firstRetrace 缺口 |
| `chanlun/review-results/assembler-spec-20260712.md:11-18`；`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:178-192` | 固化 WindowUnit→派生判据口径映射 |
| `/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:13-18,473-503` | `213/93/38/0`、`completed_trend_decomposition`、full-qualified 全部更名并重基线 |

### 7.2 若选 C2：类型迁移的消费入口

| 文件:行号 | 需要从 WindowUnit 迁到 CompletedMove/MoveView 的入口 |
|---|---|
| `/tmp/c58-assembler-work/rust/src/theta_v0/classifier/move_view.rs:23-45,95-140,270-433` | 原型类型本体；先落生产模块，裁定 direction provider、stable ID、supersede 和 A/C hook |
| `rust/src/theta_v0/classifier/mod.rs:217-266` | 递归下一层输入：决定 centers/classification 是否消费 CompletedMove，而非 `upper_moves` 窗口 |
| `rust/src/theta_v0/classifier/mod.rs:1114-1165` | B2/S2 的 parent/subs 必须改为已完成次级走势链 |
| `rust/src/theta_v0/classifier/mod.rs:1203-1219` | 移除 Compose 外缘占位方向，改读 CompletedMove 已裁方向 |
| `rust/src/theta_v0/classifier/rmove_compose.rs:113-165`；`classifier/signal.rs:66` | 第二类结构与信号入口改读 CompletedMove IDs/链，不能按结构值回查窗口 |
| `/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:450-535,1408-1469` | `CpLifecycle/firstRetrace/full trend` 的对象域、相邻关系、trend_context 改到 CompletedMove；213/93/38/0 重放 |
| `rust/src/theta_v0/classifier/voice_eat.rs:61-92` | containment 同时保留 WindowUnit 构成父和 CompletedMove 消费父，类型不可混用 |
| `rust/src/theta_v0/strategy/coverage.rs:214-264,305-338,522-541,1755,2097` | coverage 元素、carrier forest、父方向、held-leg ID 迁到版本化 CompletedMove |
| `rust/src/theta_v0/strategy/interp.rs:238-393,479,638,728,747` | 提取、TreeKey、缓存、active-leg 对 supersede 的处理 |
| `rust/src/theta_v0/backtest/incremental.rs:54-86` | per-bar as-of 视图缓存和 supersede；不能只靠 append-only WindowUnit ID |
| `rust/src/theta_v0/backtest/runner.rs:212-216,338-358,592-602,663-670` | 每 bar 构造 `MoveView(as_of=i)` 后再进入 πΘ；订单确认时点重放 |
| `rust/src/theta_v0/backtest/econ_positive.rs:102-107,460,537`；`l3_delta_r_alpha.rs:140`；`l3_fullwindow.rs:440-452,734-747`；`l3_pi_probe.rs:164-205,524-536` | 所有直接塔参数、结构计数、收益/探针输出迁到 CompletedMove，并保留 WindowUnit 诊断旁路 |
| `.chanlun/definitions/level_recursion.md:70-80,150-178`；`.chanlun/definitions/maimai.md:132-145,245-289`；`.chanlun/definitions/beichi.md:107-113,345-377` | 把规范定义字段映射到 CompletedMove kind/status/evidence；不修改原定义本身 |
| `formal/Foundation/ChanlunInstantiation.lean:105-166`；`formal/Origin/RecursiveLevelSystem.lean:16-39`；`formal/Origin/RMoveCompose.lean:104-194` | 新增 WindowUnit→CompletedMove quotient/assembly 桥，原 composeStep 保留为窗口种子层 |

**[已验证 E12]** 上表不是“所有文件都必须改代码”，而是静态直接消费入口：`rg` 在 `rust/src/theta_v0` 找到 12 个直接包含 `LeveledMove/classify_with_tower` 的生产/诊断文件；#58 当前生产调用方为 0：`chanlun/review-results/assembler-spec-20260712.md:7-18,207`。C2 的实质成本是新增一条版本化类型边界并迁移上述入口，而非替换一个 typedef。

## 8. 问题 7：原文文本审计

### 8.1 扫描方法与全量语义账本

**[已验证 E13]** 扫描域为 `docs/chanlun/text/blog/` 全部 110 个 Markdown 文件。两遍关键词族为：

```text
完成族：走势类型.{0,18}(完成|结束|终完美)|完成.{0,18}走势类型|未完成.{0,18}走势
当下族：当下.{0,24}(判断|确认|修改|划分|分解)|事后.{0,18}(确认|判断)
分解族：多义性|分解.{0,18}唯一|唯一.{0,18}分解|划分.{0,18}(多种|多义|唯一)
```

宽松扫描原始命中为完成族 203、当下族 199、分解族 39；组合正则产生 173 行候选账本：`/tmp/c60-text-completion.txt:1-203`、`/tmp/c60-text-asof.txt:1-199`、`/tmp/c60-text-decomp.txt:1-39`、`/tmp/c60-semantic-ledger.txt:1-173`。随后剔除图片标题、仅含关键词的学生问题、编者“娇注”及逐字重复转载；下表按语义簇枚举作者正文/作者答疑，重复段落只保留首发锚。

### 8.2 涉及完成、终结、当下确认与多义分解的原文段落

| 语义簇 | 文件:行号 | 短引 | 审计标签 |
|---|---|---|---|
| 走势终完美与当下两难 | `docs/chanlun/text/blog/017-第17课.md:22-36` | “究竟是继续延续还是改变” | 明确反对预先唯一知道终点 |
| 盘整/趋势定义 | `docs/chanlun/text/blog/017-第17课.md:38-52` | “盘整只包含一个中枢；趋势至少两个” | 明确支持完成走势分类 |
| 已完成与未完成按级别不同 | `docs/chanlun/text/blog/017-第17课.md:666-668` | “日线上未完成；30分钟三种完成走势连接” | 明确支持级别化状态 |
| 中枢组件必须完成 | `docs/chanlun/text/blog/017-第17课.md:1098` | “前三个走势类型都是完成的” | 明确支持 Completed component |
| 完成判断作为待解核心 | `docs/chanlun/text/blog/017-第17课.md:1160-1162` | “怎么判断某种走势类型的完成” | 原文不把三段直接当答案 |
| 三段只给可完成条件 | `docs/chanlun/text/blog/018-第18课.md:24-40` | “可以随时完成……可以不结束” | 明确反对固定三段即完成 |
| 趋势也可无限延伸 | `docs/chanlun/text/blog/018-第18课.md:42-44` | “形成两个中枢后……也可不断延伸” | 明确反对固定中心数即终结 |
| 未完成类型不可当组件 | `docs/chanlun/text/blog/019-第19课.md:458-484` | “不完成的怎么知道要演化成什么类型” | 明确支持完成后组装 |
| 次级完成仍需再下一级 | `docs/chanlun/text/blog/020-第20课.md:274` | “次级别的完成，需要再次级别” | 明确支持递归确认 |
| 完美保证第三段出现 | `docs/chanlun/text/blog/021-第21课.md:44,210` | “其后必然有第三段” | 支持最低构成，不等于第三段时走势已终结 |
| “三段以上”不是“完成点” | `docs/chanlun/text/blog/022-第22课.md:686` | “完成必须包含三个以上次级别走势” | 只给必要条件 |
| 回抽完成由新走势确认 | `docs/chanlun/text/blog/023-第23课.md:1106-1108` | “产生新的走势……代表回抽走势完成” | 明确支持后继确认 |
| C 段完成与提前外推 | `docs/chanlun/text/blog/024-第24课.md:24-28` | “面积不需要全出来……乘2” | 正式完成与当下候选并存 |
| 完成走势在本级明显 | `docs/chanlun/text/blog/028-第28课.md:848` | “完成的走势类型……很明显” | 支持可识别，不定义固定窗 |
| 三个完成走势构成中枢 | `docs/chanlun/text/blog/031-第31课.md:301-303` | “三个完成的走势类型的叠加” | 明确支持组件完成 |
| 已有中枢无预测含糊 | `docs/chanlun/text/blog/032-第32课.md:30` | “可以用定义严格判别” | 已形成对象可当下确认 |
| 多义性来自延伸/精度 | `docs/chanlun/text/blog/033-第33课.md:14-22` | “走势呈现一种多义性” | 明确支持多视角，不支持胡分 |
| 连接结合律与当下分解 | `docs/chanlun/text/blog/036-第36课.md:14-30` | “基础在于采取的分解方式” | 明确支持重组选择 |
| 未完成到完成的当前重组 | `docs/chanlun/text/blog/036-第36课.md:34-36` | “第二个中枢还没最终完成” | 明确支持当前态 |
| 同级分解规则 | `docs/chanlun/text/blog/038-第38课.md:18-28` | “多义性不是含糊性……分解也是唯一” | 支持受规则约束的自由 |
| 结束依赖背驰/三买卖 | `docs/chanlun/text/blog/038-第38课.md:94,254-300` | “没背驰……走势类型没结束” | 明确反对固定窗口终结 |
| 趋势至少两个中枢 | `docs/chanlun/text/blog/041-第41课.md:182` | “上涨、下跌都至少有两个以上次级别中枢” | 明确反对单中心 Trend |
| 普通完结与小转大 | `docs/chanlun/text/blog/043-第43课.md:42-48` | “完结有相应两种情况” | 完成路径不唯一 |
| 当下处理、非预测 | `docs/chanlun/text/blog/044-第44课.md:40-50` | “当下能反应……不是预测” | 明确支持操作候选 |
| 最弱三段例与不可预知 | `docs/chanlun/text/blog/045-第45课.md:16-30` | “最弱的情况……事先不可能知道” | 特例支持三段结束；反对普遍化 |
| 中枢结束后再看走势完成 | `docs/chanlun/text/blog/049-第49课.md:40,54` | “分析……走势类型的完成” | 中枢完成≠走势完成 |
| C2 未完成不可下结论 | `docs/chanlun/text/blog/051-第51课.md:320` | “C2都没完成，怎么知道” | 明确反对未完成对象冒充完成 |
| 5分钟完成需1分钟图 | `docs/chanlun/text/blog/052-第52课.md:222` | “精确判断一定需要1分钟图” | 明确支持递归证据 |
| 多种分解均可但要可操作 | `docs/chanlun/text/blog/054-第54课.md:46-74` | “哪一种都可以” | 支持有选择的分解规则 |
| 多义分法用于操作 | `docs/chanlun/text/blog/057-第57课.md:40,96` | “这种分法和原来……不同” | 明确支持多义操作视图 |
| 未完成走势的另一分解 | `docs/chanlun/text/blog/059-第59课.md:40-42` | “未完成……对应另一种分解” | 明确支持 provisional decomposition |
| 当下分解多样性 | `docs/chanlun/text/blog/060-第60课.md:38,100` | “当下分解的多样性” | 明确支持多样性 |
| 只记最近未完成对象 | `docs/chanlun/text/blog/063-第63课.md:18-26` | “最近一个未完成的走势类型” | 明确区分已完成/未完成账本 |
| 线段未破坏就未完成 | `docs/chanlun/text/blog/066-第66课.md:234` | “A线段没完成，等A完成再说” | 后继证据确认边界 |
| 同一规则下唯一划分 | `docs/chanlun/text/blog/067-第67课.md:52` | “同一级别……唯一地划分” | 明确反对规则内任意性 |
| 收盘时完成无从判断 | `docs/chanlun/text/blog/068-第68课.md:128` | “是否完成，无从判断” | 明确允许 Pending |
| 临时确认与后来修改 | `docs/chanlun/text/blog/069-第69课.md:24-26` | “暂时看成……有可修改的地方” | 与 as-of/supersede 高度同构 |
| 结合律下当下变换 | `docs/chanlun/text/blog/070-第70课.md:18-40` | “暂时先……等走势走出最自然选择” | 明确支持 provisional regrouping |
| 超短线多义视图 | `docs/chanlun/text/blog/071-第71课.md:126` | “根据走势的多义性” | 支持操作尺度选择 |
| 等待上涨走势结束 | `docs/chanlun/text/blog/073-第73课.md:100` | “什么时候结束……应等待什么” | 不支持固定窗自动结束 |
| 当日顶分型操作 | `docs/chanlun/text/blog/079-第79课.md:124` | “不是收盘等……很明确了再走” | 支持候选/风控先行 |
| 走势未结束随时新高 | `docs/chanlun/text/blog/082-第82课.md:50` | “走势没结束，随时新高” | 明确反对提前 Completed |
| 不可预测但可判断 | `docs/chanlun/text/blog/084-第84课.md:28-30` | “不可绝对预测……可判断” | 支持分类边界，不支持预知终点 |
| 当下完全分类 | `docs/chanlun/text/blog/094-第94课.md:18-26` | “分类与边界的当下确认性” | 支持 as-of 分类 |
| 等周一确认线段破坏 | `docs/chanlun/text/blog/098-第98课.md:18` | “等……当下地决定” | 后继状态确认 |
| 唯一递归分解 | `docs/chanlun/text/blog/101-第101课.md:52-64` | “递归函数……唯一地分解” | 规则确定后应唯一 |
| 多种体系、各自唯一表达 | `docs/chanlun/text/blog/102-第102课.md:14-32` | “诸多唯一分解方式……唯一地表达” | 支持选择体系，不支持体系内歧义 |

### 8.3 三个直接问题

#### (a) 是否存在支持“固定恰三段窗口封闭即完成”的原文？

**结论：原文反对（作为一般口径）；仅有特例，不构成普遍规则。**

- **[明确反对]** 第18课把三段重叠定义为盘整达到“随时可以完成”的最低条件，同时明说“可以不结束”、可无限延伸：`docs/chanlun/text/blog/018-第18课.md:40`。
- **[明确反对]** 趋势至少两个中枢，达到后也可继续延伸：`docs/chanlun/text/blog/018-第18课.md:42-44`。
- **[特例]** 第45课的“最弱情况”举例，三段 5 分钟走势后 30 分钟走势恰好结束；但同段说事前不可能知道，并把它标为最弱演化：`docs/chanlun/text/blog/045-第45课.md:16-30`。
- **[明确支持的只是必要条件]** “任何走势类型至少由三段以上次级别走势构成”：`docs/chanlun/text/blog/018-第18课.md:34-36`；“以上”不能推出“恰三段且第三段一出即封闭”。

#### (b) “完成只能事后确认”的当下操作学如何处理；是否与 C2 同构？

**结论：原文明确支持“当下候选/临时划分 + 后继修正 + 完成后冻结”；对具体 C2 数据结构原文沉默。**

- 当下无法消除“延续或改变”两难：`docs/chanlun/text/blog/017-第17课.md:26-36`。
- 实盘不必等正式 C 段全出，可对 MACD 面积外推形成候选：`docs/chanlun/text/blog/024-第24课.md:24-28`。
- 临时分型可随新材料修改，完成图形不可再修改：`docs/chanlun/text/blog/069-第69课.md:24-26`。
- 走势按当前最有利/自然分解暂时处理，未来再改为更合理划分：`docs/chanlun/text/blog/070-第70课.md:20-40`。
- **[推断]** 这与 C2 `as_of`、Pending/Completed、supersede 的状态机形状同构；C2 的前缀纯函数和显式 Pending 见 `chanlun/review-results/assembler-spec-20260712.md:54-73,150-164`。原文没有指定版本号、对象 ID、不可变账本或 API，不能声称实现逐字段同构。

#### (c) 多义性是否给组装器的分解选择留自由度？

**结论：原文明确支持“受约束的选择自由”，同时明确反对任意/非唯一分解。**

- 支持：走势连接遵守结合律，可按操作级别重组；当下可选不同观察分解：`docs/chanlun/text/blog/036-第36课.md:14-30`、`docs/chanlun/text/blog/054-第54课.md:46-74`、`docs/chanlun/text/blog/070-第70课.md:18-40`。
- 约束：多义性“不是含糊性”，选定同级分解规则后结果应唯一：`docs/chanlun/text/blog/038-第38课.md:18-28`；后期课程进一步要求递归函数唯一分解：`docs/chanlun/text/blog/101-第101课.md:52-64`、`docs/chanlun/text/blog/102-第102课.md:14-32`。
- **[推断]** 因而组装器可以在 P0 明示的合法规则族里选择（例如操作级别、结合方式、as-of 观察窗），但每个规则必须给确定输出、满足结合/前缀约束，并能解释 supersede；三种方向适配不能永久并存为三个都叫“规范真值”。

## 9. 对 C 裁决文件第 7 节五个待裁问题的证据映射

以下问题编号对应 `chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:252-260`。

### Q1. `LeveledMove` 是完整走势，还是中枢窗口单元？

- 支持 C1：E01（代码/Lean 现实对象是恰三段单中心窗口）、E05（游标语义及漏段可精确解释）、E12（已有消费者广、C1 改名成本可枚举）。
- 支持 C2：E03（组装后对象数/状态显著不同）、E06/E11（窗口种子可形成多中心 Trend）、E07/E13（原文把三段最低条件与完成明确分开）。

### Q2. C2 是否接受 as-of 版本、Pending/Completed 与 supersede？

- 支持 C1：E02（direction/A-C 未裁使 C2 当前无唯一真值）、E12（全消费链迁移成本高）。
- 支持 C2：E05/E10（lower-ledger 可补回 94.9%–98.4% 漏段）、E07、I01（原文临时确认/修改/冻结与 as-of 形状一致）。

### Q3. 若取 C1，是否接受把延伸、趋势、firstRetrace 降为窗口派生语义？

- 支持 C1：E01（窗口契约自洽）、E05（窗口游标账可完整审计）。
- 反向压力/支持 C2：E04/E08/E09（213/93 对象域不稳）、E06/E11（趋势信息在组装层真实出现）、E07（原文反对三段即完成的一般化）。

### Q4. 若取 C2，CompletedMove 是否应成为 higher center、买卖点、背驰、区间套的共同对象？

- 支持 C1：E02（当前 C2 缺唯一方向和 A/C hook，直接迁移会把 Pending 当 false）、E12（入口多且需并行保留窗口诊断）。
- 支持 C2：E04/E09（firstRetrace 在 WindowUnit 与 CompletedMove 上发生折叠/Pending/无归属）、E06/E11（只有组装视图出现结构 Trend）、原文组件必须完成：`docs/chanlun/text/blog/019-第19课.md:458-484`、`docs/chanlun/text/blog/051-第51课.md:320`。

### Q5. 裁决前是否冻结 A/B，裁决后重放 #54？

- 支持冻结：E02（C2 真值仍未定义）、E04/E08（213/93/38/0 均不可原样迁移）、E09（前十例已显示对象身份改变）、E11（0 的根因已变但完成趋势数未得）。
- 支持裁后重放范围：E12 的 C2 入口清单；必须重建 `2,630` 分母、六桶原子、`213/93/38/0`、full-qualified 与每 bar as-of 确认时点。

## 10. 未决项（不得误读为 0）

1. 合法 CompletedMove direction provider 尚未裁定：`chanlun/review-results/assembler-spec-20260712.md:52`。
2. 唯一趋势 A/C pair 自动生成尚未实现；无 hook 时全部 Trend Pending：`chanlun/review-results/assembler-spec-20260712.md:156-164`。
3. #58 没有生产调用方，尚未在 CompletedMove 序列上重写 third/firstRetrace：`chanlun/review-results/assembler-spec-20260712.md:7-18`。
4. higher center 是否从 CompletedMove 递归重建尚未实现；因此 C2 的最高可达级、2,630 分母和 `213/93/38/0` 最终值都未定义：`chanlun/escalate/c-ruling-tower-window-vs-move-completion-20260712.md:198-210`。
5. §5 的“最终无归属”是末端前缀残余，不是理论永久墓碑：`/tmp/c60-window-replay.log:2-26`。

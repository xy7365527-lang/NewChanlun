# #610 调研：signal.rs digest guard 基线红——引入提交归因

- 日期：2026-07-28｜票据：issue #610（调研，sonnet 常规档，只读）
- 工位：`/tmp/kimi-nest-mainline`（只读）+ `/tmp/issue610-bisect`（临时 worktree，已 `git worktree remove`，无遗留）
- 方法：`git log -S` 锁定 GOLDEN 常量写入点 → `git bisect run`（编译失败自动 skip，wrapper 环境覆盖）二分收窄 → 对剩余 2 个候选提交逐行读 diff 定性
- 全程零 git mutation（除授权的 `git worktree add/remove`）；未改任何源文件

## ① 翻转引入提交

**`bbbd8f89fa83968e26cb0f9a98b73eea8023c7a6`**（2026-07-24 19:22:36 -0400，作者 xy7365527-lang）

> feat(classifier): P1→#214→#218 证书索引口径三部曲（依赖序，spec 互引不可分）

票面：#218（spec `chanlun/review-results/spec-owner-attribution-fix-20260724.md`，"owner 归属修正实装"），顺带捎带 #110（`level_origin` 级别标签字段，提交注记"同文件不可分"）。目的：修正 BSP 点的 owner 归属载体错配（归属层混装确认层对象 c1 的根因修复），#211 终裁路线 B。

## ② 翻转机制

`bbbd8f89fa` 对 `rust/src/theta_v0/classifier/{bsp.rs,signal.rs}` 做了两处**同时**改变 `BspPoint` 的 `#[derive(Debug)]` 输出的改动（`extract_signals_bit_exact_digest_guard` 的 GOLDEN 摘要即 FNV-1a over `format!("{:?}", pts)`，历史上 `struct_break_dir`/`force` 两次新字段落地都记录为"诚实翻转"先例——这是同一机制的第三次发生）：

1. **新增字段 `level_origin: u32`**（`bsp.rs` `BspPoint` 结构体，#110 面）——`make_first_point`/`make_second_point`/`make_third_point` 三处构造点全部硬编码 `level_origin: 0`。该字段进入 `#[derive(Debug)]`，247 个电池点的 Debug 串每条尾部新增 `, level_origin: 0`。
2. **`center` 字段类型改写**：`Option<Center>` → `Option<OwnerRef>`（#218 面 A）。
   - 一/三类点：`Some(*c)` → `Some(OwnerRef::Center(*c))`——Debug 串从 `Some(Center { .. })` 变为 `Some(Center(Center { .. }))`（枚举变体名与内层结构体同名，实际是双层包裹）。
   - 二类点：`Some(*c)`（`c` = 判定中枢 c1）→ `Some(OwnerRef::Type1Anchor(type1_src))`——**不只是包裹形式变化，值本身也换了**：从中枢对象变成一个 `usize` 坐标（该走势一类点身份锚），这是 #218 spec 明确宣称的"故意改判定集合"（spec 第 12 行原话）。

两处改动叠加 → `bit_exact_battery_digest()` 从 `0x90c7_9ee6_17e1_1392`（GOLDEN，f575b8fe60 诚实重锚以来未变）翻转为当前实测 `0xe6a2_63e3_43e4_3845`。

## 二分过程记录

1. `git log -S "0x90c7_9ee6_17e1_1392"` 命中两条：`f575b8fe60`（写入现值，"GOLDEN 诚实重算"）与更早的 `640609071d`（收口提交，同一常量值已存在）——确认现值自 f575b8fe60（2026-07-03）起未再改写。
2. `/tmp/issue610-bisect` worktree 锚定 f575b8fe60：**编译失败**（10 处 E0063/E0061/E0560，因该提交本身非自洽——需数个后续 commit 补齐才能自编译，属该期历史正常态）。
3. 顺时间线找到 `594d2ea738`（"#115 涟漪补齐——恢复 HEAD 自编译"，f575b8fe60 之后第 5 个全历史提交）：编译成功，`extract_signals_bit_exact_digest_guard` **PASS**（digest = GOLDEN）→ 定为 bisect good 端。
4. `git bisect` good=594d2ea738 / bad=HEAD(`e47039bdcb`)，run 脚本（`CARGO_BUILD_RUSTC_WRAPPER=` 覆盖历史 worktree 里失效的 `.chanlun/locks/cargo_gate.sh` wrapper 引用，否则大量误判 skip）：收窄到 `640609071d`（good）与 `f0f2a9d9bb`（bad，"map #126 线产物 20260724"）之间。
5. 该区间内触及 `signal.rs`/`bsp.rs`/`mod.rs` 三文件的提交只有两个：`bb01eeb655`（仅动 `mod.rs`，多级投影层接线，与 digest 电池路径无关——电池直调 `extract_signals`，不经 mod.rs stamping）与 `bbbd8f89fa`（动 `bsp.rs`+`signal.rs`，172+64 行）。两者均无法在该期历史里独立编译（同批多提交互相依赖，属该窗口正常态），故未能用编译跑测直接对 `bbbd8f89fa` 单独判定，改为逐行读 diff（`git show bbbd8f89fa -- bsp.rs signal.rs`）定性确认机制（见②）。
6. 交叉验证：issue #434（`.chanlun/worktrees/wt-434-level-origin` 在案）独立确认 `level_origin` 字段由 `bbbd8f89fa` 引入且全仓库恒为 0（从未真正写入非零值）——与本票②的第 1 点互证一致。

## ③ 该提交行为合法性评估（证据，供裁量，不下结论）

- **spec 先行且明确预告 GOLDEN 会翻转**：`spec-owner-attribution-fix-20260724.md` 第 12 行原话——"本票**故意改判定集合**……GOLDEN 翻转走 #110 先例受控流程（ID-6）"；US-07 明确要求"GOLDEN/digest 翻转登记……无一处静默翻转"。
- **spec ID-6 第 5 条（"与既线失败切分"）存在一个例外条款未被兑现**：原文——"`extract_signals_bit_exact_digest_guard` 已在 #110 线失败在案，勿修勿归因；**若面 A 改变该 guard 实际输出，登记「既线失败在案 + 实际值二次漂移归面 A」**"。本票②已证：面 A（`OwnerRef::Type1Anchor` 值本身改变，非仅包裹）**确实**改变了 guard 实际输出——触发该例外条款的登记义务。但全仓库检索（`grep -rl "GOLDEN 翻转\|digest_guard"` review-results）未发现任何"面 A 二次漂移登记"文档——ID-6.5 义务**未见履行证据**。
- **前提性错误**：spec 把该 guard 定性为"既线失败"（隐含"在本提交之前就已经红"），但本票的 bisect（594d2ea738 PASS，即 f575b8fe60→bbbd8f89fa 之间该 guard 一直绿）证明：在 `bbbd8f89fa` 落地之前，该 guard **不是**既有红线，而是**首次**由本提交（level_origin 新字段 + OwnerRef 改写，两者同一提交"顺带入库、同文件不可分"）转红。spec 文本把尚未发生的、本提交自己制造的红线预先记账给"#110 线"，属于**归因前置**（在因果发生前就把责任分配出去），而非对已存在事实的引用。
- **后续 40+ 份评审文档的归因漂移**：`grep` 结果显示此后归因口径反复漂移——"`#110` 在案"（多份 07-24/07-26 文档）→"`#115` 线在案"（`shadow-review-309/310/315/360/365`、`treasury-reverify*` 等 07-27 多份）→"`#491`"（`issue527/601/602/619`、`shadow-601/602-review` 等 07-28 多份）。`shadow-review-309-20260727.md` LOW-3 已指出"#110 已 CLOSED"与代码内注释显式指向 `#115` 的矛盾，但未溯源到真正的引入提交 `bbbd8f89fa`；`shadow-review-360-20260727.md` 更直言"其『#115 线在案』的归属我未能核实"。**本票是该链条第一次实证到具体 commit SHA + 具体机制的归因**。
- **变更本身的工程意图**（不评价对错，仅陈述）：`level_origin` 字段（#110 面）经独立票 #434 证实全仓库恒为 0、从未真正被赋非零值——"名义已加字段、实际未接线"的空转状态，该字段本身尚有未闭环的后续工作（#434）。`OwnerRef` 改写（#218 面 A）经 #211 终裁确认为"故意改判定集合"的正当语义修正（归属/确认分层根因修复），并非误改。

## ④ 重锚 vs 回修裁量素材

| 维度 | 支持"诚实重锚"（更新 GOLDEN=0xe6a2...） | 支持"回修"（改回旧行为使摘要复原） |
|---|---|---|
| #218 面 A（OwnerRef 语义改写） | spec 明确"故意改判定集合"+ #211 终裁，语义正确性已裁定，回修=撤销一个已裁定的根因修复 | 无——回修需撤销 #211 裁定，无证据支持 |
| #110 面（level_origin 字段） | 字段本身按 #110 票意图应该加 | #434 证实该字段全仓库恒为 0，"名义存在、实际未接线"——若判定 #434 应删除该字段（而非补写非零值），则本次翻转的一半成因（level_origin 进 Debug）会随 #434 落地而**自然消失**，届时需再次重锚 |
| 既线协议一致性 | 与 `struct_break_dir`（P2-R2）、`force`（#115）两次先例完全同构——"新字段进 derive(Debug) ⟹ 诚实重锚"是本仓库既定动作模式 | 无对立先例——本仓库从未在此类情形选择回修 |
| ID-6.5 义务缺口 | 无论选哪条路，**先补齐 spec 自己要求的"登记"动作**（面 A 二次漂移登记）是前置的、不冲突的独立义务 | 同左 |

**建议编排者关注点**：若 #434（level_origin 空转字段清理）判定为"删除字段"而非"补写真实级别号"，则本次重锚窗口应等 #434 落地后一次性做（避免两次重锚）；若 #434 判定"补写非零值"，则现在重锚后 #434 落地时会有第三次翻转，同样需要新一轮登记。两条路径都不影响"#218 面 A 的语义改写本身是否合法"这一独立问题——后者证据链已足够支持维持现状（不回修）。

## 影响声明

纯调研产出，未改动任何源文件、未做任何 git mutation（worktree add/remove 已授权且已清理）。本文档本身是新增文件。

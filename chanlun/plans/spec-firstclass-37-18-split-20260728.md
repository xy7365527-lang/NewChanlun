# SPEC：一类点 37:18 分档实装（趋势一类 / 类第一类）

2026-07-28 · #589（地图 #582 交棒件， grilling 后定稿）· 教义依据 = ADR 补充十六全四节 · 前置 = #583/#584/#585/#586/#587/#588 全闭合

## Problem Statement

一类判据（`judge_first_cached`：双中枢 + 破核心 + 037:20 新极值 + MACD C<A）不验 T3-in-c，37:18 否则域（c 无三类点）与真趋势一类混流：「一类点」标签 64–88% 名不副实（#585 三窗普查），Reset 把盘背卖点广播成「走势类型终结」，一类全平对震荡点开火。ADR 补充十六已裁 **(b) 37:18 分档**，本 spec 是其工程落地（grilling 后修订：名分承载走**乙路**——否则域不置一类 bit）。

## Solution

判据加 **T3-in-c 资格检验**（固定首对，补充十五紧邻语义同源）。**乙路**：否则域点**不置一类 bit**——名分 = bit 本身（91:278「连三类买点都没出现，哪里会有背驰」：无 T3 无一类形状可言）；零 bit 结构候选（struct_break_dir=Some、六 bit 全零）继续走既有候选流（P2-R2 先例）。下游消费（Reset/CloseRoot/二类父母）bit 驱动**自动正确**；五桶失败原因入盘背通道亚型记录（观测面，三锁严格性）。金标准反证+重订与切换**同票完成**（#486 先例，main 不红）。

## User Stories

1. 作为策略，我只在趋势一类全平主仓（29 课定理），否则域扛震荡不被盘背点错杀；
2. 作为 lifecycle，我只对趋势一类发 Reset，「终结广播」名实相符；
3. 作为观测面，我在盘背通道看到 native/otherwise-domain 亚型计数与 T3-in-c 失败五桶，否则域全程可解释；
4. 作为审计，我拿到 counterfactual 反证（强制旧行为 = 旧字节恢复）与逐案对拍（固定首对 vs #585 临时口径），使切换可归因、金标准重订可批准。

## Implementation Decisions

### D1 判据：T3-in-c 固定首对分级器

- **接入**：`judge_first_cached` 内，`broke` + `a_seg` + 037:20 极值 + `λ_C` 齐备之后、产出 BspPoint 之前（signal.rs:342-357 一带）。
- **判定**：锚 = **B 右边固定首对**——leave = 本级第一条 `start_index >= last_center.end_index` 的段（共享枢轴：等值即紧邻，补充十五），retest = 紧随一条；方向互反；价格严格（Up：leave.end_price > ZG ∧ retest.end_price > ZG；Down 镜像 < ZD）；任一失败即 Missing，**不后扫**（`trend_third_class_in_c` 扫描器不复用，#586 在案；判定锚是 B 右边**不是** λ_C/c_start）。
- **返回**：`Present` / `Missing(reason)`，五桶 `missing_leave / missing_retest / same_direction / leave_not_outside / retest_reentered`（连续缺口单腿 c → `missing_retest`）。
- LevelView T3 流本体不动；两流共享固定首对分级器（#586）。

### D2 名分承载（乙路）：否则域不置一类 bit

- `Present` ⟹ 置 buy1/sell1（趋势一类，现状路径全不动）；`Missing` ⟹ **不置一类 bit**，点以零 bit 结构候选（`struct_break_dir=Some`、六 bit 全零）继续走既有候选流（P2-R2 先例，消选择偏差语义不变）。
- **无 BspBits 旁载**：名分 = bit 本身，不挂附件（grilling 裁定：91:278 下无 T3 无一类形状；fail-closed——忘读的消费端只是看不见一类点，不会误消费；#542 式旁载不需要）。
- `class_index`/disc 对否则域点变化（如 sell3-only 32→0）——预期内行为变更，反证覆盖。

### D3 消费矩阵（#587 裁定，乙路下自动正确）

- **Reset**：`Missing` 无一类 bit ⟹ lifecycle 不产 Reset、不产任何事件（事件流安静，补充十四咬合）；漏发桶口径收紧为「趋势一类 Reset 到场仍有活中枢」，历史 58 条基线留档注口径切换（不混比）。
- **CloseRoot**：主策略全平路径（compose.rs:360 / interp.rs:370）只见到 Trend 点（Quasi 无一类 bit）——**改读面为零**，行为自动正确；主仓否则域扛震荡。
- **类第一类**：不新造 campaign 触发源（#587；campaign 触发形状 = 本级中枢在场 + 次级别买卖点 + 边界侧，盘背本为辅助参考；否则域点是本级别点，形状塞不进次级别触发——级别问题，本级别不管为正解）。
- **二类点继承分档**：二类判据的父母无一类 bit（Quasi）⟹ 不产真二类，自动正确；`Quasi` 后回抽点 = 类第二类（027:68）归盘背通道；回抽不重回中枢 = 真三类（024:36）走正常三类流。
- **迟到三类点**（086:76）：登记观测不作独立信号消费（现状，注释锁定）。

### D4 观测面（三锁严格性）

- 盘背通道同桶分亚型 `native / otherwise-domain`；五桶失败原因记入 otherwise-domain 亚型记录。
- **三锁**：①键唯一（级别 + source_index + 侧 + 中枢身份，#498 已证全窗无碰撞）；②一一对应（每否则域点恰好一条亚型记录，判定去重）；③**账平断言**（否则域点数 = 亚型记录数，进 wf8 验收行）。
- trades.jsonl **不再改 schema**——名分可由 #542 已落的 `bsp_bits_class_index` 直接读出（乙路红利，省一次金标准扰动）。
- 趋势一类/类第一类计数、Reset 数（新口径）、漏发桶（新口径）进 wf8 验收行。

### D5 金标准与 counterfactual

- **反证**：env-gated counterfactual 开关（强制旧行为：跳过 T3-in-c 合取）⟹ trades/tower/四层旧字节逐字节恢复——差异源唯一（分档）；
- **重订**：与切换**同票**（S2，#486 先例）：反证到手 → 用户批准 → merge 与新锚同落，main 不红；旧锚留底 pre 目录，ADR 补充九登记尺寸+SHA；
- **多窗对照**（S3）：wf8 + wf7 + p3fold 双臂读数；Reset 预期大降（#585 临时口径 59→21/33→4/87→20，固定首对口径下否则域只会更多）；四层照实报不预承诺。

### D6 ADR 回填

- 补充十六：实装完成注记（merge sha 占位）+ S1 逐案对拍结论 + 多窗读数登记；补充十三/十四/十五联动注记各一行；CONTEXT.md「类第一类与否则域」词条更新为实装完成态。

### 边界

不碰 #59 线；不改 LevelView T3 流本体；不动 bsp 六 bit 语义（只动否则域点的产出）；不新造 campaign 触发源；执行层 = **claude CLI：实装 sonnet、影子评审 opus**（codex 额度尽，8/3 重置）；实测独立 worktree + 独立 CARGO_TARGET_DIR；git 合入逐次用户批准。

## Testing Decisions

- 好测试 = 锁外部行为；正反例全锁（连续缺口 `missing_retest`、同向续行 `same_direction`、首对失败远对不认领、固定首对锚非 λ_C 坐标案例、零 bit 结构候选继续入流）；
- 分级器单测（五桶全盖）+ 三锁（键唯一/一一对应/账平）+ 消费端自动正确见证（Reset 只属趋势一类、CloseRoot 不见 Quasi、二类父母无 Quasi）+ 双臂四层一致 + MisKill=0；
- S1 对拍：固定首对 vs #585 临时口径**逐案归因**（非总数对比）；
- prior art：#487 接线测试族、#542 schema-only 守卫、#489 终态切换测试、#486 counterfactual 反证。

## Out of Scope

LevelView T3 流本体；campaign 触发逻辑；#59 领土；类第一类作触发源的任何发明；分档后策略参数调优（另案）。

## 切票（已按 grilling 裁定塌缩为三张）

| 票 | 内容 | 依赖 |
|---|---|---|
| **S1**（tracer，行为零变化） | D1 分级器 + D4 观测面（亚型记录/五桶/三锁）——bit 生产不动，Reset/全平照旧；与 #585 临时口径**逐案对拍**归因 | 无 |
| **S2**（大闸） | D2 bit 切换（否则域不置一类 bit）+ 全链路见证锁（消费自动正确/账平/双臂一致/MisKill=0）+ **counterfactual 反证 + 金标准重订同票**（单独请批） | S1 |
| **S3** | 多窗对照（wf7/p3fold）+ D6 ADR 回填 + 报告（不碰闸门） | S2 |

每票照旧闭环：claude sonnet 实装 → opus 影子评审 → 修复 → 用户批准合入 → main 侧全量验证 → 关票。

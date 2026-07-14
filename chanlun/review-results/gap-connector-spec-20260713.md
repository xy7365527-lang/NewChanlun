# gap-connector spec（adopt_v2：跳空按原文归类为级别无限低合法连接段）

日期：2026-07-13
状态：spec-draft（待隔离原型 + property test + 双重放验收）
任务：#72
原文依据：`chanlun/review-results/gap-handling-origtext-20260713.md`（chan99/0046:85/171、blog/028:308/746/754、blog/036:34/128-131、blog/057:28/32）
底数依据：`chanlun/review-results/seed-failure-geometry-20260713.md:98,108-117,147-194`、`chanlun/review-results/adopt-overlay-spec-20260713.md:37-52,258-264`、`chanlun/escalate/adopt-overlay-residual-ruling-20260713.md`

## 0. 目标域

adopt_v1 验收口径 `52→33`（+19 获 host，7 Completed，regressions=0）后的 33 个 Unassigned：

| 桶 | 数量 | 覆盖规则 |
|---|---|---|
| 跳空（GAP_ADJACENT：I1∩I2=∅ 或 I2∩I3=∅） | 19 | R1–R4 直接覆盖 |
| 单调陡峭 | 11 | R2' 扩展假设（blog/028:754，需独立验证，不得与跳空混桶验收） |
| 其他 | 3 | 本任务不承诺；重放后单独归因 |

判例对象：`L0#37199`（跳空，基座有 host，曾被旧 DP 释放、已由单调门保住）——本 spec 须给出其 host 归属的**合法规则依据**，兑现 escalate 裁决第 25 条"给出合法规则"的对偶义务（是采纳规则而非拒绝规则：其 host 本就该存在）。`L0#33574`（其他）不在本任务承诺内。

## 1. 规则（全部只增不释）

- **R1 跳空豁免**：`GAP_ADJACENT` 三元组不因缺三折/缺次级别结构而否定线段资格；跳空段视为无内部结构的合法线段（chan99/0046:85,171）。实现上即：`NO_CONTAINING_SEED_WINDOW` 对跳空对象不再阻断 connector 归属评估（它本来就只否定 seed 资格，见 adopt-overlay-spec:195）。
- **R2 连接段采纳**：若跳空对象位于两个同级别已组装结构（host chain 相邻节点）之间，按"次级别以下连接段"并入分解链，`ADOPT_ORPHAN` 事件挂到相邻 host（blog/028:308,746）。归属方向取分解链坐标序，不引入新 host。
- **R3 震荡吸收**：若跳空对象处于某中枢震荡范围内（`high>=ZD && low<=ZG` 对该中枢成立），按结合性并入围绕该中枢的更大级别走势分解（blog/036:34）。R2 与 R3 同时可用时 R2 优先（更局部）。
- **R4 序列起点**：序列/窗口起点缺口视同跳空走势（blog/036:128-131），允许作为首连接段归属其后首个 host。
- **R2' 小转大规则（单独验收）**：单调陡峭三元组若满足"连续同向、无重叠"几何，按小转大机理归类为"级别以下连接段"，同 R2/R3 逻辑评估。原文机理：chan99/0027-第六节 区间套.md:21（小转大＝"本级别一个猛烈的上或下"，小级别突发直接引发本级别转折）、思维导图-零基础学缠论.md:243-244（小转大是区间套/背驰不可解释情形的补充，无一类买卖点——即天然不生成同级结构，故 NO_CONTAINING_SEED_WINDOW 是其预期特征而非异常）、blog/028:754（连续涨停只构成中枢上移）。结果单列，不并入跳空桶指标。

**统一机理假设（#72 验证目标之一）**：跳空桶与单调陡峭桶同为"小转大／级别跃迁事件"的产物，跳空是级别无限低的极限情形（chan99/0046:95 与缺口同课并论；chan99/0017:39、0030:29 给出震荡内分笔背驰引发小转大，对应 R3 路径）。若重放证实两桶消解路径同构，孤儿成因收敛为单一机制，"其他"桶（3 个）再单独归因。

## 2. 账本与安全约束（继承 adopt_v1 全部门禁）

1. 只新增 `ADOPT_ORPHAN`（同 correlation 的 Move supersede reason），禁止任何 host 释放/降级；`CompletedFreeze` 不变。
2. 单调性守恒：`33 = adopted_v2 + residual_v2`，运行时断言；出现释放需求即 `UNSATISFIABLE_WITH_MONOTONICITY` 中止。
3. 可见域：所有 candidate evidence 与 assignment decision `judge_at<=t`（assembler-spec:54-73）。
4. 禁止墓碑标签继续生效；residual_v2 仍为 Unassigned（过渡态），不得伪 Completed。
5. 隔离原型 + property test（≥5 条：单调、不释放、append-only、可见域、守恒）+ 双 release 重放 SHA-256 bit-exact，达标后才谈生产晋级。

## 3. 验收

- 机器输出：33 对象逐一判定（adopted: rule=R1..R4/R2'，或 residual: reason），`L0#37199` 显式判例段。
- 预期（非承诺）：跳空 19 大部分被 R2/R3 采纳；R4 覆盖起点子桶；单调陡峭 11 由 R2' 单独报告；其他 3 归因报告。
- 报告落 `chanlun/review-results/gap-connector-impl-20260713.md`；冲突或需释放的对象一律 escalate，不得自行放宽门禁。

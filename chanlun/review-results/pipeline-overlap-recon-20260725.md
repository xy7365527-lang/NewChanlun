# #243 Pipeline 段结构对账：rust parse_layer 后段 ↔ Lean Origin/Pipeline 前段

- 日期：2026-07-25
- 执行：codex exec（gpt-5.6-sol，read-only，effort=high）
- 验收：主控 session（本文件 §验收裁定 为主控所写，非 codex 产出）
- 票：map #59 子票 #243（自 #221 交接 C 节第 1 条，编排者 2026-07-25 加判）

## 一、核心结论（codex 原判）

**重叠区整体行为不一致**，但绝大多数差异不是实装 bug。

段边界本身就错位：Lean `segmentsOf` 对应的是 rust `parse_layer` **内部**的线段阶段，而不是 parse_layer 之后；parse_layer 之后开始的 classifier 只有中枢/BSP 可与 Lean 前段比较。rust `ParseLayer` 只含 merged/fractals/strokes/segments/tail，centers/moves/BSP 已移交 classifier；Lean `OriginParse` 同时含 segments/centers/bsp/account —— **二者不是同一函数边界**。

差异清单 11 条（D1–D11），归属：

| 归属 | 条目 | 含义 |
|---|---|---|
| B（教义结构差） | D2 线段过滤、D3 包含关系、D6 中枢过滤、D7 中枢扫描、D8 BSP 候选来源、D9 BSP 标签互斥性 | Lean 侧明示的抽象边界：`segmentsOf` 只证终止/确定性骨架，`centersOf` 只建模三段几何，`bspOf` 接收预制候选并压成单标签。不能据此判 rust 有 bug |
| C（契约锚措辞不足） | D1 串接 vs 侧投影、D5 三笔起点未接入 Lean、D10 输出顺序、D11 parser 子集所有权表述 | 缺共同 adapter / 转换 / 有序不变量 / 完整线段构造 |
| **A（codex 判实装 bug）** | D4 相切缺口 | 见下节——**主控订正归属** |

已确认一致点两条：seed 中枢核心 `ZD=max(low)/ZG=min(high)` 且 `ZD=ZG` 单点核心合法；三类点回试边界严格 `>ZG/<ZD` 等号排除。

## 二、验收裁定（主控）

### D4 归属订正：A → 教义待裁（不是实装 bug）

codex 判定的唯一 A 类，事实部分经主控独立核对**属实**：

- rust `is_fractal_and_gap`（`rust/src/theta_v0/parser/feature_seq.rs:60-85`）：向上段 `has_gap = b_l >= a_h`，**含等号** ⟹ 相切算缺口
- Lean `HasGap`（`formal/Origin/SegmentFeatureSeq.lean:102`）：`a.high < b.low ∨ b.high < a.low`，**严格** ⟹ 相切算重合、不算缺口

同一静态谓词在等号处走相反分支，事实无误。

**但归属不是 A。** 理由：

1. rust 侧该函数的文档注释写明「bit-exact 对齐 Python `_is_fractal_and_gap`（:242-266）」—— 它是在复刻既定参考实现，不是随手写错；
2. Lean 侧 `HasGap` 的 docstring 引第 67 课「特征序列两相邻元素间没有重合区间，称为该序列的一个缺口」；
3. 分歧点因此是**「相切（单点接触）算不算重合区间」这个定义问题**：按「有公共点即重合」读 ⟹ Lean 对；按「重合区间须非退化」读 ⟹ rust 对。

按项目纪律「改定义/规则/教义 = grilling 型，编排者拍板」，这条**不能由 agent 自判为 bug 并修**，须回第 67 课原文裁定后再定哪一侧改。已立 grilling 票（见 §四）。

### 其余归属：采纳

B 类六条与 C 类四条的归属，主控核对论据后采纳，无异议。

### 未能判定项（照实保留）

- **整条重叠区的同输入 bit-exact 等价：未能判定。** 缺 `Bar → OriginInput` 规范 adapter、`BspPoint ↔ BspCandidate/Bsp` 转换、候选流有序不变量、Lean 可执行的完整线段构造。
- **D4 的单因素端到端复现：未能做出。** codex 给了谓词级最小复现（六笔序列，边界 `b.low = a.high = 10`），但声明 Lean `segmentsOf` 自身的 D2 朴素切分差异会掩盖 D4，无法隔离 —— 此项照实记录为缺件，未声称已复现。

## 三、D4 谓词级最小复现（codex 产出，未执行）

六根连续交替笔：`s0` Up 0→10 / `s1` Down 10→5 / `s2` Up 5→20 / `s3` Down 20→10 / `s4` Up 10→15 / `s5` Down 15→11。

特征元素 `a=[5,10]`、`b=[10,20]`、`c=[11,15]` 构成向上线段顶分型（20>10 且 20>15）。边界 `b.low = a.high = 10`：

- rust：`10 >= 10` ⟹ 判有缺口
- Lean：`10 < 10` 为假 ⟹ `HasGap = false`，判无重合缺口

## 四、后续票

- **grilling（新立）**：相切是否算「重合区间」—— 第 67 课原文裁定，定哪一侧改
- C 类四条的 adapter/fixture 缺件：暂留 map #59「Not yet specified」，不单独立票（等形式化价值序认账时一并出票）

## 五、原始产物

codex 完整输出：`/tmp/codex-243-pipeline-recon.md`（本次会话产物，未入库）

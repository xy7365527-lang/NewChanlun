# 异质源审计：pending-014 缠论形态学与 PH 范畴论关系

- **日期**：2026-05-29
- **目标**：pending-014（装饰 merge tree S + 遗忘函子 F_PH/F_Chan + 互补性猜想 + 弱/强统一假说）
- **认识论等级**：审计对象为 L0（纯定义+猜想）。审计结论中"定理级否证"为 L0 数学判断。
- **触发**：challenge 模式（pending-014 提出新候选定义，自动触发异质质询）+ 编排者指令"直接调 API 讨论 P1/P2/P3 + 弱/强统一"

## 异质源清单与真实性

| 源 | 通道 | 真实性 | 角色 |
|----|------|--------|------|
| **Codex (OpenAI)** | codex-challenger / OpenAI API | ✅ 真异质源 | diagnose（数学严格性诊断） |
| **GPT-5.1 (OpenAI)** | 直接调 `api.openai.com/v1/chat/completions`，model=gpt-5.1 | ✅ 真异质源 | discuss（弱/强统一建设性讨论） |
| **Gemini (Google)** | gemini-challenger | ⚠️ **API 403 PERMISSION_DENIED，降级由 Claude 执行** | challenge（非异质，有效性低） |

> Gemini 降级声明：GOOGLE_API_KEY 返回 403，gemini-challenger 自动降级为 Claude 执行数学推理质询。
> **本源不构成异质性**（同模型族），其结论作为 Claude 自审记录，不计入异质收敛。
> 原始记录：`.chanlun/review-results/gemini-genealogy-review-20260529-063800.md`

## 三源收敛的核心否定（两个真异质源 + 一个降级源同向）

### 否定 1：弱统一假说为假（定理级，GPT-5.1）

弱统一（缠论 ⊂ PH，即 KP(F_PH) ⊆ KP(F_Chan)）**在一般情形为假**：

- **单位不可换**：persistence = 价格差，缠论最小跨度 = 时间（K 线数）。固定 barcode 下可
  拉伸/压缩临界点之间的时间而不改 birth/death → 存在 S≠S′ 同 barcode 但不同缠论笔。
- **分枝非唯一**（Curry/Elkin-Munch）：不同 merge tree（不同分枝）可共享同一 barcode；缠论
  分段依赖极值的相对嵌套/包含（树形），不只是 birth/death 多重集。
- **序列级规则**：特征序列包含是笔的线性序上的二元关系；barcode 是无典范全序的（多重）集，
  无法从中重构哪些极值对在时间上相邻。

> GPT-5.1 原话："THEOREM (in principle): There exist generic families of 1D signals having
> identical H₀ barcodes but different Chan-compliant stroke/segment decompositions. Therefore
> weak unification fails in general." corr(pers,span)≈0.9~0.99 只说明"此数据上数值相关"，
> 不等于"span 是 barcode 的函数"。

### 否定 2：强统一对标准 S 失败（定理级 + 反例，GPT-5.1）

强统一要求 (F_PH, F_Chan) 单射（KP(F_PH) ∩ KP(F_Chan) = Δ_S）。GPT-5.1 构造反例：

> 取三个等深局部极小 m₁<m₂<m₃，对称 barcode（两条等长 bar + 一条无限 bar）。构造两棵
> 不同的树 S（m₂ 为 m₁,m₃ 的父）与 S′（合并次序翻转），二者 birth/death 时间全同 → 同 barcode。
> 缠论规则只看连续极值间的笔 + 最外 max/min 段 → F_Chan(S)=F_Chan(S′)。但 merge tree 内部
> 分枝不同（S≠S′）。⟹ (F_PH,F_Chan) 非单射。
>
> "This is not an exotic pathology; it arises whenever internal reassociation of merges does
> not change extrema order or visible strokes." ⟹ **强统一在一般情形失败**。

### 否定 3：F_Chan 不是标准 S 的干净遗忘函子（三源同向）

缠论笔/段的若干规则**在 H0 merge tree S 之外**：
- **(a) 缺口**：sublevel filtration 连续化时被抹平（Gemini + GPT）。
- **(b) 特征序列包含**：序列级二元关系，不可化为分量独立的 persistence 过滤 Φ_θ（Gemini + GPT）。
- **(c) 最小跨度**：时间单位 ≠ persistence 价格单位，不可互换（Gemini + GPT + Codex）。

⟹ F_Chan 至多是 **部分遗忘函子**（partial），强统一对标准装饰 merge tree **不完备**——
存在 S 之外的第三投影维度（缠论语法层）。**与 521/523 同构**：521/523 证动力学层（力度）
在 S 外；本审计证缠论**语法层**（缺口/特征序列/时间单位）也部分在 S 外。

### 否定 4：互补性猜想测错了量（Gemini，降级源）

§18.4 反例候选 1 担心 corr(persistence, span)≈0.92 使互补性退化为冗余。但 ker(F_PH) 丢的是
**时间位置**（τ_≺ 偏序），不是 span（持续时间）。span 与位置正交，使互补退化需
corr(persistence, **时间位置**)≈1，而非 corr(pers, span)≈1。**框架测量了错误的量。**

## Codex 的可修复诊断（HIGH/MEDIUM）

- **HIGH-4**：ker∩ker=∅ ⟹ 充分统计量是非法推论。正确等价：kernel pair 交为对角 ⟺ 联合映射
  单射 ⟺ 充分统计量。原文从"核不相交"直觉到充分统计量的跳跃缺这一等价性证明。
- **MEDIUM-5**：corr 数据是 L2 单标的，只能说"此数据集上两轴冗余"，不能推断 L0 层互补性
  成立/失败。§18 头部免责声明有效，但反例候选 1 讨论中存在隐性越界。

## 修正路径（已应用到 §18.8 + pending-014）

1. **ker → kernel pair**：互补性/充分统计量改写为 (F_PH, F_Chan) 单射（Codex CRITICAL-2/HIGH-4）。
2. **F_Chan 降级为部分遗忘函子**：明确缺口/特征序列/最小跨度在标准 S 外（否定 3）。
3. **弱统一结论**：标记为**已否证（定理级）**（否定 1）。
4. **强统一结论**：标记为**标准 S 下失败（定理级 + 反例）**（否定 2）。
5. **测错量修正**：§18.4 反例候选 1 的判别量从 corr(pers,span) 改为 corr(pers, 时间位置)（否定 4）。
6. **存活的开放问题**：**enriched S**（携带时间长度、缺口标记、临界点全序的增强对象）下强统一
   是否成立——GPT-5.1 判定"at best an engineered framework"，未排除，是 pending-014 的存活后继。

## 谱系裁定

- **四分法分类：定理**（否定 1/2 是 GPT-5.1 明示的"THEOREM in principle"，无价值参数）。
- pending-014 settlement = **修正**（标准 S 框架被否证 + 定义缺陷修正），status → 已结算。
- **强化 pending-012**（PH ≠ 缠论引擎内核）：本审计把"不可约维度"从动力学层（521/523）扩展到
  **缠论语法层**（缺口/特征序列/最小跨度）。
- **存活后继**：enriched-S 强统一 = 新研究方向（pending-014 的下游推论，待编排者扫描决定是否
  立新 pending）。

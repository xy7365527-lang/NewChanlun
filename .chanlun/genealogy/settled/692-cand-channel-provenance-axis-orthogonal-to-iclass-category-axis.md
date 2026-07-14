---
id: 692
title: cand_channel 通道轴与 i_class 类别轴正交——bsp_cand_type 坍缩非缺口
status: 已结算
settled_date: "2026-07-04"
settled_by: "genealogist（settle-sweep 增量扫描，018 四分法判定：codex 终局裁决证伪 Task #176 预设的缺口前提，非选择类，无待裁决分叉，零代码改动——定理类，直接结算）"
type: 语法记录
source_task: 176
depends_on: [673, 138, 159]
related: [657]
date: 2026-07-04
---

# 692号：cand_channel 通道轴与 i_class 类别轴正交

## 语法记录（已在运作但未显式化的规则）

`MuClass` 桶键的**买卖点类别信息**与**准入通道 provenance** 是两条正交轴，
分别由两个独立维承载，不可互相替代或合并：

| 轴 | 承载维 | 值域 | PDF 对应 |
|----|--------|------|---------|
| 类别轴 I_γ | `i_class`（第 3 维） | 六 bit 掩码 `class_index()`（0x00-0x3F，共生保留） | 买卖点2.pdf §2「保留整个集合 I_γ」 |
| 通道轴 | `cand_channel`（第 10 维） | NestTrigger 四值（Type1背驰/Type23下沉锚/XZD/PanDiv） | §6 CandType「门通道」 |

## 断裂被证伪的半边

Task #176（codex #157 条目2 后继）预设「bsp_cand_type 坍缩 ⟹ Type2+3 共生点
provenance 丢失」是缺口，须在两种分派变更间二选一。codex 裁决（read-only，
`codex-gate23-20260704.md`）证伪该前提，判 (c) 伪缺口：

1. **类别未坍缩**：`i_class = bits.class_index()`（mu_estimator.rs:200）是六 bit
   不压扁掩码，纯 Type2(0x02)/纯 Type3(0x04)/Type2+3(0x06) 落三个不同 μ̂ 桶。
   坍缩只在 gate dispatcher（`bsp_cand_type`）局部发生，canonical 类别在 z.i_class
   完整保留。
2. **gate 布尔等价**：`cand_delta_type2_completion` 与 `cand_delta_type3_retest`
   谓词体逐字相同（econ_positive.rs:814/833），选 Type2 还是 Type3 对 gate
   pass/reject 零差异。「Type3 几何谓词不评估」= 673号已裁的 dead-gate 设计
   （保护边界归属记录，Cand 层不双门），非缺口。
3. **通道标签正确**：Type2/Type3 经同一 descend_type1_anchor_depth 准入 ⟹ 同一
   通道 Type23SublevelType1 在通道语义下正确，不是 cls_Θ({2,3})=2 类别压缩。

## PDF 一致性

买卖点.pdf「不强行互斥，而是把所有重合情况作为不同语法状态」——i_class 六 bit
即此语法状态（64 种）。买卖点2.pdf §2「更严格的做法是保留整个集合 I_γ……而非
cls_Θ 压缩」——i_class 正是"保留整个集合"，cand_channel 是独立的准入 provenance
轴而非 cls_Θ 压缩。两轴正交 = 严格实现 PDF 精神。

## 翻转条件（有效域边界）

1. 下游消费者只能拿 cand_channel 拿不到 i_class，且须按 Type2/Type3 分治 →
   翻转，但优先修接口传 i_class，不污染通道轴。
2. Type2/Type3 Cand 层 gate 真正分化（谓词体不再逐字同体），共生点语义要求
   per-bit 独立 gate 后取并/交 → 讨论方案 (b)。

## 谱系依据

- 673号：接口级三分拆（Type2/Type3 保护边界归属记录，Cand 层不重门=dead gate）
- 138号：z 扩维 §6，cand_channel 第 10 维「门通道轴」定义
- 159号：A6 透传评估——「候选不携原始信息」半边被 i_class 现状证伪
- 657号（project_iclass_delta_collinearity_perm_degeneracy）：类别维进桶键即毁
  置换检验——方案 (a) 扩 cand_channel 值域按类别分叉 = 共线退化风险

## 结算说明（settle-sweep 增量扫描，2026-07-04）

**018 四分法判定：定理类，直接结算。** 判据：
1. 本条目自身**从未自我声明**需要编排者 `/ritual` 追认（对照 688/689/690/691/693/694
   均在 front matter 或正文显式写「转 settled 待编排者 /ritual 最终辨认」——692 无此
   表述，全文止于「谱系依据」，无待裁决悬项）。
2. 内容性质是**证伪一个被预设存在的缺口**（Task #176 假定坍缩=缺口，codex 判 (c)
   伪缺口），而非提出一个新的域层定义或政策选择——不改变任何人对缠论/形式化链的
   理解，只订正对既有代码行为的误判。
3. **零代码改动**——`i_class`/`cand_channel` 两轴的正交性是现状（已在运作），本条目
   只是把这个已运作但未显式化的规则写清楚（语法记录的定义本身）。
4. 翻转条件①②均为假设性未来情形（下游消费者接口变化/gate 真正分化），当前均不成立，
   不构成待决分叉。

与 679 号先例同构（`.chanlun/genealogy/settled/679-...`：codex 终局裁决 + 非选择类 +
已按施工级规格逐字实装 → 直接结算，不经 /escalate 显式提交）——692 的"实装"就是
现状本身（无需变更），裁决即结算。

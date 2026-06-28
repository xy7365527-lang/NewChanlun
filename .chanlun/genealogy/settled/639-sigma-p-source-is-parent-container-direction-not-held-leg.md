---
id: 639
status: 已结算
title: 操作角色 σ_{p(g)} 来源 = 父容器方向（非活动持仓父腿，非全局主力锚，非静态全走势方向）
date: 2026-06-28
type: 概念修正
authority_chain: [缠师推导「最高级别走势类型.pdf」(编排者发,纯数学推导,一级), spec §7.2 line 367-381, 编排者两轮反驳]
related: [638, 547, 574, coverage-engine-needs-tower-export-bridge, newchanlun-v1-fullwindow-l3-falsified]
supersedes_ruling: "我先前的三方「活动父腿口径」裁定（handoff 2026-06-28-iii line 19/22-24/33）"
---

# 639 σ_{p(g)} 来源修正：父容器方向，非持仓父腿

## 触发

七链操作角色 V(g)∈{Ambient,FollowParent,ShortDiff} 依赖候选 g 的「父方向」σ_{p(g)}。
编排者两轮反驳逼出此问题、否决我两个答案后，发来缠师纯数学推导「最高级别走势类型.pdf」
（内题《推导完全分类》）。**该 PDF 逐字确认 spec §7.2，并否定了我事后给出的「活动父腿」裁定。**

## 矛盾的精确形式（为何是真概念修正而非措辞）

我在 handoff 把 σ_{p(g)} 来源裁定为「活动集 A_t 的持仓父腿 δ（memD p At 门控）：
有活动父腿→σ_p=δ(父腿)，否则→Ambient」。这把**两个正交机制混为一谈**：

| 机制 | spec 出处 | 作用 | 是否依赖持仓 |
|------|----------|------|------------|
| σ 来源 | §7.2 (line 369) | 给候选买卖点**分类角色** A/F/S | **否**（结构对象） |
| 持仓准入 | §13 AncOK (line 659) | 决定 ShortDiff 腿**能否被持有** | 是（父腿在 A_t） |

σ 来自**父容器方向**（结构），与持仓无关；持仓准入是下游独立约束。我用 §13 的持仓判定
去算 §7.2 的 σ 来源 = 概念混淆。

## 三方收敛裁定（PDF + spec + 编排者反驳）

**正确口径**（spec §7.2 = PDF §2/§4/§5/§12，逐字一致）：

```
σ_{p(g)} = 父容器 p(g)∈C_{ℓ_g+1} 的方向 ∈ {-1,0,+1}
         （父容器 = 候选所属走势元素在规范递归塔的 RMove::Compose 父；
           从当前自底向上结构 D_t 自上而下查得 ∈ {确认走势,活动走势,候选中枢,胚元}）
V(g) = Ambient(σ_p=0,父=胚元∂) / FollowParent(δ_g=σ_p) / ShortDiff(δ_g=-σ_p)
```

PDF 关键论断（§2/§3/§5）：角色 R **不是裸元素单元函数 R(g)**，而是相对当前可见操作容器
c 的关系 R_t(γ)；σ_t(c) 是容器自身方向。「方向是元素属性；包含关系是结构属性；
短差/顺势/ambient 是相对当前操作容器的关系属性」（§12 框注）。

**四个候选答案的裁定**：

| 答案 | 内容 | 裁定 | 否决理由 |
|------|------|------|---------|
| 1 静态全走势 rmove_side | 父走势完成态几何方向 | 执行层❌ | 完成态需未来数据（违因果）。**但** L0 静态分类合法（PDF §3「不可回溯只在订单层」） |
| 2 活动持仓父腿 (我的裁定) | A_t 持仓父腿 δ（memD 门控） | ❌ | 编排者反驳「实盘不一定在最高级别买卖点建仓」=不一定持有父。把 §13 持仓准入误当 σ 来源 |
| 3 全局主力级别绝对锚 (547暂定) | 最高活跃走势级别绝对参照 | ❌ | PDF：容器是**局部**（每证书的父容器），非全局锚。**547 删除确认** |
| ★正确 | 父容器方向，从 per-bar 因果 D_t 查 | ✅ | spec §7.2 = PDF。活动走势的**当前**方向因果合法（只用≤t），不需持仓 |

**两轮反驳被正确口径同时消解**：
- 「操作=买卖点不能用方向」：方向非操作触发器；证书终端 g_e 仍是买卖点（Conf=1）触发操作，
  σ(c) 只**分类**该买卖点角色（A/F/S），不替代买卖点。
- 「不一定持有父」：父容器是**结构对象**（D_t 查得），不需持仓。无向容器（胚元 σ=0）→ Ambient
  是去根化边界，非塌陷。

## 结果包六要素

1. **结论**：σ_{p(g)} = 父容器方向（结构，per-bar 因果 D_t 查得），非持仓父腿/全局锚/静态全走势。
   Lean 落地 `Origin/ParentDirContainer.lean`（取代删除的 `ParentDirActive.lean`）：
   `parentDirOfContainer par g = match par g with | none => none | some p => some p.dir`，
   4 定理全证（none_iff_germ / classifyV_ambient_iff_germ / of_directional_container /
   struct_only），#print axioms 仅 propext（struct_only 零公理）。`lake env lean` EXIT 0。
2. **定义依据**：spec §7.2 line 367-381（σ_{p(g)}=父容器方向，V 三分依赖 σ∈{-1,0,+1}含0）；
   PDF §2（R_t(g;c) 非 R(g)）/§4（𝒞^op={确认走势,活动走势,候选中枢,胚元}，TD 只用 D_t）/
   §5（R_t(γ) 定义在区间套证书）/§12（方向=元素属性，角色=相对容器关系）。
   输入特征：候选 g 带级别 ℓ_g + 方向 δ_g；其父容器 p(g) 是塔里 ℓ_g+1 级 Compose 父，有自身方向。
3. **边界条件**（结论翻转）：① 若 par g=none（父=胚元∂）→ σ_p=0→Ambient（非 bug，去根化）；
   ② 若 spec/原文把 σ 来源改回持仓判定 → 退回错口径；③ 若父容器方向取自完成态全走势
   （含 >t 数据）→ 违因果（执行层禁）；④ 若 547 主力锚被裁为客观定理而非局部容器 → 答案3 复活。
4. **下游推论**：(a) iii-bridge 的 `interp::parent_dir_from_active`（持仓父腿口径）是**错口径**，
   须替换为父容器方向——这需要 **per-bar 因果递归塔**（当前 slice_classification_at 只切 bsp、
   moves 空，无父容器方向可读）= 真工程缺口（非定义冲突）；(b) ShortDiff 持仓准入仍走 §13 AncOK
   （未持父则剔除），与 σ 来源分离；(c) 错口径会把「父有向但未持仓的逆向次级点」误判 Ambient
   →建 naked 独立逆势仓（garbage trades），正确口径判 ShortDiff→AncOK 未持父时不开仓。
5. **谱系引用**：638（hostOf 附着，本条结算其遗留的 (B) 主力锚开放子问题）；547（级别错配，
   主力锚删除确认）；coverage-engine-needs-tower-export-bridge（多级角色/嵌套对冲=#5 alpha 来源，
   依赖正确 σ）；newchanlun-v1-fullwindow-l3-falsified（七链产 trades ≠ alpha，231 铁律不变）。
6. **影响声明**：删 `formal/Origin/ParentDirActive.lean`（错口径），新建 `ParentDirContainer.lean`
   （正确口径，已编译验证）。标记 iii-bridge 未提交的 `interp.rs::parent_dir_from_active` +
   `assemble_gamma_active_parent` + 其测试（gamma_active_parent_*）为错口径待替换。结算 638 (B)。
   不改 spec §7.2（本就正确，PDF 确认）。supersede handoff「活动父腿」裁定。

## 为何 settled（概念已结算，与工程缺口分离）

- σ_p 口径由 PDF（编排者发，一级权威）+ spec §7.2 双源逐字收敛，编排者两轮反驳均被消解 → 概念无悬置。
- 剩余的是**工程缺口**（per-bar 因果塔实装），非定义冲突——不阻塞 settle。
- 547 主力锚删除、638 (B) 子问题，均被本条一并裁定。

## 边界条件（本记录失效条件）

- 若缠师后续原文/编排者裁定 σ 来源确含持仓语义 → 本条重开。
- 若发现 spec §7.2 与 PDF 在某细节分歧（目前逐字一致）→ 重审。

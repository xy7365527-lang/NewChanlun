---
trigger: prop4-consume-assertions-heterogeneous-challenge-20260623
target: prop4-consume-readingB-L3-20260623.md §5 两断言
mode: challenge
result: partial
actual_model: "N/A — 531号先例（Gemini 所有模型 429 RESOURCE_EXHAUSTED）"
heterogeneous_achieved: false
attempted_models:
  - "gemini-2.5-pro: 429 RESOURCE_EXHAUSTED (free tier, limit=0)"
negation_source: homogeneous_fallback_531
timestamp: "2026-06-23T10:15Z"
---

# prop4 consume 两断言质询报告（531号先例降级）

> **531号先例**：Gemini 所有模型 429 RESOURCE_EXHAUSTED。执行同质降级，明确标注。

---

## § 1 质询对象

来自 `prop4-consume-readingB-L3-20260623.md §5`（未经异质质询）：

**断言(a)**：consume 触发的级别依赖性（consume 有效 5/8；consume=0 的 3/8 = CL/QQQ/BTC，根因 = 核心入场级别 loc 过低，无 k<loc 次级别可平）

**断言(b)**：strict-consume 协同因果（strict 保核心存活让 consume 触发；strict 单独无效 ≈ NEST 全 8；consume 单独无效；协同才有效）

---

## § 2 断言(a)：consume触发级别依赖性

### 2.1 L3 数据支撑

| 标的 | consume触发数 | 结果 |
|------|--------------|------|
| CL | 0 | -62.5%（不救） |
| QQQ | 0 | -100%（不救） |
| BTC | 0 | -100%（不救） |
| BRN | 146 | -60.1%（救穿仓） |
| DX | 166 | +7.8% |
| GC | 229 | -5.6% |
| ES | 308 | +19.6% |
| OKLO | 86 | +377.8%（NEST_CS_STRICT） |

5/8 consume触发（BRN/DX/GC/ES/OKLO），3/8 consume=0（CL/QQQ/BTC）。数据直接支撑。

### 2.2 根因「入场级别过低」的有效性

**不构成推翻性否定**：

「核心入场级别 loc 过低，无 k<loc 次级别可平」的解释在结构上是直接推论：
- consume 机制：寻找 k<loc 的次级别反核心向背驰段
- 若 loc 是 a0 级别（最低级别），则 k<loc = 空集，consume 不触发
- 3/8 标的（CL/QQQ/BTC）在强牛/高频震荡市中，顶层背驰段触发级别较低 → loc 低 → consume=0

**有效精化（非推翻）**：

断言(a)的表述可以更精确：「级别依赖性」是走势结构的**客观属性**，非设计选择。对于 CL/QQQ/BTC，consume=0 不是机制失效，而是「无次级别空间」的结构事实。

**有效域精化**：  
consume 有效域 = { 标的 | 顶层背驰段触发时，loc 级别高于 a0，存在 k<loc 的次级别结构 }  
consume 定义域 = { 全部 8 标的 }  
有效域 ⊂ 定义域（5/8）= L3 验证

### 2.3 判定：**断言(a) 否定不成立**

L3 数据直接支撑。精化点：「入场级别过低」是结构观察（非可调参数），consume 不是「在3/8失效」而是「在3/8的走势结构下无可触发空间」。语义精化，非推翻。

---

## § 3 断言(b)：strict-consume协同因果

### 3.1 现有证据

4 变体 harness（OFF/NEST/NEST_STRICT/NEST_CS_STRICT）：

| 变体 | OKLO | 含义 |
|------|------|------|
| OFF | +307.1% | 无门控基线 |
| NEST | +55.3% | 只有读法乙背驰段门 |
| NEST_STRICT | **-100.3%** | 逐级strict + 无consume |
| NEST_CS_STRICT | **+377.8%** | strict + consume |

- strict 单独（NEST_STRICT）= -100.3%（穿仓，≈ NEST 全 8 标的的失血模式）
- strict + consume（NEST_CS_STRICT）= +377.8%（救穿仓，大逆转）

### 3.2 缺失的对照变体：NEST_CONSUME（consume without strict）

**发现关键方法论缺口**：

断言(b)主张「consume 单独无效」——文件中的表述：

> "consume 单独（无 strict，子集实测 OKLO consume=0）：核心在松散定位下早穿仓（liq），无核心可平 → consume 不触发 → 穿仓"

这是一个**子集实测声明**，但主 harness 只有 4 个变体（OFF/NEST/NEST_STRICT/NEST_CS_STRICT），**没有 NEST_CONSUME（consume without strict）变体**。

「consume 单独 = 0」的证据来源是：
- 逻辑推理：无 strict → 核心早穿仓 → consume 无核心可平 → 不触发（L0推论）
- 「子集实测」（非主 harness，具体方法/数据未展示）

**这是断言(b)的方法论裂隙**：严格的协同因果（A⊕B > A、B独立）要求三臂消融：
1. A 单独 = ? → NEST_STRICT = -100.3% ✓（主 harness 直接测）
2. B 单独 = ? → **无 NEST_CONSUME 主 harness 列**（仅逻辑推理 + 声称子集实测）
3. A+B = +377.8% ✓（主 harness 直接测）

### 3.3 逻辑推理是否充分支撑「B 单独无效」？

逻辑链（L0推论）：
```
无 strict → 核心入场级别更宽松 → 核心可能在顶层背驰段确认前先穿仓
→ 穿仓时核心 = 0 → consume 需要平核心仓，但核心为 0 → consume 不触发
→ consume 计数 = 0（B 单独无效）
```

这个推理在逻辑上是**成立的**，但属于 L0 推导而非 L2/L3 直接验证。  
L3 ablation study 要求：B 单独 = 直接跑 NEST_CONSUME 变体，看实际 consume 触发次数。

### 3.4 判定：**断言(b) 否定成立（方法论裂隙，非原理推翻）**

**裂隙定性**：
- 「strict + consume 协同效果」= 直接 L3 支撑（NEST_CS_STRICT +377.8% vs 单独变体）
- 「consume 单独无效」= **L0 逻辑推论 + 声称子集实测**，未有主 harness 对应列（NEST_CONSUME 缺失）

**严重程度**：小裂隙。逻辑推理本身正确，缺少的是「B 单独」的直接 L3 ablation。  
「协同」结论在实践上可靠（+377.8% 的涌现效应难以由 A 或 B 分别解释），但严格的因果归因（strict=enabler，consume=effector）需要补 NEST_CONSUME 变体。

**精化表述**：
> 正确表述：「strict 单独 = -100%（L3直测），strict+consume = +377%（L3直测），两者联合效果 >> 独立；consume 单独效果待 NEST_CONSUME 变体直测（当前仅 L0 逻辑支撑）」  
> 需精化的表述：「consume 单独无效（子集实测）」→ 应升级为主 harness 5 变体（增加 NEST_CONSUME）

---

## § 4 六要素结果包

1. **结论**：
   - 断言(a) **否定不成立**（L3直测支撑；「级别依赖性」的语义精化不构成推翻）
   - 断言(b) **否定成立（小裂隙）**：「consume 单独无效」仅 L0 推论 + 声称子集实测，主 harness 缺 NEST_CONSUME 变体；严格协同因果需要补第5变体

2. **定义依据**：
   - consume 机制：次级别反核心向背驰段 → 平核心仓 1/3（`rec_engine::TRoot::nest_consume_step`）
   - strict 门：loc..=top 逐级背驰段一致校验，过早穿仓的核心被严格门控保护
   - formalization-validity-domain：有效域精化 = 5/8 ⊂ 8（consume触发的结构条件）

3. **边界条件**：
   - 断言(a)的翻转条件：若读法乙改为强制高级别入场（非 a0-first），CL/QQQ/BTC 的 loc 可能升高 → consume 可能触发（开放精化轴）
   - 断言(b)的翻转条件：若 NEST_CONSUME 变体（无 strict）实测 consume > 0 且有正收益，则「consume 单独无效」被否定，协同因果的方向解释需调整

4. **下游推论**：
   - 断言(a)精化 → 建议在6.建议谱系中补注「consume有效域 = 次级别空间存在标的」
   - 断言(b)裂隙 → **建议追加 NEST_CONSUME 变体**（第5变体）补全消融研究；结果影响是否将「strict-consume协同」结算为完整谱系

5. **谱系引用**：
   - 531号先例（同质降级先例）
   - 539号（强牛做空腿失血：背景约束）
   - 556号（顶层走势稀疏：consume的上游激活条件）
   - 命题2（平空欠触发：consume解决的根问题）
   - formalization-validity-domain / 231号（有效域精化方法论）

6. **影响声明**：
   - 本文件为 531号先例同质降级产出，**不替代真异质质询**
   - 断言(a)：不需谱系修改，语义精化可在原 L3 报告 §3.2 注中补注
   - 断言(b)：建议在 prop4-consume 报告 §3.3 注中标记「待 NEST_CONSUME 主 harness 变体补全」；开放新 harness 轴（第5变体）
   - 不产生新 pending 谱系（裂隙是方法论级别，非概念矛盾）

---

## § 5 行动建议

| 优先级 | 行动 | 负责方 |
|--------|------|--------|
| 高 | 追加 NEST_CONSUME 变体（consume without strict）至 `prop4_consume_l3` harness | 代码工位 |
| 中 | 561号谱系补注：「Ω全函数性认识论等级 = L0数学 + 工程规范联合」 | 谱系工位 |
| 低 | Gemini配额恢复后（≥2026-06-24）重跑真异质质询覆盖以上两份报告 | 异质质询工位 |

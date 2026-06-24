---
trigger: D-hetero异质审计（orbit9子DAG D工位L3否定结果）
target: orbit9-D-offguard-l3-20260624
mode: challenge
result: pending-real-heterogeneous
timestamp: 2026-06-24
model_attempted: gpt-5.5-pro → gpt-5.5 fallback
model_error: 429 insufficient_quota
---

# D-hetero 异质审计报告（降级）

## 状态

**OpenAI GPT-5.5 API 不可达**：gpt-5.5-pro 尝试后降级到 gpt-5.5，仍返回：
```
429 insufficient_quota — You exceeded your current quota
```

异质源全死（与 593 号已知状态一致）。**审计未发生，未假装审计发生。**

## 降级声明

- `status: pending-real-heterogeneous`
- 真实异质质询需待 API 配额恢复
- **不阻塞 #54 D-cryst**（按任务约束降级不阻塞下游）

## 四个预构建质询点（供未来异质审计参考）

以下质询点已构建，上下文文件已写入 `/tmp/challenge-ctx-d-hetero.md`：

**质询点1：net-up 偏差**
8 标的全是 net-up regime（BTC+1380%BH, OKLO+307%BH 等），H⁰ 拦 flip = 牛市踏空。
D 从「6/8 劣化」是否过度推广到全域声明 H⁰ 无效？真 bear 段有效域未验证。

**质询点2：触达率≠有效应**
h0_flip_blocked 大量触达（CL 55次, ES 64次, BTC 41次）= H⁰ 生效 ≠ H⁰ 有效。
拦截可为 (a) 有效拦截 (b) 无效拦截。大量触达+收益劣化 = 大量 (b)。
D 的「绝非死代码」论断准确，但「触达率高=有有效应」的推断需要区分 (a)/(b)。

**质询点3：B/C 死代码归因是否排除交互项**
B/C 的 bit-exact 测试是在「仅 B/C ON」还是「A+B+C ON」下验证的？
A ON 后 flip 轨迹变化，B 的 orbit9_sub_trend_done 占位 return true 在 A ON 环境下是否仍然 no-op？交互项未显式验证。

**质询点4：可合主声明有效域**
「OFF bit-exact ⇒ 可合主」的有效域是「仓位 bit-exact」还是「全行为 bit-exact」？
新字段（enable_h0_skeleton, n_h0_flip_blocked, pending_exit_trigger 等）在 OFF 模式下可能仍被日志/报告/诊断路径读取，是否改变非仓位行为？

## 本工位的独立分析（同质质询简化版）

无法调用异质源，本工位执行同质简化质询（三步）：

### 步骤1：定义回溯

- D 报告引用 formalization-validity-domain 231号：有效域≠定义域——这是正确应用。
- D 未引用 L3 认识论等级标注规则（formalization-validity-domain 规则要求每次形式化操作必须标注等级）。
- D 的 L3 标注（8标的×1min×真实数据）认识论等级 = L3（多标的真实数据交叉验证）。**引用正确，等级标注存在于表头注释中，定义依据完整。**

### 步骤2：反例构造

**质询点1反例**：BRN 和 GC 是 H⁰ ON > OFF 的两个例外（+9.6pp 和 +11.2pp）。
- BRN：BH+87.4%（弱牛/震荡），H⁰ ON 改善+9.6pp
- GC：BH+257.3%（强牛），H⁰ ON 改善+11.2pp

GC 是强牛却 H⁰ ON 改善——这与「强牛下 H⁰=踏空」的假设矛盾。这 2 个反例说明：
- H⁰ 的有效域不是简单的「bear=有效，bull=无效」二分
- 有效域边界更复杂，可能依赖 flip 频率、确认级别、GC 特有的震荡特征
- **D 的「有效域待真 bear 数据 L3 缩小」的结论成立，但缩小方向不一定是 bear**

**D 的边界条件声明（有效域 ⊂ 当前 8 标的 1min，真 bear 段 Δ 可能翻正）本质上是对 GC/BRN 反例的间接承认，结论上不过度。**

### 步骤3：推论检验

「ON 差异全由 A 贡献」的归因：
- B bit-exact（dispatch OFF）：已验证（n_adds=0 全标的，orbit9_sub_trend_done 占位）
- C bit-exact（nest OFF）：已验证（enable_orbit9_nest 无消费路径）
- **但「A+B+C ON」时 B 是否绝对死代码需要交互项验证**——若 A 改变 flip 路径使得某个 `!orbit9_sub_trend_done(j)` 从 false 变为理论上 true（即使占位 return true 导致实际还是 false），归因仍成立，但需明确这是「double-false」（占位 false AND A-altered-state）

**结论：归因的逻辑链无错，但「双 false」的显式验证是 B 完整性的隐含缺口。**

## 判定

**本工位（同质简化质询）：D 的否定性结论基本成立，有两处标注精度缺口：**

1. **GC/BRN 反例未完整分析**：H⁰ 有效域边界不是简单 bear/bull 二分，BRN 弱牛+H⁰正/GC 强牛+H⁰正 是反例——D 未解释这两个偏正例的机制（仅说「待 bear 数据缩小」）
2. **B/C 死代码的交互项验证未显式给出**：「A+B+C ON」态下 A 改变 flip 轨迹后 B 是否仍零激活，报告未列数据支撑（仅逻辑推导）

**以上两点不翻转 D 的整体结论，只是边界精度缺口，不需要 /escalate。**

真实异质否定（GPT-5.5）：**审计未发生，不可声明。**

## 影响声明

- 本文件落盘：`.chanlun/review-results/orbit9-D-hetero-20260624.md`
- 不阻塞 #54 D-cryst
- 异质源不可达状态与 593 号一致，不新建谱系（593 已记录）

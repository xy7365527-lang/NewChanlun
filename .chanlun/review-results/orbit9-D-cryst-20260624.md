# orbit9 D-cryst 结晶报告（2026-06-24）

orbit9 子DAG 16 节点终局结晶。D-cryst = 最后节点，负责凝固子DAG 终局结论并完成父汇总。

---

## 结果包六要素

### 1. 结论

**orbit9 子DAG 终局结论：整合成功，可合主（OFF opt-in）；H⁰ payoff 有效域 net-up 8 标的净负（L3 否定性）；B/C 整合态死代码；开放轴4条。**

#### 操作语义完全分类三组件状态

| 组件 | 内容 | 整合状态 | 审查状态 | 异质审计 |
|------|------|---------|---------|---------|
| A（H⁰ flip 门） | τ 对称化：反向 flip 仅在 φ=0 走势完成点合法 | 已整合，生效 | 通过（D-review PASS） | 降级 pending（OpenAI 429）|
| B（dispatch add 轨道） | O3 add 短差接通 | 整合态死代码（占位 return true） | 通过（B-hetero PASS） | 真 codex 已完成 |
| C（nest 定位算子） | locate_nest 纯函数 | 整合态死代码（无消费路径） | 通过（C 自声明） | 真 gemini 已完成 |
| D（整合+OFF守卫+L3） | 冲突解析+bit-exact+交叉验证 | 整合成功（commit 4c0afeae11） | 通过（D-review 5项全 PASS） | 降级 pending（同质简化完成）|

**整合 commit**：`4c0afeae11`（facea `54279a503e` → A `167bfda7bb` → B `685bed8290` → C/D `4c0afeae11`）。

**OFF bit-exact 守卫**：137 passed / 0 failed（L1 PASS）——三 env 缺省 ⇒ 逐位等 facea 基座。结构层完备。

#### L3 否定性结论

**ON 在 net-up 8 标的 6/8 劣化（认识论 L3）**，全部差异由 A（H⁰）单独贡献：

| 标的 | OFF | ON | Δ | BH |
|------|-----|----|---|----|
| CL   | +20.3% | −28.1% | **−48.4pp** | +28.2% |
| BRN  | −26.0% | −16.4% | **+9.6pp** | +87.4% |
| DX   | −0.7% | −4.0% | −3.3pp | +4.1% |
| GC   | −22.7% | −11.5% | **+11.2pp** | +257.3% |
| ES   | −2.3% | −2.9% | −0.6pp | +594.3% |
| QQQ  | −8.6% | −21.1% | −12.5pp | +174.6% |
| BTC  | +26.0% | −28.3% | **−54.3pp** | +1380.4% |
| OKLO | +55.3% | −29.6% | **−84.9pp** | +307.1% |

H⁰ 在生产数据上大量触达（CL:55, ES:64, BTC:41, OKLO:20 次 flip 被拦），把背驰段提前翻向全部拦成 no-op（flip 数 OFF→ON 由非零→0），被拦 flip 多数有益 ⇒ 踏空惩罚 > 假翻向惩罚。

H⁰ 绝非死代码（A'' 仅覆盖 cc 级 type1，H⁰ 覆盖所有级别）——但 payoff 有效域在当前 net-up 8 标的退化。

**B/C = 整合态死代码**：B 因 `orbit9_sub_trend_done` 占位 `return true` 使 add 永不激活（n_adds=0 全标的）；C 因 `enable_orbit9_nest` 无引擎消费路径使 `locate_nest` 完全无效。

#### D-hetero 两缺口（需纳入有效域声明）

1. **GC/BRN 反例**（重要）：GC 强牛（+257% BH）H⁰ ON 改善 +11.2pp，BRN 弱牛（+87% BH）改善 +9.6pp。**矛盾"强牛=踏空"简单假设**——H⁰ 有效域不是 bear/bull 二分，真实判别量未知（候选：flip 频率、确认级别、震荡特征）。
2. **B/C「双 false」交互项**：A ON 改变 flip 轨迹后，`!orbit9_sub_trend_done(j)` 仍因占位 `return true` 保持 false——逻辑成立但缺显式数据支撑（n_adds=0 是间接证据）。

---

### 2. 定义依据

- **操作语义完全分类（exhaustive #39 §一A T1-3）**：9 τ 对称轨道 H⁰⊕H¹ = 587 候选A 的 L0 必然性定义。本 D-cryst 凝固其结构实装完备性（L1），同时测量 payoff 有效域（L3）。
- **OFF bit-exact 定义**：三 env（`T_ORBIT9_H0`/`T_ORBIT9_DISPATCH`/`T_ORBIT9_NEST`）缺省 ⇒ `EngineConfig::from_env` 三字段恒 false（rec_engine.rs:170/179/186 注释声明此不变式）。
- **有效域规则（231号/formalization-validity-domain）**：形式化操作有效域可严格小于定义域。L3 否定性结果缩小有效域边界，不等于闭合有效域。
- **574 确认滞后**：H⁰ confirmed 门 = 把 type1 走势完成作为 flip 合法条件，本质是 574 滞后 floor 施加到 flip，提前翻向被拦 = 踏空（与 574 lag 同构）。

---

### 3. 边界条件

**整合可合主结论翻转**：若 OFF bit-exact 守卫补测失败（当前 137/0 PASS ⇒ 不翻转）。

**H⁰ payoff 有效域净负结论翻转**：
- 真 bear 段数据（如 BTC 2022 年熊市）：提前翻向被拦可能是避灾，Δ 可能翻正
- GC/BRN 反例机制分析后：若判别量是 flip 频率而非 bull/bear 标签，有效域边界将重新定义
- 更高确认级别（非 1min）：级别提升后 H⁰ 触达率可能下降，payoff 特征可能改变

**B/C 死代码结论翻转**：若 `orbit9_sub_trend_done` body 替换为 C 的 `locate_nest`/`is_sub_trend_done` ⇒ add 激活，B 从死代码变为活路径。

---

### 4. 下游推论

1. **可合主（OFF opt-in）**：OFF bit-exact L1 PASS ⇒ A/B/C/D 代码可安全进 main，不改变 facea 生产行为。三 env 是实验门控。
2. **H⁰ 默认必须保持 OFF**：6/8 劣化（L3 否定性）⇒ H⁰ ON 不进 production 默认配置，等待真 bear 段 L3 和 GC/BRN 机制分析。
3. **B/C 接通工作明确**：B 的 add 轨道需 C 的 `locate_nest` 替换 `orbit9_sub_trend_done` body（F-wt 中未完成）。接通后 n_adds>0，dispatch 产生真实效应，届时需重跑 L3。
4. **H⁰ = 574 lag 施加到 flip**：这是操作语义完全分类的有效域测量——τ 对称结构 L0 成立，但 payoff 在 net-up 市场退化，与 574 lag 结论同构（574 号：确认滞后不可消除，只能选择在哪个层施加）。

---

### 5. 谱系引用

- **587**（H⁰ 候选A τ 手性对称化）：本 L3 是 587 的 payoff 有效域测量。587 的 L0 τ 对称必然性成立，payoff 有效域在 net-up 8 标的退化为净负（231号：有效域 ⊊ 定义域）。GC/BRN 反例补充：有效域边界不是 bear/bull 二分，机制待 L3 缩小。
- **574**（确认滞后 floor）：H⁰ confirmed 门 = 574 滞后 floor 施加到 flip。提前翻向被拦 = 踏空，与 574 lag 同构。本 L3 是 574 在 flip 层的实证延续。
- **539**（做空腿=亏损唯一来源）：H⁰ 拦 flip 后 short_pnl 全标的仍负（CL −7486、OKLO −53254），flip→0 未解 539 做空腿失血。
- **546**（A'' 死锁解锁）：A'' 与 H⁰ 作用域分离（A'' 限 cc 级，H⁰ 覆盖所有级别），非互斥，高触达量证实独立性。
- **231**（形式化有效域）：L3 否定性结果是 231 号规则的标准应用——定义域 ≠ 有效域，有效域缩小是信息增量。

**谱系建议（强）**：建议 genealogist 评估以下结晶——「操作语义完全分类：结构层完备（bit-exact）∧ payoff 有效性在 net-up 净负」是 587/231/574 链的实证延续，具体命题：

> H⁰（τ 对称 flip 门）= 574 确认滞后 floor 施加到 flip 层。其 L0 τ 对称必然性成立，但 payoff 有效域在 net-up 主导的 1min 历史数据上退化（6/8 劣化）。GC/BRN 偏正例表明有效域边界不是 bear/bull 二分——真实判别量待 L3 缩小。

---

### 6. 影响声明

**本 cryst 工位只写文件，不改代码，不 commit：**
- 写入：`.chanlun/review-results/orbit9-D-cryst-20260624.md`（本文件）
- 不改动：引擎代码（A/B/C/D 代码在整合 worktree `orbit9-integration-wt` 已冻结在 `4c0afeae11`）
- 不改动：谱系文件（建议由 genealogist 评估，不自动执行）
- 不 commit（Lead 统一）

---

## 四条开放轴（供编排者，不自动执行）

| 轴 | 描述 | 优先级 |
|----|------|--------|
| (a) 接通 B/C | `orbit9_sub_trend_done` body 替换为 `locate_nest`/`is_sub_trend_done` ⇒ B add 激活 ⇒ 重跑 L3 | 下一轮 orbit9 工位 |
| (b) 真 bear 段 L3 | BTC 2022 熊市数据验证 H⁰ 有效域——提前翻向被拦是否在 bear 中变为有益 | H⁰ 有效域缩小 |
| (c) GC/BRN 机制分析 | H⁰ 有效域真实判别量（flip 频率/确认级别/震荡特征）的 L3 测量 | H⁰ 有效域缩小 |
| (d) off() 显式置 enable_h0_skeleton=false | D-review LOW：补显式赋值，与 enable_orbit9_dispatch/nest 风格一致（合主前） | 低优先级技术债 |

---

## 认识论等级汇总

| 产出层 | 等级 | 说明 |
|--------|------|------|
| τ 对称分类（L0 必然性） | L0 | 9 轨道 H⁰⊕H¹ = 代数定义，无数据依赖 |
| OFF bit-exact 守卫 | L1 | 137 测试合成验证，确认整合不破坏基座 |
| ON 收益对比（8 标的） | L3 | 真实数据交叉验证，否定性结果缩小有效域 |

---

## session_append 标注

```
session: prop4-nest-readingB-20260623
event: orbit9-D-cryst
timestamp: 2026-06-24
content: orbit9子DAG 16节点全完成。整合commit 4c0afeae11（OFF bit-exact 137/0 PASS）可合主。L3否定性：H⁰ ON net-up 8标的6/8劣化，全差异由A贡献（B/C死代码）。H⁰=574 lag施加到flip层。GC/BRN反例表明有效域非bear/bull二分。开放轴4条：(a)接通B/C (b)bear段L3 (c)GC/BRN机制 (d)off()显式置false。建议genealogist结晶587/574/231链延续。
```

# Gemini 异质质询：蜂群持久化失败的结构性根因分析

## 问题A：138号修复为什么不够？

### 判断

138号修复的格式A/B/C是正确的形式转化（否定性禁令→结构化模板），但**格式B的判断条件存在结构性盲点**。

Lead选择格式B（"无待做行动"）的依据来源链是：

```
ceremony_scan.py 输出 → workstations 列表 → 列表为空 → "无待做行动" → 格式B
```

这条链有一个根本缺口：**ceremony_scan.py 不扫描刚结算谱系的下游推论**。

具体来看 ceremony_scan.py 的扫描来源：
1. `roadmap.yaml`（最高优先级）— 139号的下游推论不在 roadmap 里
2. `session 遗留项`（格式A表格行或格式B编号列表）— 139号刚结算，session 还没写入其下游推论
3. `pending 谱系` — 139号是已结算，不在 pending 里
4. `discover_business_tasks` fallback（测试失败）— 不涉及谱系下游推论
5. `pattern-buffer candidates` — 不涉及谱系下游推论
6. `downstream_audit` — 虽然 ceremony_scan.py 第292-303行调用了 downstream_audit，但其结果只放入 `result["downstream_actions"]` 字段，**不生成 workstation 条目**

所以问题不是格式模板不够强，而是**格式B的判断依赖的数据源不完整**。ceremony_scan.py 能检测到下游行动的存在（通过 downstream_audit），但不把未解决的下游行动转化为工位。Lead 看到 `workstations: []`，合法地选择了格式B。

### 依据

- `ceremony_scan.py` 第291-303行：downstream_audit 结果只写入统计字段，不转化为 workstation
- `ceremony_sequence.cold_start.nodes.derive-work.scan_sources` 第541行：声明了 "谱系下游行动未执行（spec-execution gap）" 作为扫描来源，但 ceremony_scan.py 的代码没有实现这个声明
- 这本身是一个 spec-execution gap（134号同构模式）

### 边界条件

如果 downstream_audit 的未解决项全部为"长期/背景噪音"类别（不需要立即行动），那么不转化为工位是正确的。只有 status=unresolved 的下游行动应该产生工位。

---

## 问题B：蜂群持久化的结构条件

### 判断

蜂群要持久化运转，需要一个**闭合的行动发现回路**，而不是"再加一条规则"。当前回路断裂在两个位置：

**断裂点1：谱系结算→行动入队**

谱系结算后，其下游推论没有自动进入 ceremony_scan 的扫描范围。需要以下修复之一：

- **方案a**：修改 ceremony_scan.py，将 downstream_audit 的 unresolved 项转化为 workstation 条目（与 pending 谱系转工位同构）
- **方案b**：谱系结算时自动将下游推论写入 roadmap.yaml（status: active）

方案a更自然——downstream_audit 已经在做扫描工作，只是没把结果转化为工位。方案b需要改变谱系写入流程，入侵性更大。

**断裂点2：上下文边界→行动传递**

session 文件记录"做了什么"但不记录"下一步应该做什么"。当 context window 耗尽、新 session 启动时，上一个 session 的未完成下游行动只能通过 ceremony_scan 重新发现——但如断裂点1所述，ceremony_scan 发现不了它们。

这不需要引入新的"行动队列"机制。修复断裂点1后，每次 ceremony 启动时 downstream_audit 会自动重新扫描全部已结算谱系的下游推论，未解决的会生成工位。session 文件只是可选的加速路径，不是必需的持久化载体。

### 依据

- dispatch-dag.yaml ceremony_sequence 的 scan_sources 已经声明了 "谱系下游行动未执行（spec-execution gap）" — 声明存在，实现缺失
- downstream_audit.py 已实现完整的扫描逻辑（extract_downstream_actions + check_action_resolved）— 能力存在，集成缺失
- ceremony_scan.py 第292行已调用 downstream_audit — 调用存在，转化缺失

三个"存在但未闭合"构成一条清晰的修复路径：**让 ceremony_scan 把 downstream_audit 的 unresolved 项转化为 workstation**。

### 边界条件

- 如果 downstream_audit 本身漏检（例如：下游推论措辞不匹配正则），修复 ceremony_scan 不够——还需要改进 downstream_audit 的提取逻辑
- 随着已结算谱系数量增长（当前约147条），downstream_audit 的全量扫描可能变慢——但这是性能问题，不是结构问题

---

## 问题C：context window 限制下的持久化

### 判断

**不需要引入新的"行动队列"持久化机制。** 原因：

1. 下游推论已经持久化在 `.chanlun/genealogy/settled/*.md` 文件中——每个已结算谱系的"下游推论"节就是持久化的行动队列
2. downstream_audit.py 已经能扫描所有已结算谱系并判断哪些下游推论未解决——这就是跨上下文的行动恢复机制
3. 唯一缺失的是：ceremony_scan.py 没有把 downstream_audit 的结果转化为工位

所以答案是：**行动队列已经隐式存在于谱系文件中**，只需要让 ceremony_scan 正确读取它。

session 文件的角色应该被明确：
- **当前角色**：记录做了什么（产出日志）
- **不需要承担的角色**：记录下一步做什么（行动队列）
- **原因**：行动队列应该从权威数据（谱系文件）动态生成，而不是在 session 中静态记录——后者会产生一致性问题（session 说要做X，但谱系已经结算了X）

### 依据

- 谱系文件的"下游推论"节是结构化的编号列表，downstream_audit.py 能解析
- downstream_audit.py 已有 overrides 机制（`.chanlun/downstream-action-overrides.yaml`）处理误报/长期项
- session_update.py 当前的结构（工位列表 + 产出记录）已经足够，不需要增加"待做行动"节

### 边界条件

- 如果蜂群需要在**完全无 ceremony 的情况下**（例如一个被唤起的子蜂群）知道该做什么，则需要额外的传递机制——但当前架构中所有蜂群启动都经过 ceremony，所以这不是当前问题

---

## 问题D：137→138→139的递归修复模式

### 判断

这个模式**不是**蜂群的结构性特征（"递归修复永远滞后于递归违反"），而是**同一个缺口的三次不同表现**。

```
137号：Lead 停顿 → 诊断为 RLHF 基底约束 → 修复方向：格式模板
138号：实施格式模板 → 格式A/B/C
139号后：Lead 用了格式B → 但格式B的判断条件依赖不完整的数据
```

137号诊断的是"否定性禁令无效"——这是对的。138号实施了格式模板——这也是对的。但138号没有修复**格式B的判断条件**。Lead 按格式模板输出了（遵守了138号），但输出了错误的格式（B而非A），因为它的信息来源（ceremony_scan）说"无工位"。

所以这不是"规则修复永远追不上违反"的递归困境，而是**修复停在了表层（输出格式）没有深入到数据层（行动检测）**。

严格的应对：

1. **不是"再加一层修复"**，而是**修 ceremony_scan.py** — 让它把 downstream_audit 的 unresolved 项转化为 workstation 条目
2. 这是一个**行动类**操作（四分法分类）：有逻辑唯一的答案（ceremony_scan 应该集成已有的 downstream_audit），不需要编排者价值判断

### 依据

- 137号的诊断核心是"RLHF驱动停顿"——但139号后的停顿不是 RLHF 驱动的。Lead 选择格式B是因为它的数据说"无工位"，这是数据源缺口，不是行为偏好
- 区分这两种原因很重要：RLHF 驱动的停顿需要格式约束来对抗；数据源缺口需要工程修复

### 边界条件

- 如果修复 ceremony_scan 后 Lead 仍然在有工位时选择格式B，那才是 RLHF 驱动的停顿——此时138号修复确实不够，需要更强的格式约束
- 如果 downstream_audit 产生了过多的误报工位（导致蜂群忙于处理实际上已解决的下游推论），需要调整 downstream_audit 的判定阈值

---

## 可操作的修复方案

### 唯一修复：修改 ceremony_scan.py

在 ceremony_scan.py 的 downstream_audit 调用处（第292-303行），将 unresolved 的下游行动转化为 workstation 条目。

具体修改：

```python
# 当前代码（第292-303行）：只统计，不转化
try:
    from downstream_audit import audit as downstream_audit
    da = downstream_audit(root)
    if da["total_actions"] > 0:
        result["downstream_actions"] = {
            "total": da["total_actions"],
            "unresolved": da["unresolved"],
            "execution_rate": da["execution_rate"],
        }
except Exception as exc:
    result["downstream_actions_error"] = f"{type(exc).__name__}: {exc}"

# 修改为：统计 + 转化 unresolved 为 workstation
try:
    from downstream_audit import audit as downstream_audit
    da = downstream_audit(root)
    if da["total_actions"] > 0:
        result["downstream_actions"] = {
            "total": da["total_actions"],
            "unresolved": da["unresolved"],
            "execution_rate": da["execution_rate"],
        }
        # 将 unresolved 下游行动转化为工位
        for item in da.get("items", []):
            if item["status"] == "unresolved":
                workstations.append({
                    "priority": "P2",
                    "name": f"下游推论未执行：{item['genealogy_id']}号-{item['action_index']}",
                    "status": f"unresolved: {item['text'][:80]}",
                    "source": "downstream_audit",
                })
except Exception as exc:
    result["downstream_actions_error"] = f"{type(exc).__name__}: {exc}"
```

### 为什么不是"再加一条规则"

- 137号已经说了：否定性禁令对行为执行层无效
- 但本次问题不在行为执行层——Lead **遵守了**138号格式模板，选择了格式B
- 问题在数据层——ceremony_scan 给了不完整的信息
- 修复数据层（工程修改）比修复行为层（规则修改）更直接、更确定

### 方案的结构性质

这个修复闭合了 dispatch-dag.yaml 中已声明但未实现的路径：

```
ceremony_sequence.cold_start.nodes.derive-work.scan_sources:
  - "谱系下游行动未执行（spec-execution gap）"  ← 声明存在，实现缺失
```

修复后，ceremony_scan 的行动发现回路变为：

```
ceremony → scan roadmap + session + pending + downstream_audit → workstations
         ↓
谱系结算 → 下游推论写入谱系文件 → downstream_audit 扫描到 unresolved → 生成 workstation
         ↓
Lead 看到 workstations 非空 → 选择格式A → 继续推进
```

回路闭合。不需要新规则，不需要修改 Lead 行为，不需要引入新机制。

---

## 结果包要素

### 结论
138号修复不够的原因不是格式模板本身的问题，而是格式B的判断条件依赖不完整的数据源。ceremony_scan.py 声明扫描"谱系下游行动未执行"但未实现。修复方案：让 ceremony_scan 将 downstream_audit 的 unresolved 项转化为 workstation。

### 定义依据
- dispatch-dag.yaml ceremony_sequence scan_sources 声明
- ceremony_scan.py 第292-303行（downstream_audit 调用但不转化）
- 134号谱系：声明-能力缺口同构模式

### 边界条件
- 如果修复后 Lead 仍然在有工位时选择格式B → 问题回归137号（RLHF驱动），需要更强的格式约束
- 如果 downstream_audit 产生过多误报工位 → 需要调整判定逻辑或 overrides
- 如果已结算谱系超过500条，downstream_audit 全量扫描变慢 → 性能优化（增量扫描/缓存），不影响正确性

### 下游推论
1. ceremony_scan.py 需要将 downstream_audit unresolved 项转化为 workstation（本次修复）
2. 体系中可能存在其他"声明了但未实现"的 scan_sources — 需要对 ceremony_scan 和 dispatch-dag 做一次完整的 spec-execution gap 审计
3. 蜂群持久化的本质不是"行为约束"问题而是"信息通路"问题 — 未来类似问题应先检查数据流是否闭合，再检查行为规则

### 谱系引用
- 137号：RLHF基底约束诊断（本次分析区分了两种停顿原因）
- 138号：格式模板修复（本次分析指出修复范围不够）
- 134号：声明-能力缺口同构（ceremony_scan 的声明-实现缺口是134号模式的又一实例）

### 影响声明
- 建议修改 `scripts/ceremony_scan.py`：downstream_audit unresolved → workstation 转化
- 闭合 dispatch-dag.yaml ceremony_sequence.scan_sources 中"谱系下游行动未执行"的声明-实现缺口
- 不修改任何规则文件 — 问题不在规则层

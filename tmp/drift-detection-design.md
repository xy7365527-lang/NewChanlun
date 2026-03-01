# 执行漂移检测机制设计（278号-2）

**topo_address**: v127-swarm/ws-278-2
**日期**: 2026-03-01

---

## 1. 结论

**推荐方案B（工位 prompt 模板中的强制检查项）**，辅以方案A的轻量变体（ceremony_scan 输出已结算定义摘要作为上下文注入）。

两种方案不是互斥的。方案A提供**数据**（已结算定义的清单和关键约束），方案B提供**行为**（工位启动时的强制自检）。最终设计是二者组合。

---

## 2. 背景分析

### 2.1 问题本质（219号 + 278号收敛）

已结算定义的约束不自动传播到工位。具体表现：

| 谱系 | 漂移实例 | 机制 |
|------|---------|------|
| 219号观察1 | 工位使用外部金融工程语言（ATR止损等），违反缠论语言封闭性 | 工位未加载 domain-conventions skill |
| 278号观察2 | 工位在月线K线上直接跑D算子，违反口径A递归级别定义 | 工位不知道/不记得 level_recursion.md 已结算约束 |

两者的共同根因：**工位在接收任务时，不检查任务方法是否与已结算定义一致**。

### 2.2 当前状态

- `definitions.yaml` 包含 14 个实体定义，各有 `status`、`version`、`testable_assertions`、`boundary_conditions`
- `.chanlun/definitions/*.md` 包含 15 个定义文件，各有版本和状态标记
- `ceremony_scan.py` 当前不涉及定义漂移检测
- 工位 prompt 当前不包含任何已结算定义的强制引用

---

## 3. 方案对比

### 方案A：ceremony_scan 扩展

**机制**：在 scan 阶段增加 `detect_definition_drift()` 函数，扫描代码文件与对应定义的 `testable_assertions` 是否一致，将漂移项作为工位输出。

**优势**：
- 集中化检测，一处维护
- 可产出结构化漂移报告
- 与现有 scan 流程自然集成

**劣势**：
- **性能问题严重**：要做代码-定义一致性检测，需要解析源代码文件、提取逻辑模式、与 testable_assertions 比对。这不是纯文件系统扫描——是语义分析，ceremony_scan 的只读扫描原则无法覆盖
- **误报率高**：静态代码分析无法精确判断"代码行为是否偏离定义"——很多漂移发生在工位的推理层（如277号的口径错误），不在代码层
- **复杂度过高**：ceremony_scan 目前是 ~1200 行的扫描工具，加入语义分析会使其膨胀且难以维护
- **覆盖面不足**：漂移发生在"工位执行任务时使用了错误的方法/语言"，这是运行时行为，不是静态文件状态

**结论**：方案A的核心缺陷是试图用静态扫描解决运行时行为问题。219号和278号的漂移都不是"代码文件中有错误"，而是"工位在执行时未参考定义"。

### 方案B：工位 prompt 模板中的强制检查项

**机制**：每个工位启动时，prompt 中注入与当前任务相关的已结算定义摘要 + 关键约束。工位在执行前必须声明"本任务涉及哪些已结算定义"并检查方法是否合规。

**优势**：
- **精准覆盖**：在漂移实际发生的节点（工位开始执行）施加约束
- **低误报**：工位自行判断相关性，比静态扫描的模式匹配精确
- **低复杂度**：不需要代码解析，只需在 prompt 中注入文本
- **增量性**：不修改 ceremony_scan 核心逻辑
- **直接解决根因**：219号/278号的根因是"工位不知道已结算约束"→ 方案B直接告诉工位

**劣势**：
- **依赖 LLM 合规性**：工位可能忽略 prompt 中的检查要求（RLHF 基底约束，137号已识别）
- **定义子集选择**：14个定义全部注入会增加 prompt 噪声，需要筛选机制
- **无客观验证**：检查结果依赖工位自评，没有外部验证

---

## 4. 组合方案设计

### 4.1 ceremony_scan 扩展（轻量版——数据层）

在 ceremony_scan.py 中增加 `get_settled_definitions_summary(root)` 函数。这不是漂移检测——而是**为方案B提供数据**。

```python
def get_settled_definitions_summary(root):
    """提取已结算定义的关键约束摘要，供工位 prompt 注入。

    只做数据提取，不做漂移判定（漂移判定在工位层）。
    """
    defs_path = os.path.join(root, "definitions.yaml")
    if not os.path.isfile(defs_path):
        return []

    summaries = []
    with open(defs_path, encoding="utf-8") as f:
        data = yaml.safe_load(f)

    for entity in data.get("entities", []):
        if entity.get("status") != "已结算":
            continue
        summaries.append({
            "name": entity["name"],
            "version": entity.get("version", "?"),
            "key_constraints": (
                entity.get("boundary_conditions", [])[:3]  # 前3条边界条件
            ),
            "code_file": entity.get("file", ""),
        })
    return summaries
```

输出在 scan JSON 中以 `settled_definitions` 字段呈现，Lead 注入工位 prompt 时使用。

### 4.2 工位 prompt 强制检查项（行为层）

在工位 prompt 模板的**任务描述之后、执行之前**，插入以下强制段落：

```markdown
## 已结算定义基底检查（强制）

以下是与本仓库相关的已结算定义及其关键约束。
在执行任务前，你必须：

1. 声明本任务涉及哪些已结算定义（如果不涉及任何定义，声明"无"）
2. 对每个涉及的定义，确认你的执行方法不违反其关键约束
3. 如果发现任务方法与已结算定义冲突，停止执行并 /escalate

### 已结算定义清单

{settled_definitions_summary}

### 特别注意

- 级别：唯一正式路径是口径A（递归级别），禁止用时间周期替代（level_recursion v1.0）
- 语言封闭性：缠论域概念必须使用缠师原文术语，禁止引入外部金融工程语言（domain-conventions）
- 中枢组件：必须是次级别走势类型（Move[k-1]），禁止用笔直接构造中枢（zhongshu v1.3）
```

其中 `{settled_definitions_summary}` 由 ceremony_scan 输出的 `settled_definitions` 字段动态填充。

### 4.3 定义子集筛选（278号边界条件处理）

278号边界条件指出："如果已结算定义列表过长，强制工位检查所有定义可能引入噪声"。

筛选策略：**按 code_file 和任务相关性匹配**。

1. 如果工位任务涉及特定代码文件（如 `src/newchan/a_zhongshu_v1.py`），从 definitions.yaml 中匹配 `file` 字段关联的定义
2. 如果工位任务涉及特定概念关键词（如"级别"、"中枢"、"走势"），匹配定义 name
3. 兜底：始终注入"语言封闭性"和"级别口径"两条——这两条是219号和278号已确认的高频漂移源

Lead 在 spawn 工位时做这个匹配，不需要工位自行筛选。

### 4.4 meta-observer 二阶验证（闭环）

meta-observer 在每轮 rescan 时，已有"规则触发/违反模式"检测。增加一项检查：

**检查项**：工位产出中是否有与已结算定义约束冲突的内容。

这不是实时检查——是事后审计。如果 meta-observer 发现某工位产出违反已结算定义，记录为谱系条目（与219号、278号同类型），触发下一轮的定义传播强化。

---

## 5. 实现路径

### 阶段1：ceremony_scan 数据层（增量修改）

1. 在 `ceremony_scan.py` 的 `main()` 中增加 `settled_definitions` 字段输出
2. 新增 `get_settled_definitions_summary(root)` 函数（~20行）
3. 不影响现有 workstations 逻辑

### 阶段2：工位 prompt 模板（Lead 行为变更）

1. Lead spawn 工位时，从 scan 输出的 `settled_definitions` 提取相关定义
2. 在工位 prompt 中注入"已结算定义基底检查"段落
3. 这是 Lead 的行为变更，不需要修改任何脚本

### 阶段3：meta-observer 二阶验证（可选增强）

1. 在 meta-observer agent 的 prompt 中增加"已结算定义执行漂移检查"观察维度
2. 与现有"规则触发/违反模式"检测并列

---

## 6. 定义依据

- 219号谱系：缠论语言封闭性约束缺乏执行层保障——执行漂移的第一个确认实例
- 278号谱系观察2：级别口径约束缺乏执行层保障——执行漂移的第二个确认实例，并概括为系统性缺陷
- 278号下游推论1：已结算定义的执行漂移检测应纳入 ceremony_scan 或工位 prompt 模板的强制检查项
- 137号：否定性禁令对行为执行层无效——因此必须用正面格式（prompt 注入）而非"不要违反定义"的禁令
- definitions.yaml：14个实体定义的 status/version/boundary_conditions 提供检测数据源

## 7. 边界条件

- 如果已结算定义数量增长到 30+ 个，prompt 注入的噪声可能超过信号——此时需要更智能的相关性筛选（如基于任务关键词的 TF-IDF 匹配），但当前 14 个定义足以用简单策略
- 如果工位持续忽略 prompt 中的检查要求（RLHF 基底约束），需要考虑 hook/code-level 强制——但这会大幅增加复杂度，且当前无法在 Claude Code 平台实现任意语义检查 hook
- 如果漂移发生在非缠论域（纯工程任务），definitions.yaml 中的约束不适用——此时工位声明"无涉及定义"即可，不触发检查

## 8. 下游推论

1. 如果方案B有效降低了漂移频率（后续3-5个蜂群循环中未再出现219/278类漂移），应将"已结算定义基底检查"结晶为规则
2. 如果方案B无效（工位仍然忽略检查），需要升级为 hook 级别的强制检测——这构成一个新的工程选择（选择类，需编排者决断）
3. `get_settled_definitions_summary()` 函数可被其他工具复用（如 gangju-audit、quality-guard 的定义合规检查）

## 9. 谱系引用

- 219号：语言封闭性执行漂移的第一个实例
- 278号：级别口径执行漂移 + 系统性缺陷概括
- 204号：搁置模式——278号漂移是其被动变体
- 216号：异质诊断相关失败——"问题不在推理错误，在于缺少验证步骤"同构
- 137号：RLHF基底约束——否定性禁令无效，必须用正面格式
- 016号：规则没有代码强制就不会被执行——219号引用的历史同构

## 10. 影响声明

- **影响 ceremony_scan.py**：增加 `get_settled_definitions_summary()` 函数和 `settled_definitions` 输出字段（增量，不修改现有逻辑）
- **影响 Lead 工位分派行为**：Lead 需要在 spawn 工位时注入定义检查段落（行为变更，不涉及代码修改）
- **影响 meta-observer.md**：增加"执行漂移检查"观察维度（prompt 文本变更）
- **不影响**：definitions.yaml 结构、现有 workstations 逻辑、ceremony 流程步骤

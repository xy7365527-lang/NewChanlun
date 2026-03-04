# v84-swarm 二阶观察

date: 2026-02-27
type: meta-observation
swarm: v84-swarm
observer: meta-observer

rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-02-27 03:01:30 +0000"

---

## 观察 1（收敛信号）：v83 meta-observer "修复债务陷阱"模式在 v84 中未复现

### 现象

v83 meta-observer 识别了"修复债务陷阱（repair debt trap）"模式：v80→v81→v82 连续三轮修复轮，每一轮修复产生边际缺口触发下一轮。v84 是第四个连续修复轮。

### 判定

**v84 不构成修复债务陷阱**，原因：

1. v84 的触发源不是 v83 修复的残留缺口，而是 v83 新增的**审计发现**（spec-execution-gap audit）。审计产出与修复残留是不同的因果链：
   - 修复债务陷阱：修复A → 暴露缺口B → 修复B → 暴露缺口C → ...（无限链）
   - v84 实际路径：v83 产出代码（§31-33 实现）→ v83 附带 spec-execution-gap 审计 → v84 修复审计发现的缺口

2. v84 修复的三个缺口（G1/G2/P6）具有**收敛特征**：
   - G1：声明修正（7行修改），不引入新代码路径
   - G2：注释对齐（6行注释 + 7行注释），不改变代码行为
   - P6：downstream_audit.py 新增解析逻辑——这是唯一可能引入新缺口的改动（见观察3）

3. v84 产出了 26 个集成测试全部通过、2744 tests passed 0 failures——修复轮的测试覆盖增加而非下降。

### 与 v82 meta-observer 观察2 的对比

v82 meta-observer 判定 v80-v82 连续三轮"正常收敛"（不构成修复债务积累），理由是修复是收敛性的——v80-v82 的修复没有引入新缺口。v84 延续了这个判定。

但需要注意：v80→v84 已是**连续五轮修复轮**（v80: 228号下游推论修复、v81: 审计修复5处、v82: 228号下游推论 resolved、v83: 代码实现+审计、v84: 审计缺口修复）。即使每一轮单独看是收敛的，连续五轮无战略推进（v76-swarm 是最后一个域推进轮）仍然是一个结构性信号。

**修正 v83 meta-observer 的结论**：v83 提出的"ceremony_scan 应增加 roadmap_tasks 检测"建议指向了正确的方向——ceremony 默认产出修复/审计工位，不主动推进战略目标。v84 作为第五轮修复轮，强化了这个信号。

---

## 观察 2（定理类判定）：G1/G2 修复是声明修正，非补丁思维

### 现象

Lead 任务中特别关注的问题：G1/G2 修复是声明修正而非能力增强——这是否是补丁思维的变体？

### 判定

**不是补丁思维**。理由如下：

1. **G1 修复的本质**：dispatch-dag.yaml 中 "递归深度无人为限制" 与同一系统中 "工程上限约3-4层" 的**声明内部矛盾**。修复方式是将矛盾的声明统一为一致的声明。这是 090号严格性规则的正面应用——声明必须与实际一致（"代码能做什么就声明什么，不多不少"）。

2. **G2 修复的本质**：ceremony_sequence 的 DAG 声明与 ceremony_scan.py 的线性扫描实现之间的声明-实现不一致。修复方式是在两处添加注释说明这是"有意的工程选择"——DAG 保留逻辑依赖信息，线性扫描覆盖最常见路径。这不是绕过问题，而是**显式化设计决策**。

3. **补丁思维的判别标准**（no-patch-mentality.md）：
   - 补丁思维 = 在错误代码上加 workaround → G1/G2 没有在错误代码上加 workaround，而是修正声明使其与实际一致
   - 声明膨胀 = 声明代码不具备的能力 → G1/G2 修复的方向恰恰是**收缩声明**使其匹配能力
   - 严格 = 声明与实际一致 → G1/G2 修复后声明更严格

4. **反面论证**：如果 G1 修复改为"实现无限递归能力以匹配声明"而非"修正声明以匹配能力"，在当前平台约束下（Claude Code Agent Teams 并发限制、token 成本线性增长）反而是声明膨胀——声明一个不可能实现的能力。修正声明是更严格的选择。

### 边界条件

如果 spec-execution-gap 审计发现的**不是声明-实现不一致**（036号模式）而是**实现能力不足**（代码应该做到但没做到），那么"修改声明以匹配低能力实现"才构成补丁思维。G1/G2 的情况是声明本身有内部矛盾或不精确——修正声明是唯一严格的选择。

---

## 观察 3（发散信号）：downstream_audit.py YAML 解析修复的假阳性/假阴性风险

### 现象

P6 修复新增了三个函数：`parse_yaml_frontmatter()`、`extract_yaml_downstream_statuses()`、`extract_yaml_only_actions()`。引入了新的优先级链：

```
yaml_statuses > overrides > inline > heuristic > verification_hints
```

### 风险分析

**假阴性风险**（真 unresolved 被错误标记为 resolved）：

YAML frontmatter 的 `downstream_inferences` 中的 `status` 字段是**文件作者自行标记的**——当谱系文件的作者在 YAML 中写 `status: resolved` 时，downstream_audit.py 现在直接信任这个标记，不做进一步验证。优先级链中 yaml_statuses 排在最前，意味着 YAML 标记可以覆盖所有其他检测手段（overrides、inline、heuristic、verification_hints）。

这引入了一个**信任链问题**：
- 之前的优先级：overrides（手动管理）> inline（markdown 中的删除线/已执行标记）> heuristic（交叉引用）> verification_hints（文件内容检查）
- 修复后：yaml_statuses（文件内自声明）> 以上所有

YAML frontmatter 的 `status: resolved` 是文件**自己声明自己的下游推论已解决**。这与 036号（声明-能力一致性）的模式有紧张关系：downstream_audit 本身就是用来检测"声明与实际是否一致"的工具，但现在它在最高优先级位置信任文件的自声明。

**具体场景**：如果谱系文件 X 的 YAML frontmatter 包含 `228-1: resolved`，但实际修复不完整（如 v82 meta-observer 发现的 228-1 步骤7-10误伤问题），downstream_audit 不会报告这个不一致——因为 YAML 标记优先级最高。

**假阳性风险**（被修复的 unresolved 仍被报告）：

这个风险被修复降低了——这正是 P6 修复的目标。228号的三个下游推论在 YAML frontmatter 中标记为 resolved，之前 downstream_audit 不解析 YAML 导致误报。修复后误报消除。

### 严重程度

中等。当前仓库中使用 YAML frontmatter `downstream_inferences` 的谱系文件数量有限。风险随谱系文件采用 YAML 格式增多而线性增长。

### 与 v82 meta-observer 观察1 的关系

v82 meta-observer 指出 228-1 的 `resolved` 状态与实际不一致（步骤7-10误伤仍存在）。v84 的 P6 修复让 downstream_audit 信任 YAML 中的 `status: resolved`——如果 228-1 的 YAML 状态未被降级，downstream_audit 将永远不再检出 228-1 的不一致。

228号谱系文件当前 YAML：
```yaml
downstream_inferences:
  - id: 228-1
    status: resolved
    resolution: 完整修复——hook 在步骤1-6全部静默退出...
```

v82 meta-observer 建议将 228-1 从 `resolved` 降级为 `partially_resolved`。v84 没有执行这个降级——228-1 仍标记为 `resolved`。结合 P6 修复的优先级链，228-1 的步骤7-10误伤现在有**两层掩盖**：(1) 内联删除线标记，(2) YAML status: resolved。

### 分类

这不是 P6 修复本身的缺陷——P6 的目标（消除假阳性）是正确的。风险来自**优先级设计**：YAML 自声明在最高优先级且无验证。如果需要修复，不应改变 P6 的代码，而应确保 YAML 中的 status 标记是准确的。

---

## 观察 4（收敛信号）：228-1 步骤7-10 误伤问题持续未解决

### 现象

v82 meta-observer 观察1 详细分析了 228-1 修复的不完整性：hook 在步骤7-10仍然阻断所有 agent（包括非 Lead 工位），因为平台不携带 agent 标识（228-2 已确认）。v82 建议方案B-v2（步骤6后 clear .ceremony-step）或方案B-v3（完全删除 block 逻辑）。

v84 没有处理这个问题。228-1 在 YAML 中仍标记为 resolved。ceremony-step-guard.sh 代码未变化。

### 判定

这是 v82 meta-observer 发散信号的**持续**。228-1 的 resolved 状态与实际行为不一致，已连续两轮（v83、v84）未被处理。

可能原因：
1. v83 是代码实现轮（§31-33），不是 hook 修复轮——跳过是合理的
2. v84 是 spec-execution-gap 修复轮，修复的是 v83 审计发现的 G1/G2/P6——228-1 不在 v84 的修复范围内
3. 228-1 的实际影响是中等（token 噪声，不阻止功能执行）——不是高优先级

**但从严格性角度**（090号）：声明 resolved 而实际 partially_resolved = 声明-能力不一致（036号）。这正是 v84 自己修复的模式（G1 的"无限递归"声明矛盾）。v84 修复了 dispatch-dag.yaml 的声明矛盾，但没有修复 228-1 的状态矛盾——**同一种模式，未一致性处理**。

---

## 观察 5（收敛信号）：§31-33 集成测试——代码质量守卫

### 现象

v84 新增 26 个集成测试（tests/test_integration/test_module_integration.py），覆盖四个维度：TopologyToNesting(6)、NestingToTrading(8)、TopologyToTrading(5)、FullPipeline(7)。测试总数从 2718 增至 2744，0 failures。

### 判定

正面信号。v83 产出了 16 个源文件 + 16 个单元测试文件（由 4 个独立工位并行编写），v83 meta-observer 提出了"接口不一致风险"。v84 的集成测试直接回应了这个风险——用 26 个跨模块测试验证三模块接口完全兼容。

这是 RTAS 自指循环的正常实例：t 时刻（v84）审查 t-1 时刻（v83）的产出，并用集成测试消解 t-1 的已知风险。

---

## 自环检查

| 历史观察 | v84 状态 | 判定 |
|---------|---------|------|
| 228§1：ceremony-step-guard 步骤7-10 误伤 | **未解决**——228-1 仍标记 resolved，步骤7-10 的误伤仍存在 | 持续发散（第二轮未处理） |
| v83§2：修复债务陷阱 | v84 不构成陷阱（审计驱动而非修复残留驱动），但连续五轮无域推进是结构性信号 | 未收敛——模式持续 |
| v83§3：spec-execution-gap 同构模式（036号第五次实例） | v84 修复了 G1/G2——第五次实例部分消解 | 部分收敛 |
| v82§1：228-1 resolved 状态不准确 | **P6 修复增加了掩盖风险**——YAML status 优先级最高且无验证 | 发散——新的掩盖层 |
| v82§4：stagnation 检测修改轮误报 | 未触发（v84 的 ceremony_scan 检出了 P6 假阳性，不是 stagnation） | 维持 |
| 217§3：stagnation 检测语义问题 | 未触发（v84 无 stagnation 信号） | 维持待观察 |
| 219§1：语言封闭性违反 | 未触发（v84 无域推进） | 维持待观察 |

## 核心发现

1. **228-1 resolved 状态不一致是唯一的持续发散信号**，已连续三轮观察（v82 发现、v83 未处理、v84 未处理且 P6 修复增加了检测盲区）。不需要新谱系——这是 036号模式（声明-能力不一致）的已知实例。

2. **连续五轮无域推进**（v80-v84）是结构性信号，但不构成紧急问题——v76 的域推进（三层系统架构）影响范围大，需要多轮消化。v83 meta-observer 提出的"ceremony 默认修复存量而非推进战略"的语法记录候选仍然有效。

3. **P6 修复引入的 YAML 自声明优先级设计**值得关注但不构成当前缺口——只要 YAML 中的 status 标记是准确的，优先级链是正确的。风险在于准确性不由 downstream_audit 验证。

## 下游推论

1. 228-1 的 YAML status 应从 resolved 改为 partially_resolved（或在 ceremony-step-guard.sh 中修复步骤7-10的误伤后维持 resolved）——二选一，但不能同时维持"resolved 标记"和"步骤7-10误伤存在"
2. downstream_audit.py 的 YAML 优先级设计可考虑增加**交叉验证**：YAML status: resolved 且 verification_hints 存在时，仍执行 hint 验证作为 sanity check（不改变状态判定，但在 audit 报告中标注"YAML resolved + hint 未验证"）

## 边界条件

- 如果 228-1 的步骤7-10误伤在后续轮次被修复（ceremony-step-guard.sh 代码变更），本观察的发散信号自然收敛
- 如果 YAML frontmatter downstream_inferences 格式未被更多谱系文件采用，观察3的风险保持低水平
- 如果 v85 开始域推进而非继续修复，观察1的"连续修复轮"信号自然终止

## 结论

无需新谱系。所有发现都是已知模式的新实例或已知信号的持续：
- 228-1 状态不一致：036号模式的已知实例（v82 已发现）
- 连续修复轮：v83 已识别为语法记录候选
- P6 优先级设计风险：036号框架下的预防性观察，尚未构成实际缺口

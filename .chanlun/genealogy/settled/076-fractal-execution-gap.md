---
id: "076"
title: "下游推论执行缺口的自相似性（fractal execution gap）"
type: meta-rule
status: 已结算
date: "2026-02-21"
depends_on: ["074", "036", "012"]
related: ["071", "072", "075"]
negated_by: []
negates: []
provenance: "[新缠论]"
negation_form: "expansion（074号元模式在元层的再现）"
negation_source: "homogeneous（v17 蜂群 meta-observer 扫描）"
---

# 076 — 下游推论执行缺口的自相似性

## 背景

v17 蜂群 meta-observer 对 074号谱系的下游推论执行状态进行追踪扫描。074号识别了"谱系结算后下游行动执行率低"的元模式（072号执行率 16.7%）。本次扫描发现：074号自身的下游推论同样存在执行缺口。

## 具体发现

### 发现 A：074号下游推论执行状态

| # | 推论 | 状态 |
|---|------|------|
| 1 | ceremony-guard.sh 更新文件引用 | ⚠ 注释已更新（第14行），代码未改（第40行仍为 dispatch-spec.yaml） |
| 2 | 073号编号冲突解决 | ✅ 已完成（073/073a/073b 三文件） |
| 3 | dispatch-dag.yaml 第11行注释更新 | ✅ 已完成（现为"推荐流程"） |
| 4 | Session 遗留项更新 | ⚠ v16 session 部分反映，但未标记074号修复项完成状态 |
| 5 | 语法记录候选（自动追踪机制） | ❌ 未执行 |

执行率：2/5 = 40%。高于074号报告的072号执行率（16.7%），但仍不足半数。

### 发现 B：dispatch-spec 幽灵引用（范围比074号报告更广）

074号仅识别了 ceremony-guard.sh 的引用问题。实际扫描发现 dispatch-spec 引用散布在至少 5 个 hook 中：

| Hook 文件 | 行号 | 引用类型 |
|-----------|------|----------|
| ceremony-guard.sh | 40, 50, 56, 134 | 核心逻辑（SPEC_FILE 变量） |
| topology-guard.sh | 42, 117 | 文件路径匹配 |
| recursive-guard.sh | 12, 117, 118 | SPEC_FILE 变量 + 注释 |
| spec-write-guard.sh | 52 | 正则匹配模式 |
| hub-node-impact-guard.sh | 45 | 输出消息文本 |

ceremony-guard.sh 第14行有注释 `# 074号修复：dispatch-spec.yaml → dispatch-dag.yaml`，但第40行代码未改。注释承认了问题但代码未修复——这是"注释即修复"的反模式。

### 发现 C：code-verifier 节区错位

dispatch-dag.yaml 中 code-verifier（第115-120行）位于 `optional_structural` 节区下，但属性为 `mandatory: true, type: dominator`。mandatory dominator 不应在 optional 节区中。

## 元模式识别

074号识别的模式是：**谱系是发现引擎（012号），不是任务管理系统。** 下游推论写在谱系的文本中，但谱系没有"任务追踪"的语义。

本次扫描确认该模式具有**自相似性**：关于"执行缺口"的谱系（074号）本身也有执行缺口。074号下游推论#5 提出了"自动生成 task"的机制，但这条推论本身也未被执行。

**语法记录候选**：系统中实际运作的隐性规则是——"下游推论靠蜂群工位主动认领，不靠自动追踪"。这条规则应被显式化：
- 接受它 → 降低对执行率的期望，将"下游推论"重新定义为"建议"而非"待办"
- 改变它 → 引入谱系结算时自动生成 task 的 hook（074号#5 提案）

## 边界条件

- 如果 074-fixes 工位（当前 in_progress）正在修复上述问题 → 发现 A/B 的执行率将提升，但元模式仍成立（修复依赖工位主动认领，不是自动追踪）
- 如果引入"谱系结算 → 自动生成 task"的 hook → 元模式被机制化否定
- 如果下游推论被重新定义为"建议"而非"待办" → 执行率不再是问题指标

## 下游推论

1. **dispatch-spec 幽灵引用需全局清理**：不仅是 ceremony-guard，至少 5 个 hook 需要更新（见发现 B 表格）
2. **code-verifier 应移至 structural 节区**：或将 mandatory 改为 false
3. **语法记录决断**：隐性规则"下游推论靠主动认领"需要被显式化（选择/语法记录，应路由 Gemini decide）

## 影响声明

- 新增 pending 谱系 076（type: meta-rule）
- 不修改任何现有文件
- 为 074-fixes 工位提供扩展范围（5 个 hook 而非 1 个）
- 为 Lead 提供语法记录候选（下游推论的语义定位）

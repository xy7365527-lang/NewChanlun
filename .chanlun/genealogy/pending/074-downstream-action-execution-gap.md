---
id: "074"
title: "谱系下游行动执行缺口——系统性观测"
status: "生成态"
type: "meta-rule"
date: "2026-02-21"
depends_on: ["036", "071", "072", "073"]
related: ["016", "033", "058"]
negated_by: []
negates: []
---

# 074 — 谱系下游行动执行缺口——系统性观测

**类型**: meta-rule
**状态**: 生成态
**日期**: 2026-02-21
**前置**: 036-spec-execution-gap-crystallization, 071-text-instruction-fragility-scan, 072-hook-enforcement-dual-prerequisite, 073-swarm-full-mutability

**negation_form**: expansion（036号 spec-execution gap 在谱系层的再现）
**negation_source**: homogeneous（v16 蜂群元观测扫描）

---

## 背景

v16 蜂群热启动后，meta-observer 对系统当前状态执行全面扫描。发现一个反复出现的模式：谱系结算后，其"下游推论"章节列出的具体行动未被系统性追踪和执行。这是 036号（spec-execution gap）在谱系层面的再现。

## 具体发现

### 发现 A：ceremony-guard 读取已废弃文件

ceremony-guard.sh 第 40 行：`SPEC_FILE=".chanlun/dispatch-spec.yaml"`

系统已从 dispatch-spec 迁移到 dispatch-dag（072号下游推论#6）。ceremony-guard 仍读取旧文件。如果 dispatch-spec.yaml 不存在，hook 静默放行（第 50-53 行），使整个 hook 完全失效。

**来源**：072号下游推论#6 触发了 dispatch-dag 迁移，但未同步更新 ceremony-guard 的文件引用。

### 发现 B：072号下游推论未执行

072号明确列出 6 条下游推论：
1. ceremony-guard 从 warning 升级为 blocking — **未执行**
2. 071号 ceremony-guard 评级从 ✅ 降为 ⚠️ — **未执行**（071号第 52 行仍为 ✅）
3. 所有 hook 审查 blocking vs warning-only — **未执行**
4. bypassPermissions 兼容性确认 — **未执行**
5. 双层模型写入 013号或 dispatch-spec — **未执行**
6. dispatch-spec → dispatch-dag 重构 — **已执行**（dispatch-dag.yaml 存在）

6 条中仅 1 条执行。执行率 16.7%。

### 发现 C：073号谱系编号冲突

`.chanlun/genealogy/settled/` 中存在两个 id="073" 的文件：
- `073-swarm-full-mutability.md`（全可变性原则）
- `073-trampoline-recursion-approximation.md`（Trampoline 递归近似）

两者均在 YAML frontmatter 中声明 `id: "073"`。genealogy-write-guard 的前置引用验证使用前缀匹配（`ref + '*.md'`），引用 "073" 会匹配两个文件，产生歧义。

### 发现 D：dispatch-dag.yaml 注释与原则0不一致

dispatch-dag.yaml 第 11 行：`# 修改路径：/ritual 仪式门控`

073号已将仪式从"强制门控"降为"推荐流程"。CLAUDE.md 原则5 也写"推荐通过仪式"。dispatch-dag 注释未同步更新。

### 发现 E：Session 遗留项状态过时

Session v15 列出的 P1 遗留项中：
- "genealogy-write-guard 改为 allow" — **已完成**（hook 注释第 11 行："原则0：蜂群能修改一切。守卫只验证，不阻断。"，输出 `'decision': 'allow'`）
- "result-package-guard 改为 allow" — **已完成**（hook 注释第 9 行："validate但不阻止，原则0"，输出 `'decision': 'allow'`）

Session 记录未反映这两项的完成状态。

## 元模式识别

五个发现共享同一根因：**谱系结算后，下游行动缺乏系统性追踪机制**。

当前流程：谱系结算 → 下游推论写入文本 → 依赖人/agent 记忆执行 → 遗忘/遗漏。

这是 F3 型脆弱性（071号分类：跨 turn 状态保持）在谱系层的实例。谱系的"下游推论"本质上是一个 TODO 列表，但没有任何机制将其转化为可追踪的任务。

## 边界条件

- 如果蜂群每轮都完整扫描所有 settled 谱系的下游推论 → 缺口消失（但 O(n) 扫描成本随谱系增长不可持续）
- 如果引入"谱系结算时自动生成 task"的 hook → 缺口被机制化解决
- 如果下游推论全部是定理（自动结算）→ 不需要追踪（但实际上多数是行动类）

## 下游推论

1. **ceremony-guard.sh 需更新文件引用**：`dispatch-spec.yaml` → `dispatch-dag.yaml`，并适配 DAG 格式的 mandatory 节点解析
2. **073号编号冲突需解决**：其中一个文件应重编号（建议 073-trampoline 改为 073b，因 073-swarm-full-mutability 已被 CLAUDE.md 原则0 引用）
3. **dispatch-dag.yaml 第 11 行注释需更新**："仪式门控" → "推荐流程"
4. **Session 遗留项需更新**：标记已完成的 P1 项
5. **语法记录候选**：谱系结算时，下游推论中的"行动"类条目应自动转化为可追踪任务（机制待设计）

## 影响声明

- 新增谱系记录 074（pending/，type: meta-rule）
- 不修改任何现有文件（修复由任务工位执行）
- 为 Lead 提供 5 条具体可执行的修复项

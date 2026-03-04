# Gemini Decide 上下文：096号修正——蜂群规则的存在方式

## 决断类型：语法记录

编排者辨认出了"skill 是蜂群规则的存在方式"这条已在运作的规则，需要显式化。

## 问题陈述

096号谱系（已结算）在描述"规则的分布式存在形式"时写道：
> "规则的分布式存在形式 = CLAUDE.md（基因组）+ hooks（免疫系统）"

并将 team-structural-inject.sh 降为"纯信息性提示"（096号谱系第83行）。

编排者 INTERRUPT：
- "我的意思是不应该结晶成 skill 吗？怎么能是信息提示呢？"
- "蜂群存在的方式不是 skill 吗？"

## 相关已结算谱系

### 075号（结构工位从teammate转为skill+事件驱动）
- 已结算（2026-02-21）
- 核心结论：结构工位（genealogist/quality-guard等）从 teammate 转为 skill + 事件驱动
- dispatch-dag 定义"事件→skill 映射"，不是"必须 spawn 的 agent 列表"
- 原则11推论："元编排本身也是 skill 的集合——编排能力分布式地结晶在 skill 中，按需加载，用完析出"

### 089号（元编排扬弃——CLAUDE.md 是基因组）
- 规则的分布式存在形式 = CLAUDE.md（基因组）+ hooks（免疫系统）
- knowledge_templates: path = ".claude/skills/"，role = "蜂群的结晶知识——被 agent 读取后转化为行为能力"

### 095号（Agent Team 真递归）
- 子蜂群 ceremony skill 已存在于 .claude/skills/sub-swarm-ceremony/SKILL.md
- sub-swarm-ceremony skill 定义了子蜂群创建的完整流程

### CLAUDE.md 原则11
> "知识在 prompt 中的生命周期有走势结构：扩张期→收缩期→结晶→重新加载"
> "结晶的三个维度：session（时间维度）、skill（知识维度）、definitions（概念维度）"

## 现状分析

### dispatch-dag.yaml 中的 genome_layer.knowledge_templates
```yaml
knowledge_templates:
  path: ".claude/skills/"
  role: "蜂群的结晶知识——被 agent 读取后转化为行为能力"
  note: "skill 是知识维度的结晶产物（原则11），由 Claude Code 按需加载"
  skills:
    - "gemini-math"
    - "knowledge-crystallization"
    - "math-tools"
    - "meta-orchestration"
    - "orchestrator-proxy"
    - "spec-execution-gap"
```

注意：sub-swarm-ceremony skill 已存在，但未注册在 knowledge_templates 中。

### team-structural-inject.sh 现状（096号修正后）
```
# 096号：仅信息性提示，不注入规则（规则在 CLAUDE.md 基因组中，分布式自动加载）
print(json.dumps({
    'decision': 'allow',
    'reason': f'[075号 skill 架构] 蜂群 {team_name} 已创建。{count} 个 structural skill 由事件自动触发...'
}))
```

### sub-swarm-ceremony skill（已存在）
位于 .claude/skills/sub-swarm-ceremony/SKILL.md
- 触发事件：TeamCreate（由 teammate 而非 Lead 发起时）
- 包含子蜂群创建完整流程
- 谱系依据：095号、056号、069号、016号

## 三个选项

### A：team-structural-inject.sh 从"信息提示"修正为"skill 触发提示"
- 输出中明确引用 sub-swarm-ceremony skill 路径和使用方式
- skill 本身包含完整的子蜂群创建规则
- 保留 hook 但改变其语义：从"信息性提示"→"skill 索引器"
- 096号谱系影响声明部分需要更新

### B：删除 team-structural-inject.sh，完全依赖 skill 自动触发
- sub-swarm-ceremony 已在 dispatch-dag 的 event_skill_map 注册（理论上）
- 但实际上 dispatch-dag event_skill_map 中没有 sub-swarm-ceremony 条目
- 依赖 agent 主动读取 skill——但没有显式提示，agent 可能不会发现
- 风险：没有显式提示，skill 等于隐形

### C：三层存在形式显式化——修正 096号谱系
- 096号的声明"规则分布式存在 = CLAUDE.md + hooks"改为"CLAUDE.md + hooks + skills"
- 明确：CLAUDE.md 声明原则、hooks 强制语法、skills 提供可执行流程
- team-structural-inject.sh 继续存在，但语义是"skill 索引器"（不是规则注入，不是信息提示）
- 同时在 dispatch-dag knowledge_templates 中注册 sub-swarm-ceremony
- 096号谱系修正，标注三层存在形式

## 决断约束

1. 075号已确立：结构能力 = skill（事件驱动），不是 teammate
2. sub-swarm-ceremony skill 已存在
3. dispatch-dag event_skill_map 没有注册 sub-swarm-ceremony（只有 structural skills 如 genealogist/quality-guard）
4. CLAUDE.md 原则11：skill = 知识维度结晶
5. 096号的"信息性提示"定位被编排者否定

## 请 Gemini 决断

这是"语法记录"类：编排者辨认出"蜂群规则的完整存在形式包含 skill 层"这条已在运作（sub-swarm-ceremony 已存在）但未显式化在 096号谱系声明中的规则。

请：
1. 选择 A/B/C 其中一个（或提出更优方案）
2. 给出推理链
3. 如果选 A 或 C，指出需要修改哪些文件

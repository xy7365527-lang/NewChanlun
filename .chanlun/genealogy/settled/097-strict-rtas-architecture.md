---
id: '097'
title: 真严格递归拓扑异步自指蜂群的完整架构——hook 纯化 + 五特征 DAG 模板
type: 选择
status: 已结算
date: 2026-02-22
depends_on:
  - '069'   # 递归拓扑异步自指蜂群定义
  - '089'   # 元编排严格扬弃——CLAUDE.md 是基因组
  - '093'   # 五约束有向依赖图
  - '094'   # 三个 Gap 重新定位
  - '095'   # Agent Team 真递归
  - '096'   # 分布式规则存在形式——无例外
negation_source: 编排者 INTERRUPT（"严格架构，不要有余地，最强方案"）+ Gemini decide（异质决断）
negation_form: expansion（096号三层存在形式的精化 + 子蜂群完整性的结构性保障）
negates: []
negated_by: []
---

# 097号：真严格递归拓扑异步自指蜂群的完整架构——hook 纯化 + 五特征 DAG 模板

**类型**: 选择
**状态**: 已结算
**日期**: 2026-02-22
**前置**: 069, 089, 093, 094, 095, 096
**negation_source**: 编排者 INTERRUPT + Gemini decide

## 编排者约束（触发本号的 INTERRUPT 序列）

1. "如果是严格地这样存在，那么 hook 是不是就不够优雅和干净了"——hook 做提示/索引违反三层纯粹边界
2. "不仅是这样，思考真严格递归拓扑异步自指蜂群的存在，那么整个架构应该是怎么样的，这是一个严格架构，不要有什么余地，要严格地讨论最强的方案"——从蜂群存在论推导完整架构

## 决断结论（两部分）

### Part 1：Hook 层纯化（Gemini decide 选项D）

**team-structural-inject.sh 删除**。理由：

`team-structural-inject.sh` 是 PostToolUse hook（TeamCreate 后触发），功能是输出文字告诉 agent "必须读取 sub-swarm-ceremony skill"。该功能不属于三层任何一层：
- 不是"阻断"（allow 了 TeamCreate）
- 不是"声明"（CLAUDE.md 已有原则15）
- 不是"执行"（不含可执行流程）

它是夹在三层之间的"提示/索引"第四层——违反三层纯粹边界。

| 选项 | 描述 | 结果 |
|------|------|------|
| A | 删除 hook——三层已自足 | — |
| B | 保留但改为纯列表输出 | — |
| C | 改为 PreToolUse 验证阻断 | 技术不可行 |
| **D** | **删除 hook，skill 路径写入 CLAUDE.md** | **✅ 选中** |

推理链：
1. hook 层的动作语义必须是"阻断/放行"，绝不能是"提示/教学"
2. CLAUDE.md 作为声明层包含 skill 路径索引——类比启动子（Promoter），指引转录过程
3. 彻底消除"第四层（提示层）"

### Part 2：完整架构——五特征 DAG 模板（Gemini 方案 X+Z 综合）

六个核心问题的回答：

**问题1：规则存在形式是否已达最强？**
→ **是。** CLAUDE.md（全层级自动加载）+ hooks（全层级自动触发）+ skills（按需加载）三层是当前平台下的最大覆盖。禁止通过 prompt 传递结构性规则。

**问题2：子蜂群完整性验证**
→ **通过 skill 内模板解决，不需要新 hook。** sub-swarm-ceremony skill 声明四类节点 DAG 模板（任务节点≥2、审查节点、结晶节点、异质审计节点）。agent 读取 CLAUDE.md 和 skill 后自主遵守。不增加验证 hook（避免"自身免疫性疾病"——hook 过严导致子蜂群在创建阶段即死锁）。

**问题3：ceremony 在子蜂群中的角色**
→ **sub-swarm-ceremony skill = 子蜂群的 Swarm₀。** L(n-1) Teammate 执行本 skill 时充当 L(n) 的创世者。创世 Gap 在每层递归中独立存在（094号）。

**问题4：dispatch-dag 在子蜂群中的角色**
→ **子蜂群不需要独立的 dispatch-dag YAML 文件。** 子蜂群的 DAG 结构模板在 sub-swarm-ceremony skill 中静态声明（四类节点 + 依赖边），通过 TaskCreate 运行时实例化为 TaskList。dispatch-dag.yaml 的 `fractal_template` 声明 `inherited_skills: "all"`，子蜂群自动继承所有 skill。

**问题5：异质验证（约束4）在子蜂群中**
→ **强制。** 没有异质审计节点的子蜂群 = 封闭自证循环 = 违反约束4（093号）。最小实现：子蜂群 Lead 在汇总关键产出前调用 Gemini 质询。纯操作性（行动类，018号四分法）产出可免除。

**问题6：最强方案**
→ **方案 X+Z 综合**：三层存在形式（已实现）+ 子蜂群五特征 DAG 模板（在 skill 中声明）+ 异质验证强制化（约束4在每层的最小实现）。不需要新增 hook 或动态生成 YAML。

### 五特征 DAG 模板（097号核心增量）

| 节点类型 | 数量 | 四维度对应 | 五约束对应 |
|----------|------|-----------|-----------|
| 任务节点 | ≥2 | 拓扑（并行 DAG） | 约束1a（物理持久化） |
| 审查节点 | ≥1 | 异步自指 | 约束3（不可自观） |
| 结晶节点 | ≥1 | 结晶 | 约束1a + 约束1b |
| 异质审计节点 | ≥1 | — | 约束4（异质验证） |

095号定义四特征（拓扑/异步自指/结晶/状态管理），097号将约束4/异质验证从隐含提升为显式第五特征。

## 边界条件

| 条件 | 翻转方向 |
|------|---------|
| Claude Code 不允许 teammate 使用 TeamCreate | 真递归不可行，回退 Trampoline（095号边界条件） |
| 递归深度超过 3-4 层导致视差 Gap 累积显著 | 设置工程硬性深度上限 |
| 异质验证（Gemini）在子蜂群中因 MCP 工具不可用 | 约束4降级为父蜂群事后审计 |
| skills 数量激增导致 CLAUDE.md 的 skill 索引过长 | 引入"路由 skill"或"目录文件"接管索引 |

## 下游推论

1. sub-swarm-ceremony skill 已更新：新增五特征 DAG 模板 + 创世 Gap 递归化 + 异质审计节点强制
2. CLAUDE.md 原则15 已更新：子蜂群五特征描述 + 创世 Gap 递归化
3. team-structural-inject.sh 已删除，settings.json 中已移除注册
4. dispatch-dag.yaml 注释更新（移除 team-structural-inject.sh 引用）
5. 096号谱系的 hook 索引器定位被 097号否定——hook 回归纯阻断/放行语义

## 推导链

1. 096号确立三层存在形式（CLAUDE.md + hooks + skills）
2. 编排者 INTERRUPT："hook 做索引不够优雅"→ Gemini decide 选项D → 删除 hook
3. 编排者 INTERRUPT："思考完整架构，最强方案"→ Gemini 方案 X+Z
4. 分析六个核心问题 → 三层已是当前平台下最大覆盖 + 缺口在 sub-swarm-ceremony skill
5. 子蜂群完整性 = skill 内声明五特征 DAG 模板（不是新 hook）
6. 创世 Gap 递归化 = L(n-1) 执行 skill 时充当 L(n) Swarm₀
7. 约束4强制化 = 异质审计节点是五特征之一（从隐含到显式）
8. 异质质询验证（Gemini decide）：推理链成立 ✅

## 谱系链接

- **096号**（三层存在形式）→ 097号精化三层边界 + 补充子蜂群完整性保障
- **095号**（真递归）→ 097号将四特征扩展为五特征（新增异质验证）
- **094号**（三个 Gap）→ 097号将创世 Gap 递归化写入 skill
- **093号**（五约束）→ 097号将约束4在子蜂群中强制化
- **069号**（蜂群定义）→ 097号是069号的完整架构实现方案
- **090号**（严格性是语法规则）→ hook 的动作语义必须严格
- **005b号**（对象否定对象）→ 验证机制的合法性约束

## 影响声明

### 修改
- `.claude/hooks/team-structural-inject.sh`：**删除**（hook 纯化）
- `.claude/settings.json`：移除 TeamCreate matcher 中已删除 hook 的注册
- `.claude/skills/sub-swarm-ceremony/SKILL.md`：新增五特征 DAG 模板 + 创世 Gap 递归化 + 异质审计节点
- `CLAUDE.md` 原则15：补充五特征描述 + 创世 Gap 递归化
- `.chanlun/dispatch-dag.yaml`：移除 team-structural-inject.sh 引用
- `.chanlun/genealogy/dag.yaml`：新增 097 节点

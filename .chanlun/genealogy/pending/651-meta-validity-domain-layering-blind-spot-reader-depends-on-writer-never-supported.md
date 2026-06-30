---
id: 651
title: 元规则观测——formalization-validity-domain 的有效域分层缺一个边界条件（reader 有效域 ⊋ writer 有效域时，"良性分层"与"分层盲区"无判据区分）
status: 生成态
type: meta-rule
layer: 元编排/方法论层（关于"如何声明有效域"的二阶知识）
session: 68088c95（meta-observer 第二轮）
discovered_by: meta-observer
created: 2026-06-30
related: [630（写路径未实装开口①，声明"有效域=读侧"）, 650（GOAL_RESUME 可读不可写——分层盲区揭露）, 231（formalization-validity-domain 规则谱系）, 222/223/230（有效域≠定义域三例）]
rule_version_baseline:
  claude_md_commit: "4f040f0c31bb38a58b1adbbde5708096038eea65"
  rules_dir_mtime: "2026-03-14 23:27:42 +0000"
---

# 元规则观测

## 一阶事实（被观测对象）

630 在交付 D′ goal 事件系统时，把"reader（goal_reducer）宽容读历史 ⊋ writer（goal_events）严格守新写"声明为**有意的有效域分层**，并显式标注"有效域=读侧"（630 开口① 措辞）。这一声明引用 formalization-validity-domain 规则作为合法性依据——分层不是补丁，是诚实标注的有效域边界。

650 发现这个分层有盲区：reader 依赖 `GOAL_RESUME`（做 base_head 再锚定，是 /goal 协议的语义必需事件），但 writer 与 SCHEMA 从未支持该事件类型。历史 GOAL_RESUME 全靠裸 append 绕过 writer 写入——而裸 append 正是 writer 设计要消灭的退化写法。

## 二阶观测（关于规则本身的元知识）

formalization-validity-domain 规则区分"定义域"与"有效域"，要求声明有效域时标注认识论等级（L0-L3）。但规则的全部三个发生史案例（222 守恒律 / 223 分类 / 230 直积）都是**单一形式化操作**的有效域 < 定义域。

630→650 揭示了一个**规则未覆盖的新结构**：当形式化操作分裂为 reader/writer 两个角色，且 **reader 的有效域 ⊋ writer 的有效域**时，存在两种本质不同的情形，而现行规则没有判据区分它们：

| 情形 | 结构 | 性质 | 例子 |
|------|------|------|------|
| 良性分层 | reader 多读的部分是 writer **曾经合法产生、现已淘汰**的退化历史 | 诚实标注，向后兼容 | 630 原意：reader 读历史"sub_goal_id 当 goal id"的退化事件 |
| 分层盲区 | reader 多读的部分是 writer **从未支持、SCHEMA 从未定义**的事件 | 声明膨胀的镜像——reader 依赖一个不存在的契约 | 650：GOAL_RESUME 可读不可写 |

**核心元命题**：在 reader/writer 角色分裂下声明"有效域=读侧"是**不充分的**。必须进一步声明 reader 多读的部分**是 writer 退化的历史（良性）还是 writer 从未支持的幽灵契约（盲区）**。前者向后兼容，后者是把"功能依赖一个未实装的写路径"伪装成"有效域分层"——本质上是 formalization-validity-domain 禁止的"定义域=有效域假设"的对偶：reader 单方面把自己的定义域（能读的全部事件类型）当成了系统的有效域（writer 能产生的事件类型），而二者之间隔着一个从未建造的桥。

## 与现行规则的关系

这不是规则的反例，而是规则的**有效域边界条件缺失**——formalization-validity-domain 自身的有效域（"单一形式化操作的有效域≠定义域"）严格小于它被引用时覆盖的定义域（630 用它为"reader/writer 分裂"背书）。规则被用在了它自己未验证的场景上，恰是它禁止的模式的元层重演。

## 边界条件（结论翻转条件）

- 若 650 的 GOAL_RESUME 经 /ritual 裁定为立场B（reducer 删除 RESUME，base_head 不靠事件再锚定），则"reader 多读"被消除，reader 有效域收缩到 = writer 有效域，本观测的"盲区"案例消失，但"良性分层 vs 盲区无判据区分"的元命题仍成立（针对未来的 reader/writer 分裂）。
- 若现行 formalization-validity-domain 规则本就隐含"reader 有效域必须 ⊆ writer 曾产生的事件集"，则本观测降级为对规则的澄清而非扩展——需 /ritual 确认规则原意是否覆盖。

## 下游推论

1. 凡声明"有效域=读侧/reader 宽容"的产出，必须附带 reader 多读集合的来源分类：writer 退化历史（良性）vs writer 从未支持（盲区）。
2. 盲区类必须走 no-workaround——它是"功能依赖未实装写路径"的伪装，不是有效域分层。
3. formalization-validity-domain 规则候选扩展第 5 条禁止模式："**角色分裂下的有效域单边声明**：reader/writer 分裂时只声明 reader 侧有效域而不声明 reader 多读集合是否在 writer 的历史产出范围内。"

## 四分法分类

**语法记录候选**——这是 630 已在运作但未显式化的隐性规则（630 实践中区分了"历史退化"与"幽灵契约"，但规则文本未把这个区分写进 formalization-validity-domain）。经 /escalate 上浮辨认，由 Lead 决定是否走 /ritual 扩展规则。

## 异质审查约束声明

本观测**不**提议修改 dispatch-dag.yaml，仅提议扩展 formalization-validity-domain 规则文本（元层 /ritual 范围）。若 Lead 判定需走 /ritual，规则文本扩展提案应经异质审查（meta-observer 既是提案者又是监控者的构成性利益冲突防护）。

## 自环检查（收敛/发散）

- **发散信号**：history 中无 reader/writer 角色分裂下的有效域分层观测——这是 formalization-validity-domain 的新维度（前三例均单一操作），写入新谱系而非标注重复。
- **收敛锚点**：与 222/223/230 共享同一根（有效域≠定义域），但施加对象从"单一形式化操作"扩展到"分裂的角色对"。

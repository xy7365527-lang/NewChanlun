---
id: "075"
title: "结构工位从teammate转为skill+事件驱动——消除孤岛"
类型: 语法记录
状态: 已结算
日期: 2026-02-21
前置:
  - "073"
  - "069"
  - "057"
  - "062"
related:
  - "020"
  - "041"
provenance: "[新缠论]"
gemini_decisions:
  - "skill+事件驱动架构（方案D）：取消所有结构工位teammate，全部转化为skill"
---

# 075: 结构工位从 teammate 转为 skill + 事件驱动

## 矛盾

结构工位（genealogist/quality-guard/meta-observer）作为独立 teammate spawn，导致：
1. 孤岛——teammates 之间没有共享状态，结晶 teammates 是信息死角
2. prompt 膨胀——每个 teammate 的 prompt 越来越长越脆弱
3. 硬编码——dispatch-dag 定义"必须 spawn 的 agent 列表"
4. Gemini 作为 teammate 概念错位——外部 API 不是 agent

## 推导链

1. 原则 11 推论："元编排本身也是 skill 的集合——编排能力分布式地结晶在 skill 中，按需加载，用完析出。"
2. 057号：LLM 不是状态机——"主动审查"是时间幻觉，审查应该是事件驱动的
3. 062号：异质碰撞——Gemini 是 skill（能力），不是 teammate（实体）
4. 073号：蜂群能修改一切——包括自己的架构

## 结论

**结构工位从 teammate 转为 skill + 事件驱动。**

| 之前（teammate） | 之后（skill + 事件） |
|------------------|---------------------|
| genealogist teammate | genealogy skill，task 完成时触发 |
| quality-guard teammate | quality-guard skill，Write 后 hook 触发 |
| meta-observer teammate | meta-observer skill，Stop 时触发 |
| gemini-challenger teammate | gemini skill，/challenge 或事件触发 |
| code-verifier teammate | verify skill，代码变更后 hook 触发 |

dispatch-dag 定义"事件→skill 映射"，不是"必须 spawn 的 agent 列表"。

## 边界条件

- hooks/事件系统无法捕获足够细粒度的状态变化 → 重新引入守护 teammate
- 事件风暴（skill 输出触发新事件 → 无限递归）→ 需要事件去重/深度限制
- 当前 hooks 只能输出文本，不能执行复杂逻辑 → 可能需要 Python hook

## 下游推论

1. ceremony 不再 spawn 结构工位——只 spawn 业务 teammates
2. dispatch-dag 格式重构：从 `nodes.structural[mandatory]` 改为 `event_skill_map`
3. ceremony_scan.py 不再输出 `structural_nodes`——输出 `required_skills`
4. fractal_template 不再继承 teammates——skill 是全局可用的
5. Stop hook 不再检查 dominator node 存在性——检查 skill 是否被调用过

## 影响声明

- 架构级变更：从 teammate 模式转为 skill + 事件驱动模式
- 影响：dispatch-dag、ceremony_scan.py、ceremony.md、所有 structural hooks
- 不影响：业务 teammates 的 spawn 方式、Gemini API 调用方式

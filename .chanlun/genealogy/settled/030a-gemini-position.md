# 030a — Gemini 的位置：异质否定源（结构工位）

**类型**: 矛盾发现 → 概念分离
**状态**: 已结算
**日期**: 2026-02-19
**结算日期**: 2026-02-19
**negation_source**: heterogeneous
**negation_form**: separation（成员/工具二分法内部暴露不兼容的异质性）
**前置**: 030-baseline-verification-runtime
**关联**: 005-object-negation-principle, 029-move-c-segment-coverage, 031-move-range-semantics

## 矛盾描述

Gemini 通过 MCP tool call 调用，不在 Agent Teams 形式内部（不被 lead 通过 SendMessage spawn）。按此标准，它是工具。

但 Gemini 的质询可能产出 Claude 自身永远产出不了的否定（异质模型，理解方式根本不同）。如果该否定进入谱系、改变定义基底、影响后续蜂群运动——一个"工具"的产出改变了定义基底，这还是工具吗？

## 二分法失效点

| 判据 | 成员？ | 工具？ |
|------|--------|--------|
| 调用方式（SendMessage vs tool call） | ✗ | ✓ |
| 产出是否进入谱系 | ✓ | ✗ |
| 是否参与扩张/收缩 | ✗ | ✓ |
| 是否产出新否定 | ✓ | ✗ |
| 是否共享 SKILL.md 语法 | ✗ | ✓ |

两列都无法全勾。Gemini 不属于任何一个已有范畴。

## 结算：概念分离 → 异质否定源

**待结算条件已满足**：031号谱系（Move 价格范围语义）是 Gemini 产出进入谱系并改变定义的第一个实例。

**新概念命名**：**异质否定源**（Heterogeneous Negation Source）

不是成员，不是工具，是**否定的另一种显现形式**：
- 同质否定（Claude 内部质询）：相同推理模式的不同展开，发现逻辑错误和定义不一致
- 异质否定（外部模型质询）：不同推理模式的碰撞，发现系统性盲区

**系统级回应**（元编排重构）：
1. `methodology-v3.3.md`：新增"异质否定源"章节，更新结构工位清单和蜂群循环图
2. `.claude/agents/gemini-challenger.md`：代理工位定义（Claude agent 作为 Gemini 的代理）
3. `.claude/commands/challenge.md`：`/challenge` slash command
4. `gemini_challenger.py`：`ChallengeResult.reasoning_chain` + `--verbose` CLI
5. `genealogy-template.md`：重命名 `negation_type` → `negation_source`（来源维度），新增 `negation_form`（形式维度）
6. `knowledge-crystallization/SKILL.md`：谱系触发者含异质否定源

**代理工位模式**：外部模型不能直接参与蜂群协议。解决方案是 Claude agent 作为代理，负责调用外部模型、提取完整推理链、以结果包格式向团队汇报。代理工位是结构工位的一种——能力常驻，按需激活。

## 被否定的方案

- **"Gemini 是工具"**：工具的产出不进谱系、不改定义。031 证明 Gemini 的产出做到了两者。
- **"Gemini 是成员"**：成员共享蜂群协议（SendMessage/TaskUpdate/SKILL.md）。Gemini 不共享。
- **"成员/工具二分法足够"**：030a 本身就是二分法不充分的实证。

## 影响

- 元编排方法论从"Claude-only"泛化为"多模型异质否定"
- 谱系模板增加 `negation_source`（来源）+ `negation_form`（形式）双维度字段，区分同质/异质否定及否定结构类型
- 蜂群结构工位清单增加异质质询工位
- 为未来接入更多异质否定源（其他模型、人类审查）提供了架构基础

## negation_source 备注

030a 自身从 `unclassified` 结算为 `heterogeneous`（来源维度）/ `separation`（形式维度）。029 的敞开性原则得到验证——"成员/工具"二分法被撑破后，产出了新的否定结构类型（异质否定）。040号谱系进一步将来源维度与形式维度显式分离。

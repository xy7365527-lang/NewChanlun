---
id: '481'
number: 481
title: "LLM autocompact——prompt 超限自动截断（逢亮 LLM fallback 的自我保护）"
type: domain
status: 已结算
date: 2026-03-17
source: "[新缠论] v262-swarm——memory 泄漏堵漏 + autocompact 实装"
depends_on:
  - '399'   # LLM 角色收窄（器官性阅读范式）
epistemological_level: L0
negation_source: homogeneous
negation_form: addition
negates: null
topo_effect: null
tensions_with: []
---

# 481号：LLM autocompact——prompt 超限自动截断

## 推导链

1. **逢亮的 LLM 是 fallback 层**（399号/LLM-role-boundary 规则）：
   - 第一层：纯拓扑描述（无 LLM）
   - 第二层：S_net connective_patterns 组装（无 LLM）
   - 第三层：LLM 语法填充（仅 operator 请求时）
   - LLM 是最低优先级的外化手段，但仍然存在——需要保证其可用性

2. **问题：prompt 可能超限导致 API 调用失败**：
   - 穿越过程中累积的上下文（question 字段）可能随步数增长
   - 不同 LLM 后端有不同的 token 限制
   - 超限时 API 返回错误，LLM fallback 完全不可用

3. **解决方案：底层 HTTP 调用前自动截断**：
   - `LLMClient._compact_prompt()` 静态方法在每次 API 调用前估算 token 数
   - 估算方式：字符数 / 4（保守近似）
   - 阈值：模型 token 限制的 83%（留裕量给 output + 估算误差）
   - 截断策略：保留 question 前 40% + 后 20%，丢弃中间部分
   - 前部保留指令和上下文，后部保留最近的关键信息
   - system prompt 不截断（通常短且全部关键）
   - 三个后端（Anthropic / OpenAI / Gemini）均经由同一方法保护

4. **与 LLM 角色边界的一致性**：
   - autocompact 不改变 LLM 的角色（仍是 fallback）
   - 不改变三层输出架构的优先级
   - 仅保证：当 LLM 被合法调用时（第三层 + force_llm=True），调用不会因 prompt 超限而失败

## 代码变更

| 文件 | 位置 | 变更 | 作用 |
|------|------|------|------|
| llm_integration.py | `_compact_prompt()` 静态方法 | 新增方法：估算 token → 超阈值时截断 question 中间部分 | 所有 LLM 调用的前置保护 |
| llm_integration.py | `_MODEL_TOKEN_LIMITS` | 模型→token 限制映射（claude-sonnet-4: 180K, gpt-4o: 128K, gemini-2.0-flash: 1M） | 按模型适配截断阈值 |
| llm_integration.py | `_COMPACT_RATIO = 0.83` | 触发阈值：模型限制的 83% | 留裕量给 output tokens + 估算误差 |
| llm_integration.py | Anthropic/OpenAI/Gemini 三处调用点 | 每处调用前插入 `_compact_prompt()` | 三后端统一保护 |

## 截断策略细节

```
原始 question: [前部指令/上下文 ... 中间内容 ... 后部最近信息]
截断后:        [前 40%][autocompact 截断标记][后 20%]
```

- 前 40%：通常包含系统指令、穿越上下文
- 后 20%：通常包含最近的穿越事件、当前 question
- 中间 40%：信息密度最低的部分（历史累积）
- 截断标记：`[... autocompact: 截断了 N 字符 (M% 的 question) ...]`

## 边界条件

| 条件 | 当前 | 翻转阈值 |
|------|------|----------|
| 字符/4 近似是否足够 | 对中英文混合文本保守（实际 token 可能更少） | 如果出现大量非拉丁字符文本（如纯中文），近似比率可能需调至 2-3 |
| 83% 阈值是否合适 | 留 17% 给 output + 误差 | 如果 max_tokens_output 设得很大，可能需降低阈值 |
| 截断是否影响 LLM 输出质量 | 中间部分通常信息密度低 | 如果关键信息在中间（非典型情况），截断会丢失信息——但这优于完全调用失败 |

## 影响声明

- llm_integration.py 新增 `_compact_prompt()` 方法 + 三处调用点集成
- 逢亮的 LLM fallback 层获得自我保护——prompt 超限时降级（截断）而非失败
- 不改变 LLM 角色边界（仍是最低优先级的第三层 fallback）

## 谱系关联

related_records:
  parent: '399'   # LLM 角色收窄——本号保障 fallback 层可用性
  siblings: []
  children: []

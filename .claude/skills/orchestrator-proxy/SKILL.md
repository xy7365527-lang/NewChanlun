---
name: orchestrator-proxy
description: >
  Gemini 编排者代理决策协议。当人类编排者将"选择"和"语法记录"类决断委托给 Gemini 时激活。
  Gemini 以 decide() 模式运行，产出结构化决策；人类保留异步审计和 INTERRUPT 覆盖权。
genealogy_source: "030a"
---

# 编排者代理决策协议

> **底层模型（2026-06-23 换装）**：`decide()` 的异质源已从 Google Gemini 迁移到 **OpenAI GPT-5.5**
> （`gpt-5.5-pro` 主 / `gpt-5.5` 降级，reasoning effort = `xhigh`）。CLI `python -m newchan.gemini decide`
> 不变（`gemini` 是工位角色名，非模型名）。触发：Gemini API 429 不可用，编排者指令换装。

## 本体论位置

030a 号谱系结算了异质否定源的位置：它既不是成员也不是工具，是否定的另一种显现形式
（底层模型现为 OpenAI GPT-5.5，下文"Gemini"均指此异质质询工位的角色名）。

本 skill 在此基础上增加第二种运作模式：

| 模式 | 功能 | system prompt |
|------|------|---------------|
| challenge / verify | 异质否定质询 — Gemini 找问题 | 质询者（否定立场） |
| decide | 编排者代理 — Gemini 做选择 | 决策者（中立立场） |

两种模式共享同一运行时（`gemini_challenger.py`），但 system prompt 和输出格式不同。质询模式寻找否定；决策模式在选项间做出有理由的选择。

-----

## 四分法路由协议

蜂群中产生的决断需求分为四类。只有后两类路由到 Gemini decide()。

| 类型 | 判据 | 路由 |
|------|------|------|
| **定理** | 从定义出发可逻辑推导，无需选择 | Claude 自动结算 |
| **行动** | 执行层操作（写代码、写文件、调工具） | Claude 自动执行 |
| **选择** | 多个合理选项，需权衡利弊做出取舍 | Gemini decide() |
| **语法记录** | 领域语法的记录方式需要决断（命名、格式、分类） | Gemini decide() |

### 路由判断规则

1. 先尝试归为定理：能否从 `.chanlun/definitions/` 中的定义直接推导？能 → 定理，不到 Gemini。
2. 再判断是否为行动：是否是纯执行层操作？是 → 行动，Claude 直接做。
3. 剩余的判断是选择还是语法记录：涉及"怎么记录/命名/分类" → 语法记录；涉及"选哪个方案/走哪条路" → 选择。
4. 两者都路由到 Gemini decide()，区别仅在上下文构建时的侧重点。

-----

## 决策上下文构建

调用 Gemini decide() 前，**必须**构建上下文文件。没有上下文的 decide 调用是被禁止的。

### 上下文文件结构（写入 `/tmp/decide-ctx.md`）

```markdown
# 决策请求

## 主题
[什么需要决断]

## 类型
[选择 | 语法记录]

## 选项
### 选项 A: [名称]
- 描述: ...
- 利: ...
- 弊: ...

### 选项 B: [名称]
- 描述: ...
- 利: ...
- 弊: ...

[可有更多选项]

## 相关定义
[从 .chanlun/definitions/ 提取的相关定义，逐条引用]

## 相关谱系
[从 .chanlun/genealogy/ 提取的相关记录摘要，含编号]

## 约束
[已知的硬约束 — 哪些选项已被排除、为什么]
```

### 构建规则

1. **定义必须是最新版本** — 从 `.chanlun/definitions/` 读取，不凭记忆。
2. **谱系必须包含相关的已结算和未结算记录** — 从 `.chanlun/genealogy/settled/` 和 `.chanlun/genealogy/pending/` 读取。
3. **选项必须穷尽已知可能** — 不能只给 Gemini 两个选项而隐藏第三个。
4. **利弊必须基于事实** — 不能编造利弊来引导 Gemini 的选择。
5. **语法记录类决策**额外提供：现有命名惯例、已有的同类记录格式示例。
6. **产出大小 ≤ 8KB**（145号下游推论）— Gemini 直接回复不超过 8KB。超出部分写入 `tmp/` 附件文件，正文仅保留摘要 + 附件路径引用。理由：Gemini 回复嵌入 Claude agent 上下文，过大回复消耗 context window。

-----

## 调用方式

```bash
python -m newchan.gemini decide "<subject>" \
  --context-file /tmp/decide-ctx.md --verbose
```

> **运行时状态（2026-05-24 更新，pending-005 结算）**：`decide` 已在 CLI 完整实装——`newchan.gemini.__main__` 的 `choices` 含 `{challenge,verify,decide,derive}`，`modes.py:decide()/decide_with_tools()` + `registry.py` decide ModeConfig 全谱就绪。本 skill 调用部分**已激活**（曾误标"待激活"=声明萎缩，反向缺口，见 pending-005）。

-----

## 决策输出格式

Gemini decide() 的输出必须包含以下五个字段：

```yaml
decision: "[明确选择 — 选了哪个选项]"
reasoning_chain:
  - "[推理步骤 1]"
  - "[推理步骤 2]"
  - "..."
boundary_conditions:
  - "[何时应推翻此决策 — 条件 1]"
  - "[条件 2]"
risks:
  - "[可能的问题 1]"
  - "[问题 2]"
trace: "orchestrator-proxy/decide | <subject> | <timestamp>"
```

### 输出处理

Claude agent 收到 Gemini 的决策输出后：

1. **格式验证** — 五个字段是否完整。不完整则重新调用。
2. **合理性检查** — reasoning_chain 是否引用了上下文中的定义和谱系。未引用 → 决策可能脱离体系，标记警告。
3. **记录** — 将决策写入当前 session 的中断点章节。
4. **传递** — 以结果包格式向请求方汇报。

-----

## 人类 INTERRUPT 权

人类编排者可随时覆盖 Gemini 的决策。INTERRUPT 不是错误，是系统正常运作的一部分。

### INTERRUPT 流程

1. 人类编排者发出覆盖指令（自然语言即可）。
2. Claude agent 执行覆盖，采用人类的决策。
3. 被覆盖的决策写入谱系：

```yaml
type: 决策覆盖
status: 已结算
original_decision: "[Gemini 的原始选择]"
override_decision: "[人类的选择]"
override_reason: "[人类给出的原因，如未给出则标注'未说明']"
negation_source: human
negation_form: "[视情况标注]"
```

4. 覆盖记录写入 `.chanlun/genealogy/settled/`（覆盖即结算，不经过生成态）。

### INTERRUPT 的意义

- 覆盖积累到一定数量后，可回溯分析：Gemini 在哪类决策上系统性偏离人类判断？
- 这是校准异质否定源的数据来源。
- 覆盖不意味着 Gemini 错了 — 可能是人类有 Gemini 不知道的上下文。

-----

## 与异质否定质询的区分

|  | challenge / verify | decide |
|--|-------------------|--------|
| **目的** | 找问题（否定） | 做选择（决断） |
| **立场** | 对抗性 — 寻找盲区 | 中立性 — 权衡选项 |
| **system prompt** | 质询者 | 决策者 |
| **输出** | ChallengeResult（否定 + 推理链） | DecideResult（选择 + 推理链） |
| **后续** | 否定成立 → 写谱系；不成立 → 报告误判 | 决策 → 执行（除非 INTERRUPT） |
| **触发者** | 任何 agent 通过 `/challenge` | 编排者路由（四分法） |

两种模式不应混用。如果在 decide 过程中 Gemini 发现了定义矛盾，应中止 decide 并转为 challenge 流程。

-----

## 边界条件

本 skill 的协议在以下条件下应被重新审视：

- Gemini 的 INTERRUPT 覆盖率超过 50% → 说明路由判断或上下文构建有系统性问题
- 出现第三种 Gemini 运作模式的需求 → 需要扩展本协议
- 人类编排者决定收回委托 → 本 skill 停用，回到"等待人类"模式
- 接入新的异质否定源（非 Gemini） → 需泛化本协议为多源版本

-----

## 核心备忘

- decide 模式是编排者的代理，不是编排者本身。人类保留最终决断权。
- 没有上下文的 decide 调用是被禁止的。
- INTERRUPT 是正常运作，不是异常。
- 质询（找问题）和决策（做选择）使用不同的 system prompt，不可混用。
- decide() 运行时已实装（`newchan.gemini`，2026-05-24 经 pending-005 验证），本 skill 可实际调用。

-----

## 当前限制（353号消费断裂标注，2026-05-24 经 pending-005 更新）

| 声明 | 当前状态 | 路径 |
|------|---------|---------|
| decide 子命令（CLI 调用） | **已实现**——`newchan.gemini` CLI 含 decide，modes/registry 全谱就绪 | `python -m newchan.gemini decide "<subject>" --context-file <ctx>` |
| escalate_choice 事件**自动**触发 | **平台仍不支持**——Claude Code 无语义事件总线 | Lead 在 `/escalate` 流程中手动识别"选择/语法记录"类型并手动调用 decide |

**剩余限制澄清**：decide 子命令本身已就绪（反向缺口已闭合）；尚未解决的是 escalate_choice → decide 的**自动事件路由**（平台无语义事件总线，正向缺口，与 016号同模式）。即"能调用"已成立，"自动被触发"仍需 Lead 手动识别四分法类型后调用。dispatch-dag.yaml 的 `escalate_choice → gemini decide` 当前为手动触发。

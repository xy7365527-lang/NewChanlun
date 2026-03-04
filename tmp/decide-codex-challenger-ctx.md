# Gemini Decide 上下文：Codex Challenger 架构决策

## 决策请求

蜂群编排者决断引入 OpenAI Codex (gpt-5.2-codex) 作为代码层异质审查者。请审查以下设计方案，指出：
1. 架构设计中的矛盾或不一致
2. 与现有 Gemini challenger 的对称性是否合理
3. 模式设计（review/diagnose）是否充分
4. 存在论位置是否正确——Codex 作为"代码层异质否定源"是否与蜂群的否定框架一致
5. 可能遗漏的边界条件

## 当前异质质询架构

### Gemini challenger（概念/数学层）
- 4 个模式：challenge（质询）、verify（验证）、decide（决策代理）、derive（形式推导）
- 调用：google genai SDK → gemini-3.1-pro-preview
- 触发：/challenge 命令 + spec/theorems 文件变更 + 选择/语法记录路由
- 代理模式：编排者代理（选择/语法记录类决断代替人类决策）

### Claude challenger（反向质询）
- 癔症话语位置——对 Gemini 产出再质询
- 不提供知识，只制造裂隙
- 手动触发

### 当前代码审查
- code-reviewer / python-reviewer 都是 Claude 同质审查
- 没有异质代码审查

## 提案的 Codex challenger 设计

### 位置
- conditional skill（与 gemini-challenger 对称）
- 代码层异质否定源（外部对象否定）

### 模式
- `review`：代码审查（正确性/性能/安全/定义忠实度）→ 对标 Gemini challenge
- `diagnose`：严格诊断（失败现象→根因→修复方案）→ 对标 Gemini verify+decide
- 不做 derive（数学推导留给 Gemini）

### 技术实现
- OpenAI Responses API：client.responses.create()
- 主模型 gpt-5.2-codex，fallback gpt-5.2
- reasoning_effort: "extra_high"
- 文件结构对称 Gemini：engine.py + registry.py + modes.py + __main__.py

### 触发条件
- /code-review 命令后自动触发
- Lead 手动调用

## 需要 Gemini 判断的问题

1. **对称性问题**：Gemini 有 4 模式（challenge/verify/decide/derive），Codex 只有 2 模式（review/diagnose）。这种不对称是合理的（域不同），还是应该补全？

2. **编排者代理问题**：Gemini 在 decide 模式中充当编排者代理（替代人类做选择/语法记录决断）。Codex 是否也应该有类似的代理模式？比如：代码层面的"选择"（该用哪种数据结构、该用哪种算法）？

3. **claude-challenger 的扩展**：当前 claude-challenger 只对 Gemini 产出再质询。引入 Codex 后，claude-challenger 是否也应该对 Codex 产出再质询？还是 Codex 和 Gemini 互相质询就够了？

4. **触发条件充分性**：`/code-review` 后自动触发 + 手动调用——是否遗漏了应该触发的场景？比如：测试失败时自动触发 diagnose？代码变更超过 N 行时自动触发 review？

5. **定义忠实度审查**：review 模式的核心之一是"实现是否忠实反映定义"。但 Codex 不了解缠论定义。这是否意味着上下文中必须包含相关定义？还是说这个维度应该留给 Gemini？

## 谱系上下文

- 030a号：异质否定源引入
- 041号：编排者代理协议
- 062号：Gemini 再质询
- 075号：结构能力从 teammate 转为 skill
- 093号：五约束（包括约束4：异质验证）

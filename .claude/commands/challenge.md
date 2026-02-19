# /challenge — 异质否定质询

调用外部模型（Gemini）通过 MCP/Serena 自主导航代码库，对指定目标进行异质否定质询。

## 用法

```
/challenge <subject>
```

`subject` 是质询目标，用自然语言描述。例如：
- `/challenge RecursiveOrchestrator 的 L* 裁决逻辑是否正确`
- `/challenge 中枢定义在递归层级间是否自同构`
- `/challenge 线段构造的边界条件处理`

## 执行流程

1. **构建上下文** — 读取相关谱系（pending/ + settled/）、定义（definitions/）、规格（docs/spec/），写入临时上下文文件
2. **调用 Gemini** — `python -m newchan.gemini_challenger challenge "<subject>" --tools --verbose --context-file <ctx>`
3. **提取推理链** — 解析 Gemini 的完整推理过程（工具调用 + 中间思考 + 结论）
4. **判定否定** — 对 Gemini 的否定执行简化质询（定义回溯 + 反例构造 + 推论检验）
5. **产出** — 否定成立 → 写谱系 pending/；否定不成立 → 报告误判

## 输出格式

```markdown
## 异质质询结果

**目标**: [subject]
**模型**: [gemini model name]
**工具调用**: [N] 次

### Gemini 推理链
[完整推理链：每步的工具调用 + 思考 + 发现]

### 质询结论
[Gemini 的否定内容]

### 判定
[成立 / 不成立 + 理由]

### 谱系动作
[写入 pending/XXX.md / 无（误判）]
```

## 前置条件

- `GOOGLE_API_KEY` 在 `.env` 中设置
- Serena MCP server 可用（`.serena/serena_config.yml` 已配置）
- `uvx` 可用（用于启动 Serena）

## 谱系关联

- 030a: Gemini 位置问题（异质否定源的本体论位置）
- 031: Move 价格范围语义（第一个异质否定实证）

## 注意

- 异质否定发现的是同质质询无法覆盖的系统性盲区
- Gemini 的推理链必须完全透明——不只看结论，要看它怎么推导的
- 否定成立不等于应该被接受——最终决断是编排者的事
- 每次调用消耗 Gemini API 配额，按需使用

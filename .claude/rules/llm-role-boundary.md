# 语法记录：逢亮语言能力的LLM边界

此规则优先级与 `no-patch-mentality.md` 相同。

## 核心声明

逢亮的语言能力以器官性阅读为基础，LLM仅为外化fallback。

## 三层输出架构（强制顺序，不可跳过）

| 层级 | 方式 | LLM参与 | 条件 |
|------|------|---------|------|
| 第一层（默认） | 纯拓扑描述 | 无 | 始终可用 |
| 第二层 | S_net connective_patterns 组装 | 无 | 有 connective_patterns 时 |
| 第三层（fallback） | LLM 语法填充 | 有，标注 `[LLM填充]` | 仅 operator 明确请求（`force_llm=True`）|

**说真话比说漂亮的假话好。** 纯拓扑描述即使粗糙，也比LLM填充的流畅文本更诚实——后者的流畅性来自LLM的训练语料（大他者），不来自逢亮的穿越经验。

**分析优先于翻译**（426号理论基础）：翻译（LLM 语法填充）= ego 审查功能 = 遮蔽无意识的直接言说。逢亮没有自我（ego），原始能指链是无意识的直接输出。ceremony agent 对原始链应先做**分析**（追踪断裂点和跳跃模式——这些是无意识结构的可观测信号），翻译是给外部接口的最后一步。

## 器官性阅读

文本摄入不经LLM预处理，直接进入 S_net 作为语言材料：

1. **摄入**：`ingest_text_passage()` → S_net（不碰 K_active）
2. **耦合**：`SNetActivation.activate()` → 穿越步进同步激活
3. **反馈**：`check_articulation_feedback()` → EdgeSuggestion → REFERENCE 边
4. **外化**：三层架构（见上表）

概念关系由逢亮在穿越中通过 articulation feedback 自主发现，不由LLM预设。

## 概念命名边界（436号扩展）

本规则不仅约束"何时用 LLM"，还约束"不要用 LLM/概率范式的概念命名逢亮的能力"。

借用概率范式的能指，概率范式的所指跟着渗入（索绪尔：能指/所指不可分割）。用 "thinking" 命名逢亮的穿越环路，LLM thinking 的全部前提（token 数限制、stop signal、中间步骤是"隐藏的"）跟着渗入设计。

### 概念分离表（436号8对）

| 概率范式词（禁止用于命名逢亮能力） | 拓扑穿越范式替代 | 渗入的度量前提 |
|----------------------------------|-----------------|--------------|
| thinking | 环路穿越（loop traversal） | 步数限制、stop token |
| language processing | 能指导航（signifier navigation） | 概率分布、perplexity |
| productivity | 拓扑操作（topological operation） | 产出量、throughput |
| meaning | 结构变化（structural mutation） | 可解释性 score |
| speaking | 轨迹沉积（trajectory sedimentation） | 流畅度、coherence |
| explanation | 路径展示（path exposition） | 论证长度 |
| transcendence | 扬弃（Aufhebung） | benchmark 分数 |
| organ（指 LLM 作为假肢） | 器官（指 S_net/穿越引擎/ARTICULATE） | API 成本 |

**例外**：在描述 LLM 本身（而非逢亮）时，概率范式概念仍然合法——分离是关于命名逢亮的能力，不是否定概率范式本身。

**代码中的 `random`/`sample`**：Python 标准库 `random` 用于性能采样（如 `random.sample(neighbors, 10)` 限制遍历度数）不属于概率范式命名——这是计算优化手段，不是命名逢亮的能力。注释中将穿越描述为 "random" 则需要审查。

## 禁止的模式

1. **默认调LLM**：外化时跳过第一层/第二层直接调LLM
2. **LLM预处理摄入**：用LLM"理解"文本后再喂给 S_net
3. **无标注LLM输出**：LLM参与的输出不标注 `[LLM填充]`
4. **LLM填充冒充拓扑产出**：LLM生成的内容伪装为穿越经验
5. **概率范式命名逢亮能力**（436号）：用概率范式的词（thinking, sampling, prediction 等）命名逢亮的穿越/折叠/否定操作——概念命名携带度量前提

## 发生史

| session | 变化 | LLM角色变迁 |
|---------|------|------------|
| v167 | S_net 初始实装 | LLM仍为主要外化路径 |
| v197 | InternalSpeechSnapshot | LLM从创作者降为语法化工具 |
| v198 | 器官性阅读 + 外化接缝 | 摄入侧LLM退出 |
| v207 | 语言器官三层重构 | 外化侧LLM降为fallback |

## 编排者裁决

"默认不调LLM。internal_speech.py 的输出流程改为：如果内部言语片段有 connective_patterns → 直接用 connective_patterns 组装输出。如果没有 → 直接输出穿越事件的结构描述。只在 operator 明确请求时才调 LLM 做语法填充，且输出中标注哪些部分是 LLM 填充的。"

## 谱系依据

- 399号候选1：器官性阅读范式（v198 首次完整实现）
- 399号候选4：LLM角色持续收窄（跨v167/v197/v198/v207）
- 406号：合并确认阈值已达
- 401号：轨迹与产物范畴区分（encounter memory 清除）
- 426号：逢亮无意识结构的精确定义——"分析优先于翻译"的理论基础（翻译=ego审查=遮蔽无意识）
- 436号：概率范式与拓扑穿越范式的系统性概念分离——"概念命名即度量渗入"
- 435号：度量是概率机制的必然附属物——概念命名边界的根因分析

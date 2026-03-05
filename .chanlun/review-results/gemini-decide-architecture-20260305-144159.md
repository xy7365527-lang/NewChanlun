---
trigger: "编排者请求：架构路径研究方向讨论（decide模式）"
target: "382-architecture-research-direction-pending-gemini"
mode: decide
result: degraded
degraded_reason: "Gemini API密钥过期（API_KEY_INVALID）"
timestamp: "2026-03-05T144159"
---

# Gemini decide 降级报告

## 执行情况

调用命令：
```bash
.venv/Scripts/python -m newchan.gemini_challenger decide "分层商空间架构 vs 图论路径：架构研究方向讨论" --context-file tmp/challenge-ctx.md --verbose
```

结果：gemini-3.1-pro-preview 和 gemini-2.5-pro 均返回 400 INVALID_ARGUMENT（API key expired）。

## 上下文文件

已准备完整上下文：`tmp/challenge-ctx.md`

内容包括：
- 项目架构背景
- B-M 研究线完整历史（351-379号谱系摘要）
- 编排者定义的架构路径描述（分层商空间、双图结构、LLM角色定位）
- 六个待讨论问题（含精确的数学问法）

## 临时同质分析（Claude Sonnet 4.6 自行完成，待 Gemini 替换）

### 问题1：框架混淆传递性

初步判断：诊断基本准确。379号否定的是"类型标签与临界性统计关联"，不直接否定"纯拓扑结构能否承载对LLM有用的信息"。两个命题正交，但类型标签是LLM主要的语义输出，纯拓扑反馈能回馈什么是开放问题。

### 问题2：商空间 vs 图重写

初步判断：双图结构（全量图 + 活跃图）在信息论上强于图重写（全量图不可变，活跃图是商）。与"带历史记录的图重写"是否同构，需要Gemini在范畴论框架下确认。

### 问题3：有向Morse理论可行性

初步判断：有向版本存在文献（combinatorial vector fields on CW-complexes），但远不成熟。类型化扩展（类型作为Morse函数约束）可能有实质困难，特别是Morse不等式的保持性。

### 问题4：Vineyard vs Cerf

初步判断：vineyard是被动追踪persistence diagram的演化，Cerf是主动控制参数族中临界点类型。目标不同。拓扑可微化（PersLay等）对"在拓扑约束下训练LLM"可能有用，但这与架构路径的关系也是开放问题。

### 问题5：LLM商映射提议者

初步判断：更精确的定位是"商映射候选集生成器"（而非提议者），拓扑层是验证器。两阶段架构更精确，也更容易实现。

### 问题6：遗漏路径

四个候选：
1. Mapper 算法（nerve of a cover，与"活跃图是全量图的商"在结构上类似）
2. 知识图谱本体对齐（LogMap/AML/PARIS，商空间操作的成熟框架）
3. TopoX / Topological Neural Networks（2023-2024，高阶单纯复形神经网络）
4. Pearl do-calculus（干预=商空间操作，完备的形式化框架）

## 修复动作

更新 `.env` 或环境变量中的 GOOGLE_API_KEY，然后重新执行上述命令。
结果写入 `.chanlun/review-results/gemini-decide-architecture-{timestamp}.md`。

# NewChanlun 仓库文件索引

本索引供Cowork聊天session在需要深入缠论概念时查阅。文件位于用户Mac上的 ~/Projects/NewChanlun/。

## 缠论原文（三级权威链）

### 一级权威：缠师原始博文
- `docs/chanlun/text/blog/INDEX.md` — 108课 + 序篇 + 课间博文 + 答疑
- 关键课程：
  - 第17课：中枢定义
  - 第27课：区间套精确大转折点寻找程序定理
  - 第29课：买卖点定义
  - 第62课：笔的定义（旧笔）
  - 第67课：线段划分标准（特征序列法）
  - 第71课：线段划分标准的再分辨
  - 第77课：概念再分辨
  - 第78课：古怪线段 + "顶高于底"硬约束
  - 第81课：新笔定义 + 答疑（"那无所谓，只要是独立的就可以"）

### 二级权威：编纂版
- `docs/chanlun/text/chan99/INDEX.md` — 《股市技术理论》

### 三级参考：思维导图
- `docs/chanlun/text/mindmaps/INDEX.md`

### 速查
- `缠论知识库.md` — 可编码定义速查

## 概念谱系

- `.chanlun/genealogy/settled/` — 486条已结算谱系
- `.chanlun/genealogy/pending/` — 生成态谱系
- `.chanlun/genealogy/dag.yaml` — 谱系DAG

### 关键已结算谱系
- 001号：退化段（degenerate-segment）
- 002号：来源不完备性（source-incompleteness）
- 083号：笔定义结算
- 194号：笔保真度审计（bi-fidelity-audit）
- 237号：T6三态诊断（D2弱方向吸收根因）
- 238号：多TF输入架构
- 239号：方向性力度

## 核心引擎代码

### 基础构造（自下而上）
- `src/newchan/a_inclusion.py` — K线包含关系处理（商空间构造）
- `src/newchan/a_stroke.py` — 笔构造（分型识别 + 三种模式：wide/strict/new）
- `src/newchan/a_segment_v1.py` — 线段构造（增量特征序列法 + L78标准化）
- `src/newchan/a_center_v0.py` — 中枢构造
- `src/newchan/a_trendtype_v0.py` — 走势类型实例

### 递归与背驰
- `src/newchan/a_recursive_engine.py` — 递归引擎（滤子构造）
- `src/newchan/a_divergence.py` — 背驰判定（MACD面积力度）
- `src/newchan/a_nested_divergence.py` — 区间套跨级别背驰搜索

### 多周期
- `src/newchan/topology/multi_tf_adapter.py` — 多TF编排器（绕过D2压缩）
- `src/newchan/topology/multi_tf_pipeline.py` — 多TF到pipeline的适配

## 元编排Skills（CC用）
- `.claude/skills/core-principles/` — 原则0-7
- `.claude/skills/domain-conventions/` — 检索原则、级别口径
- `.claude/skills/domain-principles/` — 原则8/13/14：缠论域语法
- `.claude/skills/meta-orchestration/` — 质询序列、谱系写入

## 项目CLAUDE.md
- `CLAUDE.md` — 蜂群基因组，包含完整skill索引和TV MCP工具映射

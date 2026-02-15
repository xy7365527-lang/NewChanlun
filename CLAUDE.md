# CLAUDE.md

## Language
Always respond in Chinese-simplified (简体中文).

## 缠论资料入口（本仓库）

- **速查/可编码定义**：`缠论知识库.md`
- **缠师原始博文（一级权威）**：`docs/chanlun/text/blog/INDEX.md`（108课 + 序篇 + 课间博文 + 答疑）
- **《股市技术理论》编纂版拆分**：`docs/chanlun/text/chan99/INDEX.md`
- **两份思维导图（文字版）**：`docs/chanlun/text/mindmaps/INDEX.md`
- **资料总览**：`docs/chanlun/README.md`

## 来源权威性（三级权威链）

缠论原文的权威性按以下顺序递减：

1. **缠师原始博文**（108课 + 课间答疑 + 附加文章如《忽闻台风可休市》）— **最终权威**
   - 仓库路径：`docs/chanlun/text/blog/INDEX.md`
   - 来源：https://www.fengmr.com/chanlun.html （枫之羁绊，原文+配图+答疑）
2. **《股市技术理论》编纂版**（`docs/chanlun/text/chan99/`）— 本仓库当前主要参考
   - 这是第三方从博文编纂而成的书籍，**可能有遗漏或编排差异**
   - 已知遗漏：第67课（线段划分标准）、第71课（线段再分辨）、第77课、第78课（古怪线段）、《忽闻台风可休市》（新笔定义）等
3. **思维导图 / 第三方总结**（`docs/chanlun/text/mindmaps/`）— 辅助理解，不作为定义依据

**原则**：当层级间有出入时，以更高层级为准。当前仓库主要依赖第2级，因此在涉及古怪线段、新笔定义等博文专有内容时，**必须回溯原始博文**而非仅依赖编纂版。

### 已知缺口（已补录 ✓）

以下内容曾缺失于编纂版，现已通过博文收录补齐（`docs/chanlun/text/blog/`）：

- ✅ 第67课：线段划分标准（特征序列法完整定义）
- ✅ 第71课：线段划分标准的再分辨（特征序列包含关系严格性）
- ✅ 第77课：一些概念的再分辨
- ✅ 第78课：继续说线段的划分（古怪线段 + "顶高于底"硬约束）
- ✅ 《忽闻台风可休市》：新笔定义（包含在第81课页面中）

> 相关谱系：`.chanlun/genealogy/pending/001-degenerate-segment.md`、`.chanlun/genealogy/pending/002-source-incompleteness.md`

## 检索/引用原则（避免 prompt is too long）

- **不要**整本引用 `docs/chanlun/mineru/**/full.md` 或 PDF；优先只读取拆分后的小文件（`docs/chanlun/text/**`）。
- 当用户提问涉及定义/定理/流程时：先在仓库内检索关键词，只打开最相关的 1-3 个小文件片段再回答。
- 若可用检索工具：优先用它在 `docs/chanlun/` 下搜索并按需读取，尽量不要让用户手工提供具体路径。
- **权威性**：遵循上述三级权威链。编纂版与原始博文有出入时，以博文为准。

## 级别口径（本项目关键约束）

- **级别 = 递归层级（level_id）**：从 1分钟K线为底座无限向上递归构造走势类型/中枢。
- **禁止**用"日线/30分/5分"等时间周期替代级别定义（多周期下钻最多只能作为展示/验算手段）。

## 元编排规则

本项目使用元编排 v2 方法论（ECC 工程底座 + 概念层守卫）。

### 核心原则
1. **概念优先于代码。** 定义不清楚时不写代码。
2. **不绕过矛盾。** 见 `~/.claude/rules/no-workaround.md`。
3. **所有产出必须可质询。** 见 `~/.claude/rules/result-package.md`。
4. **谱系必须维护。** 每次矛盾处理后写入 `.chanlun/genealogy/`。
5. **定义变更必须通过仪式。** 使用 `/ritual`，不直接编辑定义文件。

### 知识仓库映射
元编排中的 `knowledge/` 在本项目中对应：
- 速查定义：`缠论知识库.md`
- 域对象 schema：`definitions.yaml`
- 规则规范：`docs/spec/`（segment_rules_v1.md, zhongshu_rules_v1.md, move_rules_v1.md 等）
- 原文参考：`docs/chanlun/text/chan99/`（编纂版，已知不完整；完整原文见三级权威链）

### 谱系目录
- `.chanlun/genealogy/pending/` — 生成态矛盾
- `.chanlun/genealogy/settled/` — 已结算记录

### 可用命令
- `/ceremony` — 创世仪式（会话开始时执行）
- `/inquire` — 四步质询序列
- `/escalate` — 矛盾上浮
- `/ritual` — 定义广播仪式
- `/plan` — 实现规划（ECC，受元编排约束）
- `/tdd` — 测试驱动开发（ECC，受元编排约束）
- `/code-review` — 代码审查（ECC，受元编排约束）

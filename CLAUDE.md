# CLAUDE.md

## Language
Always respond in Chinese-simplified (简体中文).

## 缠论资料入口（本仓库）

- **速查/可编码定义**：`缠论知识库.md`
- **《股市技术理论》拆分原文**：`docs/chanlun/text/chan99/INDEX.md`
- **两份思维导图（文字版）**：`docs/chanlun/text/mindmaps/INDEX.md`
- **资料总览**：`docs/chanlun/README.md`

## 检索/引用原则（避免 prompt is too long）

- **不要**整本引用 `docs/chanlun/mineru/**/full.md` 或 PDF；优先只读取拆分后的小文件（`docs/chanlun/text/**`）。
- 当用户提问涉及定义/定理/流程时：先在仓库内检索关键词，只打开最相关的 1-3 个小文件片段再回答。
- 若可用检索工具：优先用它在 `docs/chanlun/` 下搜索并按需读取，尽量不要让用户手工提供具体路径。
- **权威性**：两份思维导图为总结/讲解材料；如与《股市技术理论》原文（`docs/chanlun/text/chan99/`）有出入，**以原文为准**。

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
- 原文参考：`docs/chanlun/text/chan99/`

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

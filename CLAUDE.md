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
- **禁止**用“日线/30分/5分”等时间周期替代级别定义（多周期下钻最多只能作为展示/验算手段）。

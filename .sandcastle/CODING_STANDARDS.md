# Coding Standards（NewChanlun）

评审工蜂经 @.sandcastle/CODING_STANDARDS.md 加载本文件；实装工蜂同样遵循。

## 语言与工具链

- 主力 Rust（`rust/` 单 crate workspace）：`cargo fmt` 风格、`cargo clippy` 零新增 warning、`cargo check` 必过；测试纪律见 AGENTS.md（测试指纹/对拍口径）。
- 文档一律简体中文；缠论概念对齐 `.chanlun/definitions/` 正本，不自造口径。

## 纪律（commit 与引用）

- commit message：`<type>(<scope>): #<GitHubIssue号>——<一句话>`；编号只指 GitHub Issue（编号声明行必备）。
- 引用缠论课原文须回查 `docs/chanlun/text/blog/` 正本（行号、逐字、作者归属三查，行内注目视避开）——伪引文有在案实例。
- 代码注释登记口径分歧必须带票号。

## 架构

- 教义裁定分层：缠师原文管语义 / Lean 管边界 / 生产代码只有否决权（AGENTS.md 教义正本节）。
- 同一判断不得有宽严两档实现；同一判定每级跑同一个判定（AGENTS.md 收敛通则/总缝规则）。

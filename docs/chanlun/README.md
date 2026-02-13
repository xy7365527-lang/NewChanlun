## 缠论资料（完整源文件）

本目录用于保存**三份 PDF 的完整内容**（原始 PDF + MinerU 抽取结果），方便后续在仓库内引用/检索。

### 原始 PDF（完整）

位于 `docs/chanlun/pdf/`：

- `缠中说禅-股市技术理论.pdf`
- `思维导图-缠中说禅定理.pdf`
- `思维导图-零基础学缠论.pdf`

### MinerU 抽取结果（完整）

位于 `docs/chanlun/mineru/`：

- `缠中说禅-股市技术理论/`
  - `full.md`：主要文字内容（含图片引用）
  - `images/`：配套图片资源（`full.md` 中 `![](images/...)` 对应这里）
  - `*_origin.pdf` / `layout.json` / `content_list_v2.json` 等：MinerU 原始产物
- `思维导图-缠中说禅定理/`、`思维导图-零基础学缠论/`
  - 这两份思维导图本身主要是**图片**，所以 `full.md` 基本只包含图片引用，核心内容在 `images/` 中。

### 给 AI/Claude Code 的使用建议（避免 prompt 过长）

- 日常问答优先引用仓库根目录的 `缠论知识库.md`（更短、面向实现）。
- 查《股市技术理论》原文时，优先引用已拆分的小文件目录 `docs/chanlun/text/chan99/`（更不容易超长），入口见 `docs/chanlun/text/chan99/INDEX.md`。
- 查两份思维导图时，优先引用文字转写版 `docs/chanlun/text/mindmaps/`，入口见 `docs/chanlun/text/mindmaps/INDEX.md`（必要时再引用图片本身；**若与原文出入以原文为准**）。
- 如果必须使用 MinerU 的 `full.md`，也请按需只引用某一节对应的小文件，而不要一次性整本塞进上下文。


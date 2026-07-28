# AGENTS.md（唯一正本基因组）

**语言**：一律用简体中文回复与产出文档。

**正本声明**（2026-07-28 #517 裁定）：本文件是全 harness 唯一正本入口。Kimi 为主力运行时（K1：编排/文档层主力；实施层 codex/claude 分工不变）。CLAUDE.md 仅留指针；`.claude/rules/`（ECC 注入机制）已归档下线；skills canonical = `.agents/skills/`（`.claude/skills` 为 symlink 投影，`agent/` 为生成物）。

## 项目总纲领

顶层路线图见 `docs/ROADMAP.md`：四大支柱（缠论引擎 / PH 拓扑 / K4 选股 / IBKR 执行）+ 五里程碑（M1 回测验证 → M5 生产加固）。

## 缠论权威链（递减）

1. **缠师原始博文**（`docs/chanlun/text/blog/INDEX.md`，108课+答疑）— **最终权威**；
2. **《股市技术理论》编纂版**（`docs/chanlun/text/chan99/INDEX.md`）— 主要参考，已知遗漏 67/71/77/78 课与新笔定义（已由博文补录）；
3. **思维导图/第三方总结**（`docs/chanlun/text/mindmaps/INDEX.md`）— 辅助，不作定义依据。

层级间有出入以更高层为准。速查：`缠论知识库.md`。

## 协作设施

- **Issue tracker**：GitHub `xy7365527-lang/NewChanlun`（private，`gh` 已认证）；操作口径 `docs/agents/issue-tracker.md`。
- **Triage labels**：`needs-triage` / `needs-info` / `ready-for-agent` / `ready-for-human` / `wontfix`。
- **Domain docs**：单 context——根 `CONTEXT.md` + `docs/adr/`（惰性创建）。
- **纪律族**（docs/agents/）：`generation-constitution.md`（名分四态+现役线名单+开票门）、`delivery-discipline.md`（关票门五子句+开票门+豁免+编号）、`stat-provenance.md`（统计口径两档）。
- **TradingView MCP**（Claude Code 专用，`~/.claude/.mcp.json`）：TV Desktop 须 debug 模式；工具映射与约束见 git 历史版 CLAUDE.md 或 `docs/chanlun/README.md`。

## 记忆（高信号持久事实，continual-learning 维护）

- **偏好**：编排者粘贴 shell 命令时期望逐字执行；破坏性 git 操作前先备份 + 只读预检，且不打印任何密钥值。
- **harness**：Kimi 主力（K1，2026-07-28 裁定）；实施层 = codex/claude CLI（2026-07-26 令，不废）。工位模型分配四档（永久令 2026-07-04 三令）：机械=haiku / 常规=sonnet / 高难=opus / 最难=fable；spawn 必显式传 model，拿不准取低档。
- **验证口径**：不全量重放——验证用靶向/原型级（差异面先全枚举再对拍，秒级）；全量仅留重型验证窗口并照实标注（用户 2026-07-26 裁定，效率纪律）。
- **执行分工**：Claude CLI 执行层——实施票用 sonnet（快），影子评审用 opus（深）；Kimi 只做编排，不下场写码（2026-07-26 用户定）。
- **工作线**：**main 是唯一现役 git 线**（2026-07-28 核：gap3-rework-codex9-fix 已被吸收，main 领先其 260）；`main-rewritten` 是其只读镜像（CI 触发器挂它），main 不推 origin。
- **Stop-Guard**：其注入内容可能反映过期状态（已结算谱系仍列为 pending）；重复执行前先核实际状态。
- **goal 事件**：正式 GOAL_SET/SUPERSEDE 由 Lead 直接 append 到 `.chanlun/goals/events.jsonl`（`scripts/goal_events.py` 只出草稿），随后用 reducer/scan 验证。
- **裁定文档**：编排者钦定的裁定 PDF 先归档到 `docs/formal-chain/` 再登记为 goal 权威。
- **测试指纹**：共享 worktree 取 cargo test 计数时，先核并行线未提交面是否含测试——含则计数被污染（#475 HIGH-4 实锤：1966 vs 干净快照 1957，差 ~9），须注明污染常量或取干净快照；失败集指纹通常不受影响（2026-07-27 在案）。
- **入仓名分**：新文件入仓前先判名分四态（宪法 §1）；会话导出/备份/工具残留一律禁入仓（.gitignore 已立模式）；review-results 是工作草稿，活期绑定票（宪法 §5）。

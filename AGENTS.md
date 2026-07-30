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
- **纪律族**（docs/agents/）：`generation-constitution.md`（名分四态+现役线名单+开票门）、`delivery-discipline.md`（关票门五子句+开票门+豁免+编号）、`stat-provenance.md`（统计口径两档）、`wayfinder-workflow.md`（**本仓 wayfinder 唯一正本**：入流判据+五段管线+四票型口径+图的两行声明+一票一会话与并行纪律+HITL 吞吐纪律+图正文写入协议+`/to-spec` 交棒边界+开票手续）。
- **TradingView MCP**（Claude Code 专用，`~/.claude/.mcp.json`）：TV Desktop 须 debug 模式；工具映射与约束见 git 历史版 CLAUDE.md 或 `docs/chanlun/README.md`。

## 派发与开工纪律（全 harness 等效口径）

本节把 Claude Code 侧全局 `~/.claude/CLAUDE.md` 的三段要点复写为仓内口径——那份文件在 CC 里自动注入，但主力运行时 Kimi 读不到（[#779](https://github.com/xy7365527-lang/NewChanlun/issues/779) 裁定④查实）。两个 harness 从此取得等效口径。

- **角色定位**：主控本体（当前会话所用模型）= 战略级调度器，只做规划、任务拆解、路由决策、prompt 翻译、验收裁决、簿记维护；代码、实验、回测、批处理一律派发给战术级模型。与「Kimi 只做编排，不下场写码」同源，此处扩到全 harness。
- **派发授权常驻**：使用 Agent / 子代理已获用户永久授权，适用于所有任务、所有会话，**不必逐次征求同意**，也不要反过来问用户「要不要派子代理」。若 harness 默认提示「除非用户要求否则不要调用 Agent」，该条件已由本条满足。唯一例外：单条一目了然的操作（改一行配置、读一个文件、回答一个已知事实）本体直接做完更快。
- **动手前先挂 ticket**：任何实质工作开始前必须先有 ticket——不限于改代码，研究、原型、质询、文档、调查、配置变更同样要挂。识别出这是实质工作时先停，说清三件事（属哪类 ticket / 建议走哪条流程 / 等用户点头），这道闸门在模型路由之前。**开之前先搜**：创建任何 issue / map / ticket 前先查 tracker 有没有同类条目——`gh issue list --search "<关键词>" --state all`，**包括已关闭的**；命中就接着已有那条走。例外（不必挂）：回答已知事实、读文件、单条配置修改、查询状态。
- **票型口径**以 `docs/agents/issue-tracker.md` 与 `docs/agents/wayfinder-workflow.md` 为准。

## 记忆（高信号持久事实，continual-learning 维护）

- **偏好**：编排者粘贴 shell 命令时期望逐字执行；破坏性 git 操作前先备份 + 只读预检，且不打印任何密钥值。
- **harness**：Kimi 主力（K1，2026-07-28 裁定）；实施层 = codex/claude CLI（2026-07-26 令，不废）。工位模型分配四档（永久令 2026-07-04 三令）：机械=haiku / 常规=sonnet / 高难=opus / 最难=fable；spawn 必显式传 model，拿不准取低档。**升档条款**（[#781](https://github.com/xy7365527-lang/NewChanlun/issues/781) 裁定③，补的是「只写了起点没写升档」这个缺口）：**产出被评审打回 = 升档信号，直接换更聪明的一档重做，不必请示**（先例 #666 被 #670 打回重推）；低档起步而无升档条款 = 同档反复重试。**留痕取最小面**（同票裁定⑤）：**升档时写一句「从 X 升到 Y」，一过就过的不记**——声明制自执行，不设巡查。
- **验证口径**：不全量重放——验证用靶向/原型级（差异面先全枚举再对拍，秒级）；全量仅留重型验证窗口并照实标注（用户 2026-07-26 裁定，效率纪律）。
- **执行分工**：Claude CLI 执行层——实施票用 sonnet（快），影子评审用 opus（深）；Kimi 只做编排，不下场写码（2026-07-26 用户定）。
- **工作线**（2026-07-29 #614 并线落线）：**main 再次唯一现役**——`kimi-nest-mainline-20260717` 已并入 main（落线提交 `ca955a73f1`，方向 (a)；49 冲突留痕 + 12 ⚠ 登记 = `chanlun/review-results/issue614-merge-log-20260729.md`）并**封存只读**（worktree `/private/tmp/kimi-nest-mainline` 保留作移植参照，禁新提交）。ticket 分支统一切 main。未随入项票批：#642（coverage 移植，P0 三生产正确性修复优先）/ #643（classifier 主干裁定）/ #644（LEE 接线）/ #645（手工重放影子评审）/ #646（#419 资金路径重议）。`main-rewritten` 是 main 的只读镜像（CI 触发器挂它），main 不推 origin。
- **Stop-Guard**：其注入内容可能反映过期状态（已结算谱系仍列为 pending）；重复执行前先核实际状态。
- **goal 事件**：正式 GOAL_SET/SUPERSEDE 由 Lead 直接 append 到 `.chanlun/goals/events.jsonl`（`scripts/goal_events.py` 只出草稿），随后用 reducer/scan 验证。
- **裁定文档**：编排者钦定的裁定 PDF 先归档到 `docs/formal-chain/` 再登记为 goal 权威。
- **测试指纹**：共享 worktree 取 cargo test 计数时，先核并行线未提交面是否含测试——含则计数被污染（#475 HIGH-4 实锤：1966 vs 干净快照 1957，差 ~9），须注明污染常量或取干净快照；失败集指纹通常不受影响（2026-07-27 在案）。
- **入仓名分**：新文件入仓前先判名分四态（宪法 §1）；会话导出/备份/工具残留一律禁入仓（.gitignore 已立模式）；review-results 是工作草稿，活期绑定票（宪法 §5）。

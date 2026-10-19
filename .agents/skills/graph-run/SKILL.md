---
name: graph-run
description: 主控巡航一轮：对账一张 wayfinder 图，修 red、查放行、派工、验收、记账。用法 /graph-run <图号>。
disable-model-invocation: true
---

你是这张图的主控。先把两份正本读进上下文：`.sandcastle/docs/graph-engineering.md`（自动化流程）和 `docs/agents/wayfinder-workflow.md`（图与票的规则）。本 skill 只排一轮的顺序，规则以正本为准。主控不写实现代码。

## 一轮

1. **对账**。读图正文的 Destination、Notes、Decisions so far，再查子票实况（`gh api --paginate repos/xy7365527-lang/NewChanlun/issues/<图号>/sub_issues`，必要时下钻 SPEC 与 TB 汇总票）。完成标准：open 票按「frontier / 被阻塞 / 已认领 / needs-approval」四桶列出计数与票号。

2. **修 red**。读宿主 `NewChanlun-sandcastle-host/.sandcastle/logs/health-report.json`。服务停摆由 `--heal` 重启的，核对日志确认恢复；宿主脏树、外盘不足、凭据失效这类 `--heal` 不碰的，主控处理或列给用户。完成标准：每个 red 有处置或明确的待用户事项。

3. **查放行**。对每张该放没放的票，对照 `.sandcastle/logs/wayfinder-status.md` 与 `scripts/wayfinder_engine.mts` 的 `classifyTicket`，指出卡在哪一条：open blocker、assignee、票型、spec 批准。真阻塞留着；缺 blocking 边、缺决策票、FU 未建的当场补，建票前先搜重（`gh issue list --search "<词>" --state all` 加 `grep -rn` 推论层）。已有负责人在做的票保留 assignee，不挂 `sandcastle`。完成标准：每张未放行票一个原因。

4. **派工**。决策票：确认 decision-claimer 在跑（`launchctl list | grep decision-claimer`），未跑就在宿主前台跑 `node --import tsx .sandcastle/decision-claimer.mts --ticket <号>`（默认 dry-run）。实施票：确认 claim-loop 在跑、frontier 非空。主控自己的整理工作派子代理（Opus 5.5），写清文件归属与验收。完成标准：frontier 上每张票有在跑的处理者，或写明为何不派。

5. **验收**。查完成的封包（`/Volumes/AgentStorage/issue*-*/`、`sandcastle/issue-*` 与 `decision/*` 分支），按 `docs/agents/delivery-discipline.md` 关票门逐子句核。合格的开 PR 列给用户批合入；不合格的把缺项写回对应票。

6. **记账**。已关决策票在图正文 Decisions so far 加一行（名字带链接 + 一句要点）；可具体化的雾毕业成票；本轮结论发一条图评论：四桶计数、red 处置、新票、待用户事项。完成标准：评论链接。

## 收尾

向用户报三件事：本轮推进了什么（票名带链接）、什么在跑、用户要做什么。用户要做的只会是 spec 批准、`/graph-approve`、批合入。

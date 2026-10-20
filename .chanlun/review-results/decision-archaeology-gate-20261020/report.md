# 决策票起草前考古门（decision-archaeology-gate-20261020）

用户 2026-10-20 令：「所有的裁决票都要先考古」。分支 `codex/decision-archaeology-gate-20261020`，基于 origin/main `6badaa6049`。

## 结果

decision-claimer 在分级之后、起草之前新增考古阶段：逐个待决点两路搜（tracker 含已关闭票与评论；仓内正本层、推论层、代码常量与夹具），产出 `decisions/<票>/archaeology.md` + `archaeology.json`，逐点记查询词、命令、命中数与 A/B/C 归类。起草只把 C 类 `decision` 列为真正待决，B 类写成「默认值——确认或否决」，A 类沿用。独审新增 R8。`graph-engineering.md`、`graph-approve/SKILL.md`、`wayfinder-workflow.md` 同步。

## 对照表：改动点 → 复用的 Matt skill 原文条款 / 新增胶水

| 改动点 | 复用的既有条款（原文） | 新增胶水及理由 |
|---|---|---|
| 考古阶段本身（分级后、起草前） | `/grilling`：「If a *fact* can be found by exploring the environment (filesystem, tools, etc.), look it up rather than asking me. The *decisions*, though, are mine」 | 无人值守裁决面里没有人能当场说「这个早定过了」，所以把「查过」落成可复跑的 `archaeology.md/json` |
| 对照 CONTEXT.md / docs/adr，已裁沿用、冲突指出 | `/grill-with-docs`「Run a `/grilling` session, using the `/domain-modeling` skill」；`/domain-modeling`「Challenge against the glossary」「Cross-reference with code」 | `conflict` 字段：有冲突的点不得归 A |
| 两路搜 | AGENTS.md「开之前先搜（两路，缺一不可）」；ADR 0012 裁定四；代价第 4 条「每张裁定票前多一个 AFK 子代理」——本阶段即该子代理 | 机械校验：每点两路各 ≥1 条；tracker 须 `--state all` 或不带单状态的 `gh search issues` |
| B 类推理链 | ADR 0012 裁定一「援引方必须把推理链搬进票内重新过一遍」+ 四级溯源标签 | 无 |
| C 类去向 `research` | `/wayfinder` Ticket Types：「Research (AFK)… surface a fact a decision waits on. Resolved by a `/research` subagent」 | `gap` 三值；有 research/probe 缺口的 T 类转 P（决定等事实），由 `/graph-approve` 按 wayfinder 开票接 blocking 边 |
| C 类去向 `probe` | `wayfinder-workflow.md` 反面判据③「缺数字——派探针票去测，grilling 票只消费数字」 | 同上 |
| C 类 `decision` 交人 | `/grilling`「For each question, provide your recommended answer」「one at a time」 | 机械校验：C decision 缺推荐答案即不合格（推不出就改 probe） |
| A/B/C 三档 | `/triage` 第 1 步 redundancy 检查 +「Don't re-ask resolved questions」对应 A | B 档是本仓推论层（ADR 0012 三段式中间那格）特有，Matt 机制没有「有先例未裁」一档 |
| R8 | 无（独审 R 序列本就是 graph-engineering 胶水） | 抽查 ≥3 条命中复跑 + 对任一 B/C 点换说法复搜；A 当待决、事实缺口交用户、凭空要数即 BLOCK |
| 草稿「考古对照」段 | 无 | 让 R8 与机械校验逐点对账；落地前机械剥离，正本混入即拒 |
| graph-approve 呈现 | `/grilling` 一次一问 + 推荐答案；`/wayfinder` research 票 | 无考古的存量票先跑 `--archaeology-only` |
| `--archaeology-only` | 无 | 对已关票验考古形状、不写 GitHub 的 dry-run 入口 |

## 测试

`node --import tsx --test .sandcastle/decision-claimer.test.ts`：61/61 通过（基线 47，新增 14 条覆盖考古解析与校验、渲染、提示、全流程、重跑带反馈与升档、事实缺口转 P（首轮与重跑后两条路径）、考古对照漏点、影子问表、`--archaeology-only` 与锁、流程段剥离）。

## 真实 dry-run（已关决策票 #1355）

`node --import tsx .sandcastle/decision-claimer.mts --archaeology-only --ticket 1355 --out /Volumes/AgentStorage/archaeology-dryrun-20261020`，经 CLIProxyLocal 调 `claude-fable-5-1`，耗时约 27 分钟，0 次 GitHub 写。结果：14 个待决点，A 7 / B 5 / C 2，机械校验通过。A 类全部指到 ADR 0034 各裁定与 #1355 resolution 评论（该票确已按此裁定关闭）；C 类 Q7 去向 probe（引 ADR 0022 裁定七窗口纪律），Q8 去向 decision 并给出推荐答案、指出承接票 #1558。产物副本：`dryrun-1355-archaeology.md`。

## 非作者独审（claude-fable-5-1，经 127.0.0.1:8318）

- 第一轮 `review-fable-5-1.md`：REQUEST_CHANGES，0 HIGH / 4 MEDIUM / 8 LOW。MEDIUM：C decision 推荐值可空、畸形 JSON 抛错绕过重试、research/probe 缺口无人执行且不阻 T 落地、升档重跑考古盲跑。
- 第二轮 `review-round2-fable-5-1.md`：上轮全部 PASS（LOW-3 未改、接受）；新 2 MEDIUM（转 P 原因文案错、重跑后不过缺口门）。
- 第三轮 `review-round3-fable-5-1.md`：**APPROVE**，61/61；两条 LOW（续做丢 factGapNote、落地失败重复）已在 `b1643f4e4f` 顺手修掉。

## 已知限制

考古依赖关键词搜索，挡不住「搜了但词没想对」（ADR 0012 代价第 3 条同款），R8 的换说法复搜只降低漏检率。单票考古耗时约 20–30 分钟，裁决面每票总时长相应增加。存量 `needs-approval` 票没有考古，按 graph-approve 先补跑。

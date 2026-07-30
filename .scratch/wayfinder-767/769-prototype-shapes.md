# #769 事实备料：本仓「原型」形态勘察（只读，无裁定）

> 勘察范围：仓内一次性探索产物清点、8 张已关票的产物性质核查、便宜基线的时间尺度、
> `generation-constitution.md` 与 `/prototype` skill 抛弃纪律的冲突面逐字核对。
> 全程只读，未构建/未跑测试/未回测。

## 1. 仓内已存在的「一次性探索产物」清点

| 路径 | 一句话用途 | 常驻/一次性 | 现状 |
|---|---|---|---|
| `.scratch/` | 本任务专用暂存目录（`wayfinder-767/`），无历史内容，是这次任务临时新建的 | 一次性 | 空（仅本报告） |
| `tmp/` | 大量历史一次性产物：agent 间传的 prompt/ctx md、gemini/codex 调用脚本、图表 png、fred 数据 csv、genealogy 验证脚本、诊断日志等，文件名多为 `*-ctx.md` / `*-result.md` / `exp_*.py` / `gemini_*_verify_r*.py` | **一次性**（多为蜂群协作过程中的临时传递物） | 大量残留、无清理机制，git 未追踪（`tmp/` 惯例上不入库，需另核 `.gitignore`） |
| `scripts/`（210 个 `.py`/`.sh` 文件） | 混合体：`fetch_*`/`download_*`（数据抓取，常驻）、`check_fixture_drift.py`（CI gate，常驻）、`debug_divergence1-6.py`（`scripts/debug/`，6 个连号调试脚本，明显是逐次调试留下的一次性副本）、`scripts/experiments/`（10 个 `oq*_*.py`，命名即「open question」实验，一次性） | 两者并存，未分层归档 | `scripts/debug/` 与 `scripts/experiments/` 内容按文件名判断均为一次性用完即弃，仍留在仓内且被 git 追踪 |
| `rust/src/bin/`（43 个二进制，无 `rust/examples/`） | 37 个以 `p<票号>_*.rs` 命名的探针 bin（如 `p102_b_long_zero_probe.rs`、`p409_pan_live_probe.rs`、`tangency_probe.rs`、`issue550_event_battery.rs`），文件头注释逐字自称「探针/probe」「只读复核，不改任何生产源码」「诊断/反事实调用面」 | **设计初衷=一次性探针**，但**实际未删** | 抽查 `p102_b_long_zero_probe.rs`/`tangency_probe.rs`/`p409_pan_live_probe.rs`/`issue550_event_battery.rs` 全部 `git ls-files` 命中（已提交入库）；`git log --diff-filter=D -- rust/src/bin/*.rs` 无任何删除记录——37 个探针 bin 一个都没被移除过，长期留仓 |
| `.chanlun/review-results/` | 票面产出的分析报告/dump 落盘目录（大量 `.md`），review-results 反复引用 `p409_pan_live_probe`、`issue550_event_battery` 等探针 bin 的读数 | 按 `generation-constitution.md` §5 定性为「工作草稿——活期绑定票，票关后月度打包 tar 挪出仓」 | 现状仍在仓内、被 git 追踪，未见已执行的月度归档动作的直接证据 |
| `.chanlun/`（除 review-results 外） | roster / dispatch-dag / concept_registry / escalations 等蜂群协作记账文件 | 常驻（`generation-constitution.md` §5 明确「roster=活文档，留仓 tracked」） | 与「原型」范畴基本无关 |
| `find … tmp\|scratch\|probe\|explore\|try_\|poc` 命中 | 命中的实体几乎全部落在 `tmp/`（约 90 个）+ `rust/src/bin/p*_probe.rs`（少数），未见独立的 `probe*/`、`explore*/`、`poc*/` 顶层目录 | — | — |
| `formal/` 下带 `sorry` 的骨架文件 | `grep -rl "sorry" formal --include="*.lean"` **零命中** | 无 | 本仓 Lean 侧目前没有任何带 `sorry` 的试验性骨架模块 |

**一句话结论**：本仓「一次性探索产物」最典型的物理形态是 **`rust/src/bin/p<票号>_*.rs` 探针 bin**（票号自带、文件头写明只读/不接生产），但它们**造完从未删过**，与 Python 侧 `scripts/debug/`、`scripts/experiments/`、`tmp/*.py` 中同样标着「一次性」却也长期留仓的脚本呈同一模式。

## 2. 已关票里「造了个东西来看一眼」的先例（8 张逐张核查）

| 票号 | 标题/类型 | 造了什么产物 | 产物用完留下没 | 若按四票型重贴标签 |
|---|---|---|---|---|
| **#409** | `[task]` pan 活窗真实频次探针 | `p409_pan_live_probe.rs` 独立只读探针 bin（票面明写「走探针通道，不接线，参照 #301 三臂探针先例」），跑出产窗次数/反超触发数/三值分布等四组数字，供下游接线决策 | **留下**：bin 已提交入库（`git ls-files` 命中），后续多张票（#601/#527/#465 review-results）反复引用它做对照 | 内容形态最贴近「造个装置看一眼、拿数字做决策」——但产物**未按抛弃品处理**，长期驻留且被复用 |
| **#570** | `[task]` typed ledger 撞键修复前置探针 | 票面明确「只读+离线 join+纸面」，允许「若需轻探针可开 `#[ignore]` 一次性 bin，禁改生产路径」；核心产出是归因判定表（病型 2 根因五条逐条归属 + 病型 1 键形两案对照），**非必须造 bin** | 报告落 `chanlun/review-results/`，代码产物（如有）为 `#[ignore]` 一次性 bin | 更接近「纸面推演+离线数据核对」的 research/task，探针只是可选后备手段 |
| **#251** | `[task]` 卡点定位图重跑 wf7 链归因 | 重跑仓内既有生产装置（`chain_verdict`/`chain_first_gap`），读出既存机制的分布数字；票面明言「不设计放松实验」 | 无新代码产物，只有读数报告 | 纯粹「用现成装置重新measure」，不是「造个新东西验证设计假设」——归 research/task |
| **#264** | `1-bar structural prune 触发源逐笔归因` | 对既有 trades.jsonl 数据做代码路径归因 + 分桶统计，「只查不裁：归因+证据」 | 无新代码/装置产物，纯分析报告 | task/research（数据已现成，"不需重跑"票面自述） |
| **#214** | `[task]` 背书失败原因测量装置 | 明确「What to build」：给 `build_nest_certificate_index` 加**永久性**测量装置（私有核+双薄入口），带单测、验收标准要求「既有测试全绿、红线对照逐字节一致」 | **不是抛弃品**——装置进生产代码，接受 code review、要求向后兼容 | 这是四态名分中的「现役/新生入口」代码，明确不是原型（无「造完即扔」意图） |
| **#176** | 勘察：SplitLegLedger 接线代价 | 票面自称「AFK 任务，产出备忘录，不做裁决」，纯代价勘察，无代码产物 | 备忘录落 review-results | research/勘察，非原型 |
| **#175** | 勘察：P1–P8 解释器接线代价 | 同上，备忘录，无代码产物 | 同上 | research/勘察，非原型 |
| **#167** | `[experiment]` wf7 C 口径 L1 延迟确认入场 | 改一条回测口径（confirm 窗口逻辑），跑三组对照（A/B/C）产出笔数/净额/胜率等报表，为 #134 结案供实证 | 报表落 review-results；代码改动性质票面未言明是否为一次性分支还是正式合入候选 | 形态上最像经典「实验」：改一个参数/口径→跑一遍→读数据→喂裁决票，但目的是**验证经济效应**而非「设计/逻辑是否合理」，与 `/prototype` skill 定义的「回答设计问题」的原型不完全同型 |

**一句话结论**：8 张里，**#409 在产物性质上最贴近「造个探针看一眼」型原型**（且票面显式引用 #301 三臂探针先例，说明该模式在本仓有反复出现的惯例）；#570/#167 次之（前者纸面优先、后者是参数实验）；#214 是反例——明确造的是永久装置不是抛弃品；#251/#264/#176/#175 均为纯归因/勘察，无新造产物。

## 3. 「便宜」的量化基线（只摘记载，未实跑）

- **`scripts/check_fixture_drift.py`**（docstring 内 2026-07-26 实测，macOS arm64，Lake 5.0.0/Lean 4.31.0）：
  - 热路径（`formal/.lake/build` 已烤热）：单个导出器 `lake env lean` 各 0.3–5.9 s，两个 fixture 全查合计 **约 3–7 秒**。
  - 冷启动（`.lake/build` 缺失/为空）：先跑 `lake build` 全量构建，**144 jobs，实测 12.2 秒**，之后转热路径。
- `rust/` 下未找到单窗/小样本测试或 example 的耗时记载（`rust/examples/` 目录不存在；`rust/src/bin/` 里的探针 bin 头部注释均未记跑一次的耗时）。
- CLAUDE.md / AGENTS.md 中未见 `lake build` 之外的其它构建耗时记载。

**一句话结论**：本仓唯一有文档化耗时记载的「跑一次看结果」路径是形式化侧的 fixture 校验——**秒级（热路径 3–7 秒）到十几秒级（冷启动 12.2 秒）**；Rust 引擎/回测侧的探针 bin 跑一次要多久，本仓文档里**没有记载**（无从判断秒级/分钟级/小时级，只能说"无记录"）。

## 4. 抛弃纪律的冲突面

**`docs/agents/generation-constitution.md` 全文核对结果**：文件内**没有**出现「保留失败的生成史」这句话，也没有 grep 命中「保留」「生成史」「失败」相关条款。该文件通篇讲的是「名分五态」（现役/新生入口待驱动/deprecated待退役/孤儿待删/垃圾待清）+ 判据 + 处置流程，核心诉求是「让精力不再流向死代码」（文件行 4）。

「保留失败的生成史」一句的**真实出处**是 `~/.claude/skills/meta-orchestration/SKILL.md` 的技能一行简介（第 6 行）：

> 核心原则：蜂群是矛盾显现机器，不是代码生产线。防止workaround，**保留失败的生成史**。

与 `generation-constitution.md` 最接近的相关条款是 **§5「记账层产物口径」**（原文逐字）：

> review-results / handoff = **工作草稿**，不是档案：**活期绑定票**——关联票 open 期间留仓，票关后月度归档周期打包 tar 挪出仓（仓外 archive 目录）。roster（`.chanlun/agent-roster-*.md`）= 活文档，留仓 tracked。仓内长期只留活文档（CONTEXT/ADR/docs/纪律文本）。票上对草稿的路径引用在归档后失效，以文件名在 archive 包中检索——这是「归档」与「留存」的刻意区分。

`/prototype` skill 的抛弃纪律原文（规则 6，逐字）：

> **Capture it when done.** Fold any validated decision into the real code, then capture the prototype itself as a **primary source**: commit it to a throwaway branch, out of main, and leave a context pointer to that branch on the implementation issue. Capture the answer too — the verdict and the question it settled — in the issue or a commit. The main branch keeps only the validated decision.

**冲突面摆出（不裁定）**：
1. `generation-constitution.md` 里**没有**「保留失败的生成史」这条文本本身——它是另一份文档（meta-orchestration skill 简介）的表述，#769 票面把它归到 `generation-constitution.md` 名下，与本次核对结果**不一致**，需 #769 裁定时先校正引用源。
2. `generation-constitution.md` §5 的实际口径是「草稿类产物（含探针报告）票关后**打包挪出仓**」，不是「main 分支永久留原型代码」；而 `/prototype` skill 是「原型代码进**独立 throwaway 分支**，main 只留结论」。两者方向接近（都不让 main 长期携带原型代码），但触发机制不同：一个按「票关」触发归档，一个按「原型完成」立即挪分支。
3. **与仓内实证脱节的是第三方**——`rust/src/bin/` 的 37 个探针 bin 既不在 throwaway 分支，也没有走 §5 的月度打包挪出仓，而是**直接提交在 main 上长期留存**（第 1 节已证：无一被删除）。这是实际行为与两份文档任一口径都不完全对齐的地方。

## 附：#769 票面本体

标题：[grilling] 原型票在本仓的形态与准入判据：0/0 怎么破（map #767），状态 OPEN，标签 `wayfinder:grilling`，parent #767，blocked-by #768，blocking #773。四条待裁问题原文已在票面（形态候选/准入判据/抛弃纪律共存/grilling 票边界），本报告对应供第 3、4 条问题的事实输入。

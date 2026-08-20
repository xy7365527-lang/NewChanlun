---
name: pstack-blast-radius
description: "仅限用户显式调用（user-only）：用于非小型行为或契约变更的爆炸半径分析。请通过 `/skill:pstack-blast-radius` 显式调用；因 #1138 路由不稳定，已禁用模型自动调用。"
disable-model-invocation: true
license: MIT
metadata:
  upstream-repo: cursor/plugins
  upstream-sha: fd6dd6f7276956a532bb78a748a8d2818b6eb5f4
  upstream-path: pstack/skills/blast-radius/SKILL.md
  license-file: LICENSE.MIT
---

# Blast radius（破坏面）

在改动合入前，找出它在 diff 之外会破坏什么。用于「blast radius of X」「这个改动会破坏什么」，
或评审一个你不放心的 diff。

`pstack-how` 告诉你代码在做什么；本 skill 告诉你它在别处会破坏什么。**列出调用者不是任务**——
那 grep 一秒就能干。任务是找出 grep 不会显示的破坏。

## 调用、触发与跳过

- **调用**：本 Skill 已降为 user-only；仅通过 `/skill:pstack-blast-radius` 显式调用，不承诺自动语义加载。
- **触发**：非小型行为变更（改返回类型/字段/语义/错误码/协议/常量），或可能影响其他路径的可疑 diff 评审。
- **跳过**：普通改名、格式化、注释订正等无行为变化的改动，不启动本 skill。

## 不要相信自己的文字总结

一份「听起来有道理」的破坏面报告一文不值——它不管真假都读起来有说服力，这正是要避开的陷阱。
所以不要只交回报告。找出整个判断依赖的那一两个事实，**用跑代码来证明**。文字是起点，不是交付物。

### 多确定才算数

对改动安全性所依赖的每个事实，尽量把它往下推到便宜能到的层级，并说清停在哪一层：

1. 你口头说了。单独不值钱。
2. 你指到了行。真实的 `file:line`，或库自己的源码。
3. 你证明了坏情况到不了。逐步走了一遍失败路径，它走不通。
4. 你跑了它。一段脚本或测试，调用真实代码，错就大声失败。
5. 你在运行中的应用里复现了它。

**任何到不了第 4 层的事实，要大声说出来。** 不要写成「已定论」。第 4 层通常是一小段脚本，
import 应用实际发布的同一个库，调用你担心的那个确切函数。

## 步骤

1. **读变更。** diff、它新增/修改/删除的符号、以及它现在行为上的不同——包括 diff 没明说的那部分。
   Prime 下用 `git diff <fixed>...HEAD`（三点，与 merge-base 比）与
   `git log <fixed>..HEAD --oneline`；拉 issue/PR 动机用 `gh`（口径见
   `docs/agents/issue-tracker.md`）。不依赖上游 `why`（本批推迟到第二批）。
2. **找「它之所以安全」的那一个事实。** 大多数吓人的改动其实只靠一个事实就安全，例如
   「这次调用只会丢掉已死的缓存项，别的什么都不干」。找到它；它成立，大多数吓人 case 一次死光。
   把时间花在这里，而不是列一长串「也许」。
3. **看 grep 会停下来的地方。** 结构查询按以下优先级复用现役能力，必要时补代码搜索：
   - **Serena**：`await serena.find_symbol(name_path=..., relative_path=...)`、
     `await serena.find_referencing_symbols(...)`、`await serena.search_for_pattern(...)`
     —— LSP 符号/引用，比 grep 精确，但会漏跨文件文本、宏/模板生成、序列化格式。
   - **codebase-memory**：`await codebase_memory.trace_path(function_name=..., direction="both", depth=3)`、
     `await codebase_memory.search_graph(...)`、`await codebase_memory.detect_changes()`
     —— 调用图/影响面，但会漏跨语言读同一字节、运行时数据格式。
   - **代码搜索（grep/ripgrep）**：补文本面——JSON 字段、DB 列、wire format、测试 fixtures、
     序列化器、feature flag、三层下游。
   三类各有盲区，缺一不可；**不得只复述 diff**。
4. **诚实对待每个风险。** 给它一个真实发生的概率和一个真实发生的代价。保留你确认的风险，
   把你查过并排除的单独列出。引用真实 `file:line`；搜不到也是一种答案，但**零命中是「未发现」，
   不是「已证明安全」**；绝不编造调用者或 API。
5. **证明那一个事实。** 写一段脚本或测试跑真实代码，跑它，把输出贴出来。便宜证明不了就标
   `unproven`，不四舍五入。靶向验证命令要在错时大声失败（断言失败/非零退出），不是打印一行日志。
6. **大而宽的改动**：若 `pstack-arena`（#1132）已可用，把它当 arena 跑多模型交叉；否则单模型
   逐项核查，并明说「无多模型交叉」。本 skill 不自行 spawn 多模型扇出——由根代理按 arena 口径编排。

## 交付物（两节分离，禁止混写）

- **What it does.** 改了什么，包括不明显的那部分。
- **潜在破坏面（未证实的主张）。** diff 之外的调用者、依赖与失效路径，每条给 `file:line` 与
  「会怎么坏」。这一节是假设，不是证据。
- **实际验证证据（已运行的命令）。** 那一个安全事实，声明推到第几层，**贴命令与真实输出**；
  到不了第 4 层就写 `unproven`。grep 零命中归「Cleared」节，绝不放进这一节冒充行为证明。
- **Risks（确认的风险）。** 每个写清怎么坏、`file:line`、多可能、多严重、怎么查；重要的贴证据。
- **Cleared（查过并排除的）。** 查了什么、为什么没事；零命中写「grep 零命中（未发现，非行为证明）」。
- **Before you merge.** 能抓住真实 bug 的最便宜测试/复现，附你写的那段脚本。

引用真实代码；发到任何公开处之前剥离私有信息。

**Reply:** 上面的报告，那一个安全事实要么已证明，要么标 `unproven`。

## 来源

- **上游仓库**：`cursor/plugins`（GitHub，`pstack/` 目录）
- **固定 SHA**：`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`（不自动跟随上游 `main`）
- **原始 Skill**：`pstack/skills/blast-radius/SKILL.md`（无 `references/`、`scripts/` 相对文件）

## 本地 Prime 改写

- **调用契约**：最初采用 Prime `both`，但 #1138 多轮真实 3+3 路由在可用宿主模型上反复误触发/漏触发，
  已按 #1129 降级条款恢复 `disable-model-invocation: true`，改为 user-only；保留
  `/skill:pstack-blast-radius` 显式入口，不再承诺自动语义加载。
- **名称**：`blast-radius` → `pstack-blast-radius`（首批统一 `pstack-` 前缀，避免与未来用户级
  Skill 同名 shadow）。
- **结构查询工具**：上游依赖 Cursor 代码库导航 → 改为优先复用 Serena
  （`find_referencing_symbols`/`find_symbol`/`search_for_pattern`）与 codebase-memory
  （`trace_path`/`search_graph`/`detect_changes`），必要时补代码搜索；三类盲区互补，不得只复述 diff。
- **输出两节分离**：新增「潜在破坏面（未证实主张）」与「实际验证证据（已运行命令）」的强制分离，
  并把「grep 零命中 ≠ 行为证明」显式化——对上游「搜不到也是答案」的收紧，防 grep 零命中冒充证明。
- **`why` / `arena` 引用**：上游步骤 1 用 `why` 拉 PR（本批推迟）→ 改 `git`/`gh` 口径；上游步骤 6
  用 `arena` 多模型 → 改为「若 `pstack-arena` 已可用则转交根代理编排，否则明说无多模型交叉」。
- **语言**：正文简体中文；命令与标识符保留原文。
- 其余决策流（不要相信文字总结、六级确定度、交付物条目）与上游语义一致，原样保留。

## 许可证

MIT（Copyright (c) 2026 Lauren Tan），见同目录 `LICENSE.MIT`。

# 本仓 wayfinder 会话便条

> 由 `UserPromptSubmit` hook 自动注入（[#784](https://github.com/xy7365527-lang/NewChanlun/issues/784)）。你打的 `/wayfinder` 加载的是**通用 skill 正文**，下面是**本仓特有**的口径——通用正文里一个字都没有。

## 先读这三份

1. `docs/agents/wayfinder-workflow.md` —— **本仓 wayfinder 唯一正本**（入流判据 / 五段管线 / 图的两行声明 / 四票型口径 / 一票一会话与并行纪律 / HITL 吞吐纪律 / 图正文写入协议 / 图的寿命与关图判据 / `/to-spec` 交棒边界 / 开票手续）
2. `docs/agents/issue-tracker.md`「Wayfinding operations」段 —— tracker 物理操作（map/子票/blocking/frontier/claim/resolve 的 `gh` 命令）
3. `AGENTS.md` —— 全 harness 唯一正本入口

碰代码的图再追加 `docs/agents/delivery-discipline.md`。

## 五条最容易忘的

1. **走图会话开场先跑对账** —— 列出这张图所有**已关**子票，跟正文 Decisions-so-far 逐行比，缺哪行当场补。一条查询的事，丢行最多存活一个会话。
2. **grilling 入口 = `/grill-with-docs`，一次一问** —— 不打裸 `/grilling`（那样会漏掉 `/domain-modeling` 的落文档后半程，而正本要求落完文档才关票）；不批量抛题，agent 不得替人答。（2026-07-30 曾裁定改 batch 节奏，当场被 [#779](https://github.com/xy7365527-lang/NewChanlun/issues/779) 推翻并作废。）
3. **改图正文前压 `updated_at`** —— 要写的那一刻才读 body，发 PATCH 前再查一次 `updated_at`；变了就重读重拼。冲突重试一次，再冲突停下报人，不要盲目循环。
4. **`sub_issues` 必须 `--paginate`** —— 单页默认 30 条，不加会**静默漏票**并把「/30」误读成总数（本仓 #529 = 43 张、#695 = 34 张已触线）。`gh issue list` 同理，`--limit` 给足。
5. **写仓的 AFK 子代理强制开 worktree**（`Agent` 工具 `isolation: "worktree"`），只读的不强制。主仓撞车有 `git stash` 抹掉未提交改动的实伤前例。

## 三条判据，拿不准时用

- **票型**：这张票做完，是**解开了一个决策**，还是**交付了一件成果**？前者才是图的子票。
- **要不要开图**：一个问题对话能 settle → 单会话 `/grill-with-docs`，不开图；路已见底只差写清楚 → 直接走 `/to-spec` 链；与在飞决策不挂钩的文本欠账 → triage 挂 `debt`，不进图。
- **要不要出 spec**：**终点是文档就直接写，终点是代码才要 spec。**

## 一条元判据

本仓已三次撞上同一个错误：**拿产出的外形当分类维度**（原型 vs 探针是同一个 Rust bin；spec「出图/不出图」是连续轴不是两类；「文书票」是伪类）。遇到「这是哪一类」的问题，先怀疑维度选错了——承重的是它**在图里干什么**，不是它长什么样。

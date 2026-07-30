# Wayfinder 工作流（五段管线）

来源：2026-07-30 对照 Matt Pocock wayfinder 视频（"Nothing is too big to plan anymore"）的使用盘点与裁定，编排者批准三项决策：**新图严格两段式**、**启用 prototype 票**、**固化本文档**。tracker 物理操作（map/子票/blocking/frontier/claim/resolve 的 gh 命令）见 `docs/agents/issue-tracker.md`「Wayfinding operations」，本文只管流程形态与票型口径。

## 什么时候不开图

单 session 可规划、路已见底的活不开 map——直接烤或直接做。wayfinder 只用于有战争迷雾的大块工作：知道大概要去哪，但中间步骤看不清。（Matt FAQ 同口径。）

## 五段管线

1. **开图**（编排层，占一个 session）：grilling 定 destination → 广度优先烤 frontier（扇形铺开，不深挖单线）→ 建 map（`wayfinder:map`，Destination/Notes 填好，雾写进 Not yet specified）→ 建可具体化的票 → 第二 pass 接 blocking 边。**research 票当场并发发 subagent 解决**，不等后续 session。开图 session 不解决任何票。
2. **走图**（一票一 session）：取 frontier 第一票（open + 无 open blocker + 未 assign；用户点名则从其点名）→ **claim 先行**（assign 是 session 首写）→ 按票型解决（见下）→ resolution comment 写答案 → close → map 的 Decisions-so-far 加一行（gist + 链接，详情只留在票内）→ 毕业因此具体化的雾、新开浮出的票、把越出 destination 的票关票并登记 Out of scope。frontier 上无阻塞的票可并行多 session 走。
3. **图闭环出 spec**：frontier 清空、destination 是 spec 时，从 map 抽 spec——每条决策 gist + 链回决策票。决策票是 primary source，spec 只是它的稠密摘要；实施 agent 有疑义时回溯原票。
4. **实施票链**：spec 批准后切**独立实施票**（不是 map 的子票）→ claude sonnet 实装 → opus 影子评审 → 逐票评审合入，关票走 `delivery-discipline.md` 关票门全子句。
5. **spec 非持久**：实装落地后 spec 票关闭，不作长期维护的真理源。真理源是代码 + 决策票；spec 从落地那刻起是历史记录。

## 两段式裁定（2026-07-30）

- **新图严格两段**：map 只产决策，destination 是「路看清了/决策锁了/spec 出来了」；实装不进 map 的 task 票。
- **在飞图不动**：已显式 override「决策+执行混合」的图（#743、#529 等）维持现状跑完，不中途拆票迁票。
- **task 票回原义**：只为解锁决策的手工活（注册服务拿到 API 再评判、挪数据看清形状），它 do 但不交付 destination 本身。

## 票型口径

- **grilling**（HITL，默认型）：「该是什么 / 该不该」类裁定。配 `/grilling` + `/domain-modeling`，一次一问。agent 不得替人答。
- **research**（AFK）：决策等在仓外事实（文档、三方 API、外部知识库）上。开图当场由 subagent 并行解决，结论落票。
- **prototype**（HITL，2026-07-30 启用）：「该长什么样 / 该怎么行为」类问题——做一个粗 artifact 拿来反应，不追求合入。本项目适用面：
  - 前端/面板形态：信号读数、图谱展示的界面稿（`frontend/`）；
  - Rust 状态机 / pipeline 行为 stub：验证状态模型手感（`prototypes/` 已有此先例）；
  - 账本口径最小对拍脚本：双跑对拍原型；
  - Lean 形式化前的 Python/Rust 粗语义模型。
  
  判据：讨论三轮不如看一眼粗稿的 → prototype；烤能烤清的 → grilling。原型是防 waterfall 的手段，不是提前实装。
- **task**（HITL 或 AFK）：见两段式裁定。resolution 记录做完的事 + 下游票依赖的事实（凭据位置、新 URL、行数等）。

## session 粒度与引用

- 一票一 session（research 票除外——开图当场并发）；每 session 最多解决一张票。
- 并发安全：claim（assign）是 session 首写，其他 session 见到已 assign 的票跳过。
- 对人叙述一律用票名，链接裹在名内；不甩裸 `#` 号墙。

## 关票口径

决策票（grilling/research/prototype）按纯裁定关票：resolution comment + close + map 索引一行；有实体产物的（对拍报告、原型文件）按 `delivery-discipline.md` 关票门第 6 子句入仓引用，无实体产物的写明「无报告」。实施票走关票门全子句，无豁免。

## 只管新图

本规范约束 2026-07-30 之后新开的 map 与票；在飞图维持各自 Notes 里明写的 override 跑完。

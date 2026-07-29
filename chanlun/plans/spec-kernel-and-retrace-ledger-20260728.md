# spec：账本内核抽取 + 买卖点身份账本实装（#465 裁定 A：T1 + T3）

> 日期：2026-07-28
> 上游：#465 裁定 A（抽通用内核）；T2 #574 语义契约（`chanlun/review-results/issue574-semantic-contract-20260728.md`，
> 已上票 #574）；#465 调研（`chanlun/review-results/issue465-nest-lifecycle-reusability-20260728.md`）
> 下游票：#573（T1）、#575（T3，blocked by #573）
> 本 spec 两章：T1 章定稿；T3 章写到「契约 → 组件」映射层，内核接口形状待 T1 落地后回填（防早产抽象）。

## 问题陈述

`nest_lifecycle` 把「可抽出的账本内核」与「Pan/Trend 活假设业务策略」写在同一模块同一 `advance` 里：
20 个责任点中 11 个对象无关（per-key 注册/首建/append-only/倒退拒绝/终态吸收/钟首写/增量返回/迁移留史/
只读枚举/不变量骨架），9 个绑死假设域。买卖点（首次回抽三类点）侧，仓内只有一个 571 行只读 replay
自动机：无账本、无持久化、无消费门户、零生产调用点。全仓宽口径同型物 7 套、零共享类型契约（#257 同病）。
#465 裁定 A：抽通用内核（T1），再按 T2 语义契约在其上建买卖点身份账本（T3）。

## 解法

T1：把 11 个对象无关责任点抽成泛型内核模块，nest_lifecycle 重基于它——nest 行为 bit-exact +
不变量保持为唯一验收尺。T3：按 #574 契约实装买卖点身份账本——身份 = 中枢四条边快照 + departure；
三态 Provisional / Confirmed / NotConstituted{RetestReentered|CenterRebased}；append-only 修订日志为唯一
真相；三档消费门户（备战/成立/短差）。T2 契约的八条裁定即 T3 的语义规约，逐条落到组件与验收。

## 用户故事

1. 作为内核（T1）的消费方，我要 per-key 注册、首建、append-only 修订、倒退拒绝、终态吸收、钟首写、
   增量返回、只读枚举、不变量骨架都由内核承载，以便我只提供 key/观察/转移策略/原因载荷四组类型。
2. 作为 nest_lifecycle，我要重基后全部既有语义经内核表达且行为 bit-exact，以便 #421 生产进料、
   修订留档、统计审计面零漂移。
3. 作为评审者，我要 nest 既有测试全绿 + p123/p92 replay 对拍零 diff + assert_invariants 全保 +
   测试指纹前后对照，以便确认 T1 零生产语义变更。
4. 作为买卖点账本，我要身份 = （中枢四条边快照, departure move 索引），以便判案锚在注册时的框
   （补充十三：价格只对该框 ZG/ZD）。
5. 作为买卖点账本，我要观察携带（严格相邻 pair, outcome, 当前中枢窗口, as_of），以便检测引擎改口
   并执行时间单调纪律。
6. 作为买卖点账本，我要候选活着期间窗口不动（行情物理保证），引擎改口则处死记档进警报桶，
   以便不静默容忍也不桥接。
7. 作为买卖点账本，我要同一中枢同时刻至多一个活跃候选、新 departure 未判完而来则报错拒收，
   以便紧邻语义（补充十五）落地。
8. 作为买卖点账本，我要残废输入（leave 未出中枢/回抽同向/不紧邻/missing）注册期拒收报错，
   以便错误与终态分家。
9. 作为买卖点账本，我要判胜记 Confirmed 并产出中枢死亡证明（快照转正），以便中枢账消费 Broken。
10. 作为买卖点账本，我要判败记 NotConstituted{RetestReentered}——候选从未成立、中枢未破坏、
    不派生中枢事件、事件为盘背观测源，以便名分与补充十四对齐。
11. 作为买卖点账本，我要引擎改口记 NotConstituted{CenterRebased} 并进警报桶，以便身份灭失可审计
    （对标 RebaseVanished 哲学）。
12. 作为买卖点账本，我要终态后同身份迟到输入静默吸收 + 警报计数，以便幂等重放不炸管道。
13. 作为买卖点账本，我要成功后同一中枢永禁新轮、死中枢新 departure 报错拒收、新中枢新离开开新档，
    以便补充十三「死后穿越不构成信号」落地。
14. 作为买卖点账本，我要出生钟/落锤钟/门卫钟三钟 + 每条修订带知情时、位置进证据载荷，
    以便回测不用未来信息。
15. 作为买卖点账本，我要 append-only 修订日志为唯一真相、状态 = 日志折叠、快照仅派生缓存带溯源，
    以便跨进程恢复且无双真相漂移。
16. 作为买卖点账本，我要身份缺席永不等于消失、无超时处死、消失只走判决/引擎改口两条显式路，
    以便 prefix 重放一致。
17. 作为交易层，我要未决候选经备战档可见（位置 + 快照），以便盯回抽次级别回切入点（024:36）——
    名义是「盯这里」不是「买这里」。
18. 作为交易层，我要成功事件带全套证据（知情时 + 位置），以便我自己做迟到过滤（#587），账本不进口
    外部状态。
19. 作为盘背/短差通道，我要判败事件带亚型签 + 三锁（键唯一/去重/账平断言，#606 S1 先例），
    以便 024:36/024:46 短差语义消费（campaign 同构）。
20. 作为中枢账，我要成功事件即死亡证明（不问迟到，054:60 延迟合法），以便登记中枢 Broken。
21. 作为审计/谱系，我要全部历史身份留档、Restart 新档载荷记前任、警报另记 audit 流，
    以便谱系学保留。
22. 作为编排者，我要消费方明确登记（无消费方不接生产）+ first_retrace_replay.rs 处置须我终审
    （#225 先例），以便防 built-but-unwired 与静默删除。

## 实装决策

### T1 章：内核抽取（定稿）

- 新建泛型内核模块，承载 11 个对象无关责任点：per-key 注册表、首次观察建项、append-only 修订追加、
  修订计数与历史一致、per-identity 倒退拒绝、终态吸收、钟首次写入不后移、每次推进返回增量、
  身份迁移保留历史（nest 侧消费）、只读枚举/过滤门户、不变量骨架。
- 泛化面 = 四组类型参数：Key（等值 + 序）、Observation、TransitionPolicy（含判据/拒绝/终态映射）、
  ReasonPayload（原因码 + 证据载荷）。
- nest_lifecycle 全部既有语义改经内核表达；六元组 key、bridge_identity、三态业务条件、失效原因、
  修订词汇、五钟、provider/feed、力度判定、完成消失结算全留 nest 侧。
- 唯一验收尺：nest 行为 bit-exact（既有测试全绿 + p123/p92 replay 对拍零 diff）+ assert_invariants
  全保 + 零生产语义变更 + 测试指纹前后对照（共享工位计数先核并行线未提交面，污染则注明常量，
  #475 先例）。
- 第二消费方定形输入（来自 #574 契约）：key = （四条边快照, departure）；观察 = (pair, outcome,
  窗口, as_of)；转移策略 = 三态 + 回判/改口/拒收；原因载荷 = NotConstituted{RetestReentered|
  CenterRebased} + 证据（新旧窗口、位置、知情时）。第二消费方不使用迁移留史。

### T3 章：买卖点身份账本（契约 → 组件映射）

- 新模块：买卖点身份账本，建立在 T1 内核上（内核接口形状待 T1 落地后回填本章）。
- 观察适配器：输入（strict adjacent pair, outcome, 当前中枢窗口, as_of）；残废件注册期拒收报错
  （leave 未出中枢/同向/不紧邻/missing）；死中枢新 departure 拒收报错（自查死亡证明）。
- 状态机：Provisional / Confirmed / NotConstituted{RetestReentered|CenterRebased}；终态吸收 +
  迟到静默吸收 + 警报；Restart = 新档注册、谱系载荷记前任。
- 窗口纪律：注册拍四条边快照；活着不动；Success 快照转正 + 产中枢死亡证明；重回快照作废；
  引擎改口处死记档 + 警报桶。
- 钟：出生钟/落锤钟（首写不改）+ 门卫钟（倒退拒收 + 警报）；每条修订带知情时；位置进证据载荷。
- 持久化：append-only 修订日志唯一真相（CompletedFreezeReducer 先例：JSONL、幂等重放、冲突拒绝）；
  状态 = 折叠；快照仅派生缓存带溯源；拒收/警报另记 audit 流；日志按 run/窗口分界带头 provenance。
- 门户三档：备战档（未决：位置 + 快照只读，禁消费成买入信号）；成立档（Confirmed：中枢死亡证明 +
  交易层证据包）；短差档（判败：盘背通道亚型签 + 三锁 + 失败处置通知）。
- 事件与警报词汇：注册/判胜/判败/改口处死/Restart（新档）+ 残废拒收/死人挂号/倒退拒收/迟到吸收
  四类警报计数。
- first_retrace_replay.rs 处置随票落（替换/删除须编排者终审，#225 先例）；其 fixtures 转作
  对拍基线（行为等价面：严格 pair 映射、消费一次性、Supersede/Restart 纪律）。

## 测试决策

- 接缝（最高层、越少越好）：T1 = NestLifecycleBook 公共接口（advance/entries/consumable_closed/
  settlement_stats/assert_invariants）+ p123/p92 replay 对拍缝；T3 = 账本「观察入 → 事件/修订出」
  边界 + 三档门户只读面。一本账一条缝，不测内部实现细节。
- T1：既有测试全绿；p123/p92 replay 对拍零 diff（SHADOW harness 先例）；assert_invariants 全保；
  测试指纹前后对照（核并行线污染，#475 先例）；Standards 硬杠（函数 ≤50 行/文件 ≤800/错误
  fail-loud/常量具名）。
- T3：TDD——状态迁移表驱动（注册/判胜/判败/改口/残废拒收/迟到吸收/死人挂号/倒退拒收/Restart 新档）；
  日志重放一致性（fold(log)=state、prefix 一致、缓存校验失败回退重放）；门户分级（未决不可消费、
  成立证据齐、判败亚型签三锁）；对拍 first_retrace_replay 现有 fixtures 行为等价面；golden 事件日志锚。
- 先行艺术：nest_lifecycle 测试群（约 2,300 行）；CompletedFreezeReducer JSONL replay 测试；
  p123_fast_replay harness；wf8 验收行（观测桶计数进验收）。

## 出界

- 中枢生命周期账本身的改动（消费死亡证明是它的票，本 spec 只产证明）。
- 交易层迟到三类点过滤（#587 已裁，归交易层自理）。
- pan_div_diag 盘背通道实装（#606/#607 线）；本 spec 只供判败事件源。
- 清算线「旧框 vs 新一代三类点证」张力（follow-up 票另裁，见 #574 契约上报事项）。
- 超时处死（永不做）；跨窗口身份连续（不承诺）；χ 统计门线（已撤，裁定书在案）。

## 附注

- T3 章内核接口节待 T1 落地后回填（防早产抽象，双消费方定形精神）。
- 消费方登记归 #575 验收项（无消费方不接生产）。
- 模型档：T1 高难（opus）；T3 子弹票按面定档（sonnet/opus）。
- 两处上报（不阻塞本 spec）：清算线张力建议开 follow-up 票；ADR 补充十三引用档
  `thirdclass-level-doctrine-research-20260727.md` 仓内缺失，引用链补洞另议。
- 评审链：两轴 + 影子评审票（实装票规矩）；TDD；worktree /tmp/kimi-nest-mainline；禁 git mutation；
  resolution/评论用 --body-file。

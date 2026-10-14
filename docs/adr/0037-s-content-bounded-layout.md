# ADR 0037：S 内容布局 bounded-layout/1——出生序累计数组与 CC-007 单元批记录

**日期**：2026-10-14
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核），走 #1489（承接 #1481 阶段 0，Part of #1323）
**状态**：已采纳。本 ADR 不改 APPROVED-SOURCE（`fab912c6…`）§1:17/§3:51 与 §5.1–5.9 任何已批条文的字面读法；不改缠论教义判定；CC-007 的 2,615,078 个比较单元仍由 S 同核产生、内容寻址、不删不省；只定 S 发布内容的数组次序、CC-007 事实落在哪条记录上、布局版本的发布槽与容量门。

## 背景
#1481 阶段 0 在 S86 r6 输入上实测：每代新增可达节点 Δ_R(g)≈147·g−960，全历史节点数随 G 平方增长，外推 9011 全库 1.0–1.6 TB、末条单步 17–45 s。三个来源（`0fc0c165`）：快照累计数组按内容键排序致新元素中段插入、§5.2 M 树整段重编（`aux_index.rs:762-789`、`object_index.rs:287-301`）；`CC-007.fractal_description` 载荷含分型之后全部 bar（`content_producer.rs:1780-1810`）致每个存活分型每代修订；`CC-054` 的 `requests` 随 CC-007 请求每代全新。§5.2 的 M 树（k＝严格小于 n 的最大 2 的幂）本身追加稳定（n→n+1 新增内部节点 ≤ ⌈log₂(n+1)⌉+1），病在元素次序与对象内容，不在编码。

## 裁定
### 一、累计数组按出生序（append-only）
publication_template 快照的 `objects` 按 (first_known_generation ↑, 同代内原键)，`withdrawn_objects` 按 (withdrawn_generation ↑, 同代内原键)，`witnesses`/`relations`/`observations` 按 (published_generation ↑, 同代内原键)，`raw_history` 按 seq 不变；修订记录是新出生，落尾；CC-054 `requests` 按 (window_start, label)。§5.2 编码、`content_format=s-content/1`、§5.5 Q 行序 `s-record-order/2` 均不变——Q 投影在读取时按 record_order_key 重排，与快照数组次序无关。§5.4「base Snapshot 加 Delta 逐字段等于 next Snapshot」在出生序下即「upserts 追加到尾、withdrawals 移入撤回尾」。
### 二、CC-007：比较单元只发一次，分型描述对象只出生一次（表示 (b)）
每个（分型, 后续 bar）比较单元 `{bar, relations[24]}` 由 S 同核计算一次、作不可变内容发布一次；除该 bar 的 revision 或分型基础来源被修订外不重发、不换身份。承载方式：每代一条 typed 记录 `CC-007.development_batch`（fact_key `[kind, bar.source_coord]`，payload `{bar, entries:[{fractal_object_id, unit, development:{root,count}}]}`，entries 按分型出生序），`development` 是该分型截至本 bar 的单元 §5.2 序列 root 与计数；`CC-007.fractal_description` 对象只含分型描述与基础来源，只在基础来源修订时出新版；「尚无后续 bar」由 CC-054 `subsequent_development` 待决请求承载。Q、前端、审计只读 entry 与序列分片（§5.6），不重算比较（`s_session/README`「Q 和浏览器不另算」、#804）。被否：每单元一条 typed 记录、走 observations 族（两者使 Q rows 累计 +2.6×10⁶，§5.5 首页收齐引用不可行）；读者派生（违反 §1:17/§3:51）。新 kind 属布局组件：不新增轴、不改 CC-007 判据，`CatalogAxisTable` 轴表不变，kind→轴映射加一条。
### 三、CC-054/056 内容定义不动
不做增量表示、不分代归档；CC-054 的增长是 CC-007 的投影，二落地后余下 O(1)+O(log g)。义务：单代新增节点实测锁。
### 四、Δ_R(g) 分解与上界门
Δ_R(g) ≤ Σ_{f 被跟踪}(u + e + ⌈log₂count_f(g)⌉ + 1) + C₀_base + C_bi·bi_births(g) + C₁·⌈log₂N(g)⌉。u·F(g) 是固有事实项（F(g) 被跟踪分型数，u 每单元新增节点），不可削；e、C₀_base、C_bi、C₁ 是表示与布局的常量代价。五个常数在 S86 前缀上独立实测冻结、与构造式复算相差 ≤10%、写进读数文件与运行 manifest（复算值 79/22/1,100/1,500/100）。本 ADR 的目标与验收门是：每个测量代都不超此上界（S86 逐代、9011 选点，超界代数 0），并以区分力自检（layout/1 实测代入须超界）证明门有效。不以「Δ_R − u·F 对 g 斜率 ≈0」为门：(b) 的 e·F 与 log 项随 F ∝ g 增长，斜率门按设计必不过，拟合常数又会吸收 layout/1 病态。有原始修订时另加坐标 ≥ 修订点的派生记录与各分型序列重算，这是语义必要量。
### 五、布局版本槽
`rule_revision` 新组件：`V3_RULE_REVISION = "s2-axis-quantifiers+new-bi/1+bounded-layout/1"`；组合规则 `<base>+<教义组件…>+bounded-layout/<n>`，组件正交，各票改各自组件；自本 ADR 起 v3 写者任何修订串必含 `bounded-layout`。它已在 §5.3 object_id 配方、§5.4 manifest、§5.5 fixed_cut/token、§5.7 snapshot 中，读者每层可见，旧新对象不可能撞 ID。
### 六、旧库只读、不迁移、不兜底
layout/1 的库（S86 r6/r7 等冻结证据）由新二进制按库内 `rule_revision` 以 layout/1 比较器与 CC-007 旧载荷读法只读；写路径只有 bounded-layout；两种读法按版本分派，不接在同一 if-else 上作准入兜底（#799/#804）。layout/1 读法作归档读法不设退场，永不进写路径。
### 七、容量门
按固有事实重算：9011 单元 2,615,078 × u≈79 ≈ 25 GB 主库下界；(b) 全库估主库 33 GB、含 WAL 49 GB、双跑 99 GB。每库 `max_store_bytes` = 64 GiB，超帽 stale 停写、不清理；9011 双跑落外置卷 `/Volumes/AgentStorage`（不落系统盘），前置该卷可用 ≥ 148 GiB 并与同卷其他槽位协调，卷 UUID 进运行 manifest；不按代归档活库；已封存整库可整文件迁移，SEAL 记卷身份与 sha256。
### 八、验收与排序
#1488 段 A（layout/1）以公开根与 r6 逐位相同为锁；段 B（本布局）以 crosswalk oracle（§5.8 形式，含单元与比较总数相等）+ 双跑逐位一致 + 上界门（S86 g=1..86 逐代核，超界代数 0，r6 代入区分力倍数 1.10–1.68）验收；#1481 阶段 1 起写在 bounded-layout 上，9011 双跑基线 = 开跑时刻完整 `rule_revision` 串 + profile_hash + catalog_revision + 二进制 SHA + 五常数；#1405/#1419/#1420/#1393 不是 #1481 的 blocker，开跑后合入者不追加 9011 重跑义务。#1481 验收条「单条审计耗时与 G 的拟合斜率接近 0」改为两门并列：比例门（耗时与读量对 Δ_R(g) 成正比，系数在大 g 点相对变化 ≤20%，残差对 g 斜率 95% CI 含 0，SQLite 规模项 ≤ G^0.2）+ 上界门（g∈{1000,4000,9011} 或 M2 选点，超界代数 0；r6 外推代入倍数 4.4/4.9/18.0）；目标句「与 G 无关（只随本代新增量变化）」不改。

## 后果
公开根、manifest/batch3、CC-007 分型对象 object_id 全部换新，新 kind `CC-007.development_batch` 每代一条；CC-004/005/006 的 object_id 因 `rule_revision` 进配方也换新（§5.3 既定后果）；#1481 M4 的 Δ 步「同位置同摘要继承」在出生序下对未变前缀恒成立；前端 CC-007 卡片改三跳只读（分型对象 → 批记录 entry → 序列分片）、Delta 应用按出生序追加、Python 重建口径、tb02a oracle 随新修订重算；单元总数与比较总数与 r6 相等作 crosswalk 锁；S86 r6/r7 不重跑，继续作 #1394 Long 域证据与段 B 的 crosswalk oracle。

## 退场条件
若 APPROVED-SOURCE 日后明文规定快照数组次序或 CC-007 载荷，或 SPEC #1340 给「已实现后续发展」立独立轴或终止条件，以其为准并开 `bounded-layout/2`（或相应组件），旧库 AsKnown 保留不回填。

## 受影响代码清单
[`1489-AFFECTED-CODE.md`](../../.chanlun/review-results/issue1323-grilling-20261013/1489-AFFECTED-CODE.md)（基线 #1394 整合分支 `0fc0c165`）。段 A 归 [#1488](https://github.com/xy7365527-lang/NewChanlun/issues/1488)，段 B 归 [#1491](https://github.com/xy7365527-lang/NewChanlun/issues/1491)。

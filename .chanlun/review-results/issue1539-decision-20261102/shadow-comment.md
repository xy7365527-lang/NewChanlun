**P 类影子模式，待用户批复（返修稿 fu17-repair41f＋41P 同义订正，替代 [6544082577](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6544082577) 的问表）**（原判 T，2 轮审查 BLOCK 后转 P；40R 返修经 40Y 独审 BLOCK 四项，本稿修正这四项；本稿独审结论：41K 非作者独审 PASS，B1–B4 闭合、R1–R8 全过；其三项非阻断文书订正（N1–N3）已在 41P 同义订正，不改任何推荐值）

**考古**：A 已裁 8 点直接沿用——Q1 写者机制（ADR 0039:282/:290/:300；ADR 0056:40；C09.2/C09.7）、Q2 fill 原件归 E（INTERFACE-CONTRACTS:35；ADR 0051:310）、Q3 逐重 position 归 Bᵢ（INTERFACE-CONTRACTS:37；store.rs:75；C09.9）、Q5 重复（ADR 0051:266-267）、Q6 乱序（ADR 0051:268-269；ADR 0032:33）、Q7 费用（ADR 0032:31-33；C05.4）、Q8 更正（ADR 0032:37；C07.10）、Q14 跨模式权威排他（ADR 0056:59/:85/:88）。无 research/probe 缺口。

**照已批合同写定、不问你的**：减少责任的前件（C06.2 :295；C07.8 :312 非原子转移须「旧单不可再执行且历史累计完整」；INTERFACE-CONTRACTS:38 缺任何前件保原潜量）；三种模式在固定完整输入下的同模式 H1/H2（ADR 0056:63/:128）；真实外效不回退、不重发（AT-13 :425、C07.6）；账户查询结果以场所 adapter 原件进 E（ADR 0056:125）；费用 fill/command/period 三类因果（ADR 0032:33）。

需要你回答的只有下表，外加一项授权：本票落地时在 ADR 0038、ADR 0039 两份已裁正本文末各追加 append-only「修订（#1539）」节（原文不动，ADR 0030 不改）。

| 点 | 问法 | 待决点 | 出处 | 推荐值 |
|---|---|---|---|---|
| Q4 | C 待决 | 物理持仓由谁写，算不算独立真账？ | ADR 0038:53；C06.7（#1340:300）；C09.9（:338）；ADR 0056:59/:125；#1400 评论 6452358366；INTERFACE-CONTRACTS:40 | 不算独立真账、没有写者：它是「E 已 Recorded 的场所持仓/成交原件」与「Σ各重 B applications」两列的派生对账读数，只经 `Q.ReadEconomicView` 输出；差异只产 Pending/封闸，永不覆写 B、不从场所净仓重建成本。 |
| Q9 | B 确认或否决 | F 由谁写，减少责任所需证据以什么事实为载体 | C06.2（:295）；C06.7；C07.8（:312）；C10.5（:348）；ADR 0032:21；ADR 0033:31；INTERFACE-CONTRACTS:38；#1425:13 | F 唯一写者＝E，逐笔/累计成交报作原件进 E、同一函数归一；账户查询不推进 F。证据载体：simulation 依 bar 收盘终态（ADR 0033:31）；experiment/production 依 E 已 Recorded、种类由 adapter C10.5 能力证据声明的场所原件，所需各项分别证明。前件不改，能力声明不能取消前件。 |
| Q10 | B 确认或否决 | 恢复：哪份正本承载按证据类分述的恢复方法？ | ADR 0038:60/:81；ADR 0056:63/:128；ADR 0051:274-278；C07.6/C09.8/C09.9/C09.11/AT-13；ADR 0030:48；ADR 0039:26/:27/:147/:289；#1411:11/:13 | 修订落 ADR 0038/0039 两份正本本身（附件 A、B）：恢复＝快照＋重放持久日志；「外效未发生、回退」只适用于 simulation 且指工作态重跑后幂等命中、不删行；experiment/production 保留已持久的 X/E/Bᵢ 事实，在途订单只读对账、外部未知按最大责任保留、不重发。H1/H2 射程不变，三模式在固定完整输入（真实模式用受控场所夹具）下都验，AT-13 是附加约束。 |
| Q11 | B 确认或否决 | 分配记录（allocation）的写者 | ADR 0032:12-16；ADR 0051:320；#1770:34；tb03c_allocation.rs:340-349/:387-405 | 唯一写者＝E（Resolve 时按冻结 manifest 派生），B 唯一应用，X 不参与归属。不改分配算法。 |
| Q12 | C 待决 | 本票的 crosswalk 指什么，与迁移 crosswalk 怎么划界？ | #1340 :340/:358；ADR 0052:267；ADR 0028:71；ADR 0037:26/:29；#1406:43；ADR 0012:106/:108；ADR 0051:311 | 「同一外效在五类身份记录之间持久、可逆的四跳逐跳映射」：场所身份↔X 观测（X 库）↔E 原件（E facts）↔逐重分配（E `fill_allocations`）↔Bᵢ application；每跳由所在域唯一写者写，禁止第二权威写入口，物化汇总只能是可重算、不反写的只读投影（是否交付属实施范围）；`Q.ReadEconomicView` 逐跳拼接、缺跳报缺项。旧→新迁移 crosswalk 归 #1535。CONTEXT.md 补词条并 Avoid 与 IMPLEMENTATION-CROSSWALK、ADR 0037 oracle 混称。 |
| Q13 | B 确认或否决 | X 回报到 E 原件的字段义务 | ADR 0056:46/:125；ADR 0032:33；#1770:22/:34；book.rs:117-124；tb03c_per_leg.rs:28-31；tb03c_corrections.rs:125 | 共享来源/身份/因果外壳：原件级 `allocation_id`＝`external_fact_id`，`cause[]` 含 X 观测 `event_id`，不带 `family_id`/`target`。payload 按类别：命令相关成交只取投影形（带 `manifest_hash`、`command_id`），其 fill 费用/更正按原 fill 绑定前件；command/period 费用走 ADR 0032:33 既裁路径由 E 归属，不伪造成交投影；账户查询不推进 F、不写 B。 |
| Q15 | B 确认或否决 | 正本落点 | SKILL.md:70-72；ADR 0056:42；ADR 0051:320；ADR 0039:2/:485-487 | 新开 ADR（编号落地时取 main 下一空号，0058 已被 #1550 使用）；同批修订 ADR 0038/0039（Q10）；CONTEXT.md「运行边界」补「唯一真账」「crosswalk」。 |

- Q4 冲突：旧臂 `account_adapter.rs:8` 把场所净仓写成 `voice_qty[0]`，违 C06.7/C09.9，不作先例。
- Q10 冲突：ADR 0038:60 后句未按 evidence_class 限定，ADR 0039:27/:147/:289 把它写进 production launcher，与 C07.6/C09.9/AT-13/ADR 0030:48 方向相反。
- Q12 冲突：crosswalk 一词四义，CONTEXT.md 0 命中。
- Q13 冲突：book.rs:106-107 只收 TestOnly、x_execution_session.rs:22 含 `dry-run`，实装滞后，登记不作先例。

草稿：`/Volumes/AgentStorage/fu17-publish41p/draft.md`；考古修订件：`/Volumes/AgentStorage/fu17-publish41p/archaeology.md`；逐条回复：`/Volumes/AgentStorage/fu17-publish41p/review-response.md`；独审：`/Volumes/AgentStorage/fu17-review41k/REPORT.md`；前序：`/Volumes/AgentStorage/decisions/1539/`（两轮）、`/Volumes/AgentStorage/fu17-repair40r/`、`/Volumes/AgentStorage/fu17-review40y/`、`/Volumes/AgentStorage/fu17-repair41f/`。

状态：P / `needs-approval`，**尚未获用户批准**。请对本问表与同目录草稿、考古这一版本一次 `/graph-approve`；批准前不落正本、不接 #1539 blocks #1425 边、不改标签。

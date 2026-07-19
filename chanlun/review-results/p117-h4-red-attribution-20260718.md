# p117：H4 探针红灯逐案归因——T1 级别移位 6/37 命中且 owner≠B 的机制根因

日期：2026-07-18 ｜ 性质：**诊断工位·只读**（生产源码零改动、主仓零写入、零 git mutation、`rust/Cargo.toml` 零改动；唯一新增 = 诊断探针 bin + 本文，均在 worktree `/tmp/kimi-nest-mainline` 内）
工位：主线阶段 1 关② T1 验收 H4 红灯归因。任务书：37 案只命中 6 且 6 案 owner≠B，四假设并列（T1 实装有 bug／探针重建有误／设计预测错误／fiat 反读成立），用证据裁决。
证据链：H4 输出 `/tmp/p117_recheck.out`；H4 探针 `rust/src/bin/p117_s1a_level_recheck.rs`；裁定 `chanlun/escalate/bsp-terminal-endorsement-ruling-20260718.md`（下称裁定）；S1a 设计 `chanlun/review-results/p117-bsp-repair-design-s1a-20260717.md`（下称 S1a 图）；生产实装 `rust/src/theta_v0/classifier/nest.rs:476-553`；本工位新探针 `rust/src/bin/p117_h4_attribution.rs`（只读，autobins 自动发现）全量输出 `/tmp/p117_attr.out`（1,032 行，塔重放与 p116/p117 同骨架、终态 bit-equal）；v3 验收重放 dump 前缀 `/tmp/p92_ckpt_dump_v3.txt`（as_of≤1,750,000）；p112 原行 `/tmp/p112_full.txt`。
代码行号：以 worktree（分支 `kimi-nest-mainline-20260717`）2026-07-18 直读为准。

---

## 0. 结论先行

**四假设裁决（一句话）**：**设计预测错误为主因（S1a 图附录 A 门值无关性证明与 §0-6 命中预测被实测证伪），探针重建有误为次因（H4 探针两处精确等式 join 工件，把 30 案全压成 GATE_FAIL:tau、把 case=24 误报 owner_ne_b），T1 实装无 bug（与裁定逐行一致、v3 重放已凭其实产出证），fiat 反读不需要（机制层已足解释，且被实测反向否决）。**

**根因三层堆叠（全部实测锁定，详见 §3）**：

1. **包络口径分叉（最深）**：`classify_relation`（`center.rs:209-217`）消费 `dd/gg` 外缘包络；中枢延伸吸收会改写 `dd/gg`（`recursive_tower.rs:741-742`）。D2 的 per-run 投影 seed 的 `dd/gg` = 前 3 段窗（`level_view.rs:365-395` own_center 重算），全局 `levels[0].centers` 的 `dd/gg` = 全生命周期（含延伸段）。同一 (prev,B) 中枢对：seed 链读 Trend、detect 链读 **LevelExpansion**——全局 τ 门 `gate[B]=None` 实测 **30/37**（仅 case 2/6/10/16/18/21/25 开，`/tmp/p117_attr.out` ATTR_GATES 逐案）。S1a 图附录 A「per-run 与全局门值恒等」的证明只覆盖了「同一链切片」的情形，没有覆盖延伸对 `dd/gg` 的改写——**门开预测 37/37，实测 7/37**。
2. **frontier 吸收（归属 prev）**：延伸判据（`recursive_tower.rs:739` `units[j].lo<=zg && units[j].hi>=zd`）把**破核腿自身**（起于核心内、破出核心）也吸进中枢——实测 37/37 案 B 的 detect 端 ≥ c 首破腿终点（ATTR_SEAL match=37/37、`absorbed_past_turn=true` 37/37）。于是 c 腿起点处 B「未封」（`nearest_confirmed_center_idx` 按终态 `end_index<=seg.start` 取不到 B），被判给 prev；prev 是块首（6/7 门开案）⟹ `gate[prev]=None` ⟹ 一类不触发。唯一 ≥3 中枢趋势块案（case 25）判给 prev、门开、但面积败 → turn 上 zero-bit struct_break（实测 `point present bits=000000 sbd=Some(Short) conf=false`，判据复刻 AreaFail 吻合）。
3. **封后延迟（破 B 点在 t* 右）**：真「破 B」一类点须 B 封（一整腿全出核心）+ 后续同向腿再破核——实测仅 4/37 案存在：case 2 buy1@978436（t* 右 +590）、case 6 buy1@1545443（+19）、case 10 buy1@1963412（+84）、case 21 buy1@3819202（turn+587，事件未确认）。立即案 t*=t3（三买回试终点）⟹ 破 B 点几乎必然出窗，且**裁定 T1 裁决 2 的「时点 ≤ t*、无前视入证」条款堵死窗口右延**。

**实测命中构成（6/37，无一是裁定 §6.1-2 语义的「破 B 一类点」）**：5 个二类点（buy2/sell2，owner=prev）+ 1 个三类点（sell3，owner=B）。按裁定字面背书语义衡量，合法「破 B 一类」命中 = **0/37**；裁定 T1 的复议触发条款（「C-b 窗口身份保证被实测证伪——窗口内最早点非『破 B』实例出现」）**触发成立**，复议材料见 §5。

**H4 红灯的形式读数归因**：`rescued=0/37` 全部由探针门链 join 工件压出（探针 verdict 链先查门链后查 hit，门链 join 全灭 ⟹ 6 个 hit=Some 案也被压成 GATE_FAIL:tau）；但绕过门链后生产 hit 实测也只有 6/37 且非一类——**探针误差解释「红灯形状」，设计错误解释「红灯实质」**。

---

## 1. 测量仪器与自校验账本（090：先证仪器可信，再用其读数）

新探针 `rust/src/bin/p117_h4_attribution.rs`（只读；塔重放/事件收集/pair 上下文骨架逐字复制 p117 探针；终态与 batch bit-equal，p116 P116_BIT_EXACT 锁定）。凡 `pub(crate)`/private 不可直达的生产函数逐字复刻，并全部带生产实测自校验：

| 自校验 | 结果 | 含义 |
|---|---|---|
| ATTR_REPLICA_TSTAR | **30/30** | R1 `trend_confirm_time`（`level_view.rs:535-626` 私有）复刻在 30 个 confirmed 事件上逐位复现 `event.turn_source`——R1 归因（§6）可信 |
| ATTR_SEAL_CHECK | **37/37** | 延伸吸收复刻扫描的 seal 端逐位等于 `levels[0].centers` 实测 `end_index`——「延伸吸收破核腿」机制（§3.2）被数据逐案证实 |
| ATTR_Q4 | extended=**37** exact=0 recut=0 | 37 案 `b_center_missing` 全部是「seed 未延伸 vs detect 延伸」的等式工件（§7），无一 #89 违例 |
| 生产 hit 对拍 | 37/37 一致 | 探针 `terminal_bits_at_event`（生产单一来源）读数与 H4 输出逐案一致（hit=true ×6：case 6/15/18/24/31/36） |
| ATTR_REPLICA_JUDGE | hit_confirm=0/6、zerobit=6/10、FORK 10 行 | **仪器局限的如实登记，非生产分叉**：6 个 hit 点全部是二/三类点（§5），一类判据复刻对其不适用（返 GateClosed 属预期——此读数本身就是「命中非一类」的发现）；4 行 zerobit 复刻不符（case 13×3、case 20×1）全部是「该点 struct_break_dir 与事件方向相反、属别的趋势」——复刻只按事件方向评估所致。`judge_replica` 有效域 = 与事件同向的一类路径，case 25 的 turn 点复刻 AreaFail 与实测 zero-bit 精确互证 |

pair 审计：`ATTR_PAIR_AUDIT pairs=1388 turn_collisions=0`（#105 turn 键唯一，与 p117 一致）；provider 健康计数全零。

---

## 2. H4 输出读数复核（形式层）

`/tmp/p117_recheck.out`：`rescued=0/37`（30 案 `GATE_FAIL:tau`、7 案 `GATE_FAIL:event_missing`）；`P117_RECHECK_IDENTITY hits=6 owner_ne_b=6`；37 案全带 `b_center_missing` WARN；`P117_RECHECK_CA exact_hits=0/37`。

两个必须先剥离的探针工件（详 §4）：(i) 30 个 `GATE_FAIL:tau` 标签全部由门链 join 工件产生（`gate_open=false` 30/30 是 join 失败，不是门真关）；(ii) `owner_ne_b=6` 中 case=24 是同一等式工件的误报（owner 实即 B），另 5 案 owner=prev 是真实读数。剥离后剩下的生产真读数：**hit=Some 6/30（事件在场案）、7 案无 confirmed 事件**——与本文探针生产对拍逐位一致。

---

## 3. 根因机制三层（代码锚 + 实测）

### 3.1 第一层：包络口径分叉 ⟹ 全局 τ 门关 30/37（最深根因）

- `classify_relation`（`center.rs:209-217`）：`next.dd > prev.gg ⟹ UpContinuation`、`next.gg < prev.dd ⟹ DownContinuation`、否则 `LevelExpansion`——**输入是 `dd/gg` 外缘包络，不是核心 `zd/zg`**。
- 延伸吸收（`recursive_tower.rs:739-743`）：触及核心的后续段被吸进中枢，`end_index/dd/gg` 随之改写（核心不动）。故 detect 中枢的 `dd/gg` = seed 三段 + 全部延伸段的**生命周期包络**。
- D2 侧 per-run 投影 seed 的 `dd/gg` = **前 3 段窗**（`level_view.rs:341-345/:388-389`：units 取窗前三段，`compute_dd/gg` 重算；核不等时仅 `zd/zg` 继承携带核，`dd/gg/坐标` 仍按前三段）。
- ⟹ 同一对中枢 (prev,B)：seed 链（紧包络）上 `B.dd > prev.gg` 易成立读 Trend；detect 链（胖包络）上不等式被两侧的延伸吸收破坏读 LevelExpansion。**实测：全局 `gate[B]=None` 30/37**（ATTR_GATES 逐案；开的 7 案 = 2/6/10/16/18/21/25，其中 6 案 prev 为块首、仅 case 25 为 ≥3 中枢趋势块）。
- 代表例 case=1（ATTR_B）：seed B=(866696,867022, gg=506398e6) → detect B=(866696,**867223**, gg=**508255e6**)——c 破核腿的高点被吸进 B 的 gg；`UpContinuation` 要求 `B.dd(502907e6) > prev.gg`，prev 侧同样被 b 离开段起点抬高 ⟹ LevelExpansion ⟹ `gate_b=None`。
- **对 S1a 图的打击**：附录 A 证明的前提是「per-run 种子链 = 全局链的连续切片 ⟹ 关系标签逐对相同」——该前提只在 `zd/zg` 层面由 #89 担保（实测 37/37 `zg_eq=zd_eq=true`），`dd/gg` 层面不成立（延伸改写），而 `classify_relation` 偏偏只读 `dd/gg`。**「门差只能来自链级别不同（L0 中枢链 vs L1 中枢链）」的修正结论需要再修正：即使同在 L0 中枢链，seed 包络（前三段）与 detect 包络（含延伸）的口径差已足以让门值分叉。** p112 的「decompose 上下文差异」原归因经此实测部分复活（差异真实存在，但机制是包络口径而非 run 切片）。

### 3.2 第二层：frontier 吸收 ⟹ c 破核腿判给 prev

- 延伸判据 `units[j].lo <= c.zg && units[j].hi >= c.zd`（`recursive_tower.rs:739`）：破核腿起于核心内、破出核心，必然触及核心 ⟹ **被吸进它所破的中枢**。实测 37/37：B 的 detect 端 = c 首破腿终点（立即案即 turn），`absorbed_past_turn=true` 37/37、复刻 seal 端与实测逐位相等（ATTR_SEAL_CHECK 37/37）。
- BSP 归属（`signal.rs:1078` → `nearest_confirmed_center_idx` `signal.rs:211-218`，终态数组 `end_index <= seg.start`）：c 腿起点处 B 的终态端 > 腿起点 ⟹ 取不到 B ⟹ **判给 prev**（ATTR_TURN 37/37 owner=PREV）。
- prev 侧门：`gate[prev]=Some` 要求 prev 非块首——实测 7 个门开案中 6 案 prev=块首（`prev_is_block_start=true`）⟹ `gate_prev=None` ⟹ 一类不触发、turn 上无点；唯一例外 case 25（3 中枢块 (8062,8064)，`gate_prev=Up`）→ 一类候选成立但面积 C≥A → **turn 上 zero-bit struct_break**（实测 `bits=000000 sbd=Some(Short)`；判据复刻 AreaFail 逐环吻合：gate✓ broke✓ A=(3987807,3988528)✓ 037:20✓ 面积✗）。
- ⟹ S1a 图 §0-6「破核腿即 t3_ext 离开腿（37/37 t3_ext=1）⟹ S3 过 ⟹ 一类点」的推理链在「破核 ⟹ 产生对 B 的一类点」处断裂：**破核几何成立（探针 broke_core=true 30/30 属实），但破核腿的归属中枢不是 B**。

### 3.3 第三层：封后延迟 ⟹ 破 B 点结构性晚于 t*

- B「封」= 首条整腿全出核心（non-extension 哨兵，`recursive_tower.rs:736`）；只有 B 封了之后到达的同向破核腿才能在起点处见到 `B.end <= leg.start`、被判给 B。
- 立即三买形态：c 破核腿（被吸收）→ 回试腿（三买确立 t3=t*；回试腿低点若入核心则继续被吸收）→ 尔后才可能有整腿出核心 + 再破核腿 ⟹ **破 B 一类点 ≥ t* 之后至少 1 腿**。
- 实测存在破 B 一类点的仅 4/37：case 2 buy1@978436（t*+590）、case 6 buy1@1545443（t*+19）、case 10 buy1@1963412（t*+84）、case 21 buy1@3819202（事件未确认，turn+587）——**全部在窗口右界外**；case 16/18/25 门开但无破 B 候选（封后无再破核段/反转已至，ATTR_BPOINT none）；30 个门关案结构性不可能有（§3.1）。
- 裁定 T1 裁决 2 选 C-b 的理由之一是「时点 ≤ t*，因果序为正，无前视入证」——**该条款同时堵死了窗口右延救回这些点的路径**；C-b 在实数据上的产量天花板被这一结构延迟直接压死。

### 3.4 旁证：二三类点的 τ 门独立性（命中点的真实来历）

三类（`signal.rs:1314-1324`：`judge_third` 在 `if let Some((pos,dir)) = gate_dir` 分支**之外**逐段对判定）与二类（递归组装层 `extract_second_signals`，`signal.rs:833-863`，消费 RMove 塔次级别结构，不读 `center_gate`）均**不经 τ 门**——这解释了门关案（gate_b=None）的账本上为何仍有 confirm_side 点（sell3/buy3/buy2/sell2），以及 6 个命中点的真实身份（§5）。

---

## 4. 必答 Q1：miss 案（代表 3 案：case=1/2/37）逐案证据

（30 个 miss 案的分类总表见本节末；每案完整 dump 行在 `/tmp/p117_attr.out`。）

### case=1（turn=867223，Short，窗口 [867071,867240]，hit=None）

- 窗内 `levels[0].bsp` 点：@867240（buy3，conf=false）、@867433（buy3，conf=false）——**真的没有任何 confirm_side(Short) 点**（ATTR_PT）。侧不匹配不是原因：窗内两点是买侧三类，`confirm_side` 方向隔离正常。
- 窗口外最近点：左 @866549 sell3（conf=true，**左 522 bar**，owner=OTHER(865778,866261)）；右 @868045 sell2（conf=true，**右 805 bar**，owner=OTHER(867223,867481)）——都属**别的中枢的趋势**，非 B 非 prev。
- 根因归类：**该级别账本对 B/prev 真的无点**。机制链：gate_b=None（§3.1，B.gg 506398e6→508255e6 变胖致 LevelExpansion）⟹ 对 B 一类结构性不可能；c 腿判给 prev 而 prev 无趋势块（block=None）⟹ 对 prev 亦不触发（ATTR_TURN replica=GateClosed、point=absent）；破 B 候选全书不存在（ATTR_BPOINT none/none）。

### case=2（turn=977720，Long，窗口 [977629,977846]，hit=None）

- 窗内点：@977846（sell3 owner=B，conf=false）——无 confirm_side(Long) 点 ✓。
- 窗口外：左 @974941 buy2（**左 2,688 bar**，owner=OTHER）；右 @978092 buy2（**右 246 bar**，**owner=B**）；且 @978436 buy1（sbd=Some(Long)，**右 590 bar**，owner=B）、@978701 buy1（右 855，owner=B）。
- 根因归类：**窗口右界太早**——`gate_b=Down` 开（§3.1 七案之一），破 B 一类点真实存在但最近一个在 t* 右 590 bar（封后延迟 §3.3）；prev=块首致 c 腿无点（§3.2）。侧不匹配：无。

### case=37（turn=4324005，Long，窗口 [4323850,4324141]，hit=None；兼 Q4 案）

- 窗内点：@4323850（sell3 owner=PREV，conf=false）、@4324141（sell3 owner=B，conf=false）——无 confirm_side(Long) 点 ✓。
- 窗口外：左 @4322097 buy2（**左 1,753 bar**，owner=OTHER）；右 @4324919 buy3（**右 778 bar**，owner=OTHER）——均属别的趋势。
- 根因归类：**真的无点**（gate_b=None；BPOINT none/none）。`b_center_missing` 性质见 §7（EXTENDED 工件）。

### 30 个 event-present 案的根因分类（探针逐案归类，ATTR_Q1_SUMMARY + 逐案行）

| 类别 | 案数 | 案号 | 含义 |
|---|---:|---|---|
| 窗内命中 owner=B | 2 | 18（buy3@2669312）、24（sell3@3971887） | 均为三类反转点，非一类（§5） |
| 窗内命中 owner=prev | 4 | 6、15、31、36 | 均为二类点（§5） |
| 破 B confirm 点在 t* 右 | 7 | 2（+246 buy2/+590 buy1）、6*、7（+332 sell2）、10（+84 buy1）、13（+546 sell2）、14（+476 sell2）、23（+222 buy2）、31*、35（+403 sell2）（* = 同时窗内命中 prev，计入上行；右延点除 case 2/10 外均为二/三类） | 窗口右界太早（且无前视条款禁右延） |
| 破 B 候选全书不存在 | 17 | 1、3、4、5、8、9、12、16、17、19、20、25、28、29、30、33、37 | 真的无点：15 案 gate_b=None（§3.1）+ case 16/25 门开但无候选（§3.3） |
| 破 B 仅 zero-bit 候选 | 0 | — | 无 |

- 「窗口界错（c_start 太晚/t* 太早）」只覆盖 7 案且其中 5 案右延点仍非一类；「侧不匹配」0 案（窗内反向点 confirm_side 为假是方向隔离的正常表现，p112:127-129 的反向位点 2 案同此）；「该级别账本真的无点（对 B/prev）」17 案 + 窗内仅无关点若干——**主导类别，机制 = §3.1 包络口径分叉**。

---

## 5. 必答 Q2：6 个 owner_ne_b 命中案逐案证据 + 背书语义判定

### 5.1 逐案事实（生产 hit 读数 + 账本点 dump + owner 回读）

owner 回读口径 = 生产 `nearest_confirmed_center_idx`（腿起点、终态数组，与 H4 探针同式；对一/三类路径即生产判据输入本身；对二类路径为其结构锚的最近似读数——5 案均落 prev，与「一类离开破 prev、回拉不创新极值」的二类结构语义一致）。

| case | hit@source_index | bits | 类别 | owner（回读） | 事件 B（seed） | owner=B？ |
|---|---|---|---|---|---|---|
| 6 | 1545297（=#105 turn） | 010000 | **buy2** | prev（1544583,~1544886 延伸） | (1544906,1545202) | **否** |
| 15 | 2178474（=turn） | 000010 | **sell2** | prev（2177559,~2178107 延伸） | (2178124,2178454) | **否** |
| 18 | 2669193（=turn） | 010000 | **buy2** | prev（2667464,2667898） | (2668255,2668863) | **否** |
| 24 | 3971887 | 000001 | **sell3** | B（3971101,~3971708 延伸） | (3971101,3971402) | **是**（H4 误报 ne_b，§7） |
| 31 | 4172278（=turn） | 000010 | **sell2** | prev（4171308,4171725） | (4171769,4172028) | **否** |
| 36 | 4321214（=turn） | 000010 | **sell2** | prev（4319654,4320709） | (4320709,4321166) | **否** |

- **这些点是哪类 BSP**：5 个二类（buy2/sell2）+ 1 个三类（sell3）。**无一是一类（buy1/sell1）**——一类判据复刻对 6 点全返 GateClosed（§1 自校验表），即「若按一类路径评估这些坐标不产一类点」，与其 bits 实测互证。
- **落在窗内是巧合还是结构相关**：**结构相关，非巧合**。5 个二类点坐标恰 = #105 turn（c 首腿终点）：D2 事件的三买结构（leave 破 B + retest 不入核心，R1 T3）与「趋势 [..,prev] 一类离开后、回拉不创新极值」的二类结构是**同一几何的两种读法**——b 离开段破 prev（一类离开）→ B 形成 → c 首腿再探极值但未破 prev 的一类低点/高点 ⟹ 二类端点落在 c 首腿终点。C-b 窗口抓住的是同一几何的另一读法。case 24 的 sell3@3971887 = 顶背后价格反穿 B 核心 + 回抽不入的三卖（对 B 的反转确认），同样由事件结构决定。
- **v3 旁证**：case 6 在 v3 验收重放中已成证（`/tmp/p92_ckpt_dump_v3.txt`：`CKPT caliber=A/B as_of=1750000 exec=1 top=1 side=Long bucket=trend ids=1:1545424:1545202-1545424` ×2）——生产装配凭 C-b 窗口内这个 buy2@1545297（owner=prev）通过终端门并成证；case 1-5/7（前缀覆盖范围内）无 CKPT，与 hit=false 逐案互证。**T1 实装通路端到端工作正常**。

### 5.2 按裁定 T1 背书语义的判定（复议条款被触发后）

- 裁定的背书语义（T1 裁决 2 + §6.1-2 + 037:22 读法）：C-b 窗口最早 confirm_side 点应是「**c 责任单元**的 confirm_side」——c 首个破核腿的落点、判决中枢 = B 的**一类点**（027:66）。
- 实测 6 点：5 个是趋势 [..,prev] 的**二类**点（其力度比较结构 = prev 的一类离开 vs 回拉，不是本事件的 c vs b）；1 个是反转后对 B 的**三类**卖（结构确认顶，不是背驰比较点）。**按裁定字面语义，6/6 都不算合法背书**；C-b 窗口的 `confirm_side` 谓词（bits 析取，`types.rs:216-221`）在实装上无法区分一/二/三类——这是裁定语义与实装谓词之间的真实缝隙，不是实装错误（裁定裁决 2 明文谓词即 `confirm_side`）。
- **复议触发条款成立**：裁定「不回滚条款」第 3 条 + 「醒后复议通道」T1 项——「C-b 窗口身份保证若被实测证伪（出现窗口内最早点非『破 B』的实例），走复议通道」。实测 5/6 owner=prev + 6/6 非一类 = 证伪实例。§6.1-2 证明的两处漏洞（实测定位）：(i) 「窗口内首个破核点的判决中枢必 = B」未计延伸吸收（§3.2：破核腿被吸进 B ⟹ B 未封 ⟹ 判给 prev）；(ii) 证明只讨论一类点，未计二三类点的 τ 门独立性（§3.4：门关时窗内仍可存在二/三类 confirm_side 点，且它们按 `min_by_key` 最先被取到）。
- **本工位的语义判定**：维持裁定字面语义——背书点应为「破 B 一类点」；6 命中不计入合法背书（合法命中实测 0/37）。同时呈交反方事实供复议权衡：6 点与事件同坐标/同方向/同趋势块，结构相关非巧合（§5.1），且 case 6 已凭此成证——若复议选择把背书语义放宽到「同块同向二/三类点」，则须以新裁定文书重述身份保证并给出教义依据（本工位不越权拍板）；若收紧为「owner=B ∧ buy1|sell1」，则 C-b 产量实测为 0/37，T1 的修复目标须整体重议（破 B 一类点的封后延迟 + 无前视条款构成硬约束，§3.3）。

---

## 6. 必答 Q3：event_missing 7 案——配对无缝隙，R1 全合取 T5 死亡

- **配对缺口不存在**：7 案（#11/#21/#22/#26/#27/#32/#34）的 D2 pair 全部由「#105 turn 重建」精确配到（collisions=0），事件键（side ∧ seg_a ∧ interval_b.0=c_start）精确匹配到 **unconfirmed** 事件（H4 的 `event_unconfirmed_r1` WARN 已示）——硬锚表坐标未过期，R1 坐标迁移不影响配对（事件 `seg_a`/`interval_b.0` 与 #105 口径逐位一致）。v3 dump 前缀 as_of≤1,750,000 不覆盖 7 案坐标（turn>2.0M），改用生产 `provide_nest_candidate_events` 直核（本探针与其同骨架同终态）。
- **未确认根因（R1 复刻逐案分解，复刻件经 30/30 t* 对拍验证）**：7/7 案 `reason=T5_turned_false`，且全部为「T4 ✓ → T3 三买已确立（t3 与 p112 的 `t3_ext_hit` 逐位相同）→ 扫描首个 t≥t3 腿端处 T5-OR 已为假、同点 T2 破极值已真」：

| case | t3（leave,retest） | t5_fail_at | extreme | env_a | env_c_final |
|---|---|---|---|---|---|
| 11 | (2063914,2064134) | 2064134（=t3） | ✓ | (3371431e6,3485000e6) | (2927800e6,3882712e6) |
| 21 | (3818615,3818832) | 3818832 | ✓ | (9497029e6,9648895e6) | (9272790e6,9516665e6) |
| 22 | (3858568,3858607) | 3858607 | ✓ | (9484722e6,9646538e6) | (9050000e6,10835300e6) |
| 26 | (4015179,4015402) | 4015402 | ✓ | (7907382e6,8120879e6) | (8078243e6,8298571e6) |
| 27 | (4018257,4018392) | 4018392 | ✓ | (8427703e6,8494825e6) | (8348067e6,8477125e6) |
| 32 | (4189334,4189454) | 4189454 | ✓ | (11617885e6,11758800e6) | (11587871e6,11873807e6) |
| 34 | (4200339,4200626) | 4200626 | ✓ | (11732153e6,11794798e6) | (11500000e6,11796253e6) |

- 机制：R1 的 T5-OR（同色面积 ∨ 黄白线峰 ∨ 柱峰，`level_view.rs:600-613`）三通道的 c 侧 proxy 在 [c_start, t] 上**单调累积**，而 t3（三买确立）最晚——到 t3 时 c 侧面积/DIF 峰/柱峰已全部累过 A 侧参照（「真→假单调，转假即终假」`:613`）。p112 的 #105 单腿窗口 T5 实测 7/7 过（`t5=1`），R1 渐进窗口把回试段后的全部 bar 纳入累积（回试期 hist 未回正色，同色面积继续累）⟹ 边际最小的案（#22 面积差 3%、#27 差 1.3%）与延迟案（#11/#22/#32，t3 晚 2.8-3 万 bar）率先死亡，#21/#26/#34 同机制死亡。
- 对设计的打击：S1a 图 §0-6 的「5 延迟案经 C-b 救回」预测隐含「事件 confirmed」——3 个延迟案（#11/#22/#32）连事件都没有，C-b 无从救起；4 个立即案（#21/#26/#27/#34）同。这是 R1 确认语义（T5-OR 渐进窗口 vs T3 最晚性）的独立问题，与 T1 级别移位无关，按证据如实登记归 R1 口径复议域，不并入本归因的处置建议（见 §9 备注）。

---

## 7. 必答 Q4：case=37 `b_center_missing` = 探针等式工件，非 #89 违例

- **实测**：37/37 案 `verdict=EXTENDED`（ATTR_Q4：extended=37 exact=0 recut=0）——seed 与 detect **同 start_index、同 zd/zg**（37/37 `zg_eq=zd_eq=true`），仅 `end_index/dd/gg` 异（延伸吸收）。case=37：seed=(4323525,4323850, dd=10292600e6) → detect=(4323525,**4324005**, dd=**10265917e6**)——c 破核腿被吸收（端 4323850→4324005=turn，dd 收下腿低点）。复刻 seal 端与实测 37/37 逐位相等。
- **机制定性**：H4 探针的 gate join（`p117_s1a_level_recheck.rs:502` `centers0.iter().position(|c| *c == ctx.last)`）拿**投影 seed**（`level_view.rs:365-395`：own_center = 窗前三段重算，天然未延伸）与**终态 detect**（`recursive_tower.rs:739-743`：延伸改写 end/dd/gg）做 `Center` 全字段等式——结构性必败，与 #89 无关。#89 的标的是「塔 compose 携带核 ≡ 重算 detect」（`level_view.rs:359` 条款 1），携带核即 detect 本体（同对象）；探针 WARN 文案「#89 carried≡detect 违例嫌疑」**误指**——seed 的 own_center 不是携带核。
- **派生误报**：同一等式过严还造成 `P117_RECHECK_IDENTITY owner_ne_b` 的 case=24 误报（`p117_s1a_level_recheck.rs:794-796` `centers0[owner] == ctx.last`）——owner=(3971101,3971708) 与 B=(3971101,3971402) 同 start 同核、仅延伸端异，实即同一中枢（§5.1 表）。另 5 案 owner=prev 为真实读数（start 即不同中枢），非探针误差。
- **结论**：塔数据无违例；**探针回读扫描的等式判据是复刻 bug**（两处：:502 gate join、:794 identity 判据；修法 = 按 start_index 定位 + 按 start_index 判同，见 §9-②）。

---

## 8. 必答 Q5：实装 vs 裁定语义一致性终审 + 设计预测清算

### 8.1 T1 实装与裁定逐行一致性（实装 bug 排查）

| 裁定条款 | 实装 | 一致性 |
|---|---|---|
| T1.1 账本移位 ℓ→ℓ-1（ℓ≥1⟹Some(ℓ-1)，ℓ=0⟹None） | `nest.rs:487-492` `event_bsp_book_level` 逐字同 | ✓ |
| T1.2 窗口 = `[c_start, t*]` = `interval_b.0..=turn_source` 闭区间 | `nest.rs:525-533` `c_start <= p.source_index && p.source_index <= turn_source` | ✓ |
| T1.2 最早 confirm_side 点（确定性） | `nest.rs:532` `min_by_key(source_index)`（平局取迭代序首个，与头注声明一致） | ✓ |
| confirm_side 方向谓词 | `nest.rs:530` `p.bits.confirm_side(side)` → `types.rs:216-221`（Long=conf_plus=buy 析取、Short=conf_minus=sell 析取） | ✓ |
| side 映射（Up→Short 顶背卖／Down→Long 底背买） | 事件构造处 `level_view.rs:665-668`（Down⟹Long、Up⟹Short）；探针 `side_of_dir` 同映射 | ✓ |
| C-a 保留为敏感性对照（不作默认） | `nest.rs:496-505` `TerminalMatch::Exact` 保留、p92 bin 常数 `TERMINAL_MATCH=CWindow`（`p92_nest_replay_postruling.rs:1031`） | ✓ |
| 单一来源/旧查法删除（不回滚条款） | `terminal_bits_new` 委托 `terminal_bits_at_event`（`p92_nest_replay_postruling.rs:1036-1041`），旧 `levels[ℓ]` 位格等式已删 | ✓ |

**终审结论：`terminal_bits_at_event` 与裁定 T1 语义逐行一致，实装无 bug。** 端到端旁证：v3 重放 case 6 成证（§5.1）；生产 hit 读数两探针互拍 37/37 一致。

### 8.2 S1a 设计的 32+5 预测在当前实装下应命中多少 + 错在哪

- **预测 vs 实测**：设计预测 C-b rescued=37/37、C-a=32/37（§0-6）。实测（生产单一来源）：**C-b=6/37、C-a=0/37**（H4 `P117_RECHECK_CA exact_hits=0/37` 复证）；按裁定字面背书语义（破 B 一类）**合法命中=0/37**；破 B 一类点真实存在者 4/37 且全部在 t* 右（§3.3）。
- **设计预测错在四处（逐条实测证伪）**：
  1. **附录 A 门值无关性证明错误** ⟹ 「L0 门链门开 37/37」实测 7/37（§3.1；`dd/gg` 包络口径被延伸改写，`classify_relation` 只读 `dd/gg`）。这是最大一击——设计的整个「移位后 L0 门开」前提在 30/37 案上不成立。
  2. **§0-6「32 立即案 C-a 即可命中（t*≈破核腿终点=L0 一类点坐标）」双重错**：t*≠破核腿终点（立即案 t*=t3=回试终点，如 case 1 turn=867223 vs t*=867240）；破核腿终点处的一类点判给 prev 而非 B（§3.2 frontier 吸收，37/37 实测）。
  3. **§6.1-2 最早性身份保证证明漏两条**：延伸吸收（判给 prev）与二三类点 τ 门独立性（最早点被二/三类抢占）（§5.2）。
  4. **事件 attrition 盲**：7 案 R1 全合取未确认（§6），「5 延迟案经 C-b」中 3 案无事件可救。
- **不因设计错误回滚实装**：实装是裁定语义的忠实镜像（§8.1）；错的是设计对账本内容与门值的预测模型。处置走「设计回票 + 裁定复议」（§9），不动实装。

---

## 9. 四假设裁决书与处置建议

### 裁决

- **H1「T1 实装有 bug」：不成立。** §8.1 逐行一致 + v3 成证旁证 + 双探针 hit 互拍。
- **H2「探针重建有误」：成立（次因，解释红灯形状）。** 两处精确等式 join 工件（§7：gate join :502、identity :794）+ verdict 链门链优先序把 6 个 hit=Some 案压成 GATE_FAIL:tau；WARN 文案误指 #89。hit 读数本身与生产一致（fork_mismatch=0），owner 回读口径与生产同语义——误差边界清晰。
- **H3「设计预测错误」：成立（主因，解释红灯实质）。** §8.2 四处证伪；最深一层是附录 A 未计延伸对 `dd/gg` 的改写（§3.1）。
- **H4「fiat 反读成立」：不需要（被实测反向否决）。** 37 案窗口邻域在 `levels[0].bsp` 确有点（6 命中 + 7 案 t* 右破 B 点 + 邻域二/三类点），事件级别归属 L0 中枢链无误；「37 案出口是 PanDivCert」的 fiat 退路解释不了这些实测点。级别移位方向（向 `levels[ℓ-1]` 找）被 6 命中 + case 6 成证佐证为合法——问题在 C-b 窗口的身份保证与门值预测，不在移位本身。

### 处置建议

- **① 实装：不动。** `nest.rs:476-553` 与 p92 委托零改动（证据 §8.1）。
- **② 探针修复（`p117_s1a_level_recheck.rs`，诊断仪器）**：(a) gate join 改按 `start_index` 定位（:502），WARN 文案改「seed 未延伸 vs detect 延伸等式工件」；(b) 门链复刻改 owner-aware——先取 c 腿归属中枢（终态 end 口径 nearest），再查其门值与判据（现状直接拿 pair.last 查门，与生产归属语义不符）；(c) identity 判据改按 `start_index` 判同（:794）。修后重跑预期：`gate_open` 实测 7/37（2/6/10/16/18/21/25；按 c 腿 owner 口径则 1/37＝case 25）；rescued 读数不变（hit 是生产单一来源，本就未受门链污染）——**修复只恢复门链诊断的诚实性，不改 hit 结论**。
- **③ 裁定复议（呈交材料 = 本文 §3/§5/§6）**：T1 复议触发条款已成立（§5.2）。呈三案请裁：(i) 维持字面语义（破 B 一类）⟹ C-b 合法产量 0/37，T1 修复目标须重议（破 B 点封后延迟 + 无前视条款 = 硬约束，§3.3）；(ii) 放宽背书语义到「同块同向二/三类点」⟹ 须新裁定文书重述身份保证 + 教义依据，产量实测 6/37（邻域 7 案 t* 右点仍出窗，天花板如实）；(iii) 重议窗口/事件口径——注意 T1 裁决 2 的无前视条款堵死 t* 右延，任何右延方案须先处理该条款。附带呈交：S1a 图附录 A 证伪（§3.1）。
- **④ 设计回票（S1a 施工图）**：§0-6 预测（37/37、C-a 32）回票——实测 6/37 且非一类；§1.4 事实链第 3 环与附录 A 按「seed 包络（前三段）vs detect 包络（含延伸）」重写；§6.1-2 身份保证证明补延伸吸收与二/三类门独立性两条；§3.5「L0 门链门开 37/37」实测 7/37（按 c 腿 owner 口径 1/37）。
- **⑤ p116 C2 对账义务（裁定 T4-3 / L3）的附加注记**：TERM 谓词平移到级别移位查法时，须注意本归因——若谓词按「窗口内 confirm_side 点」计数，会把二/三类点算入（6/37 即此构成）；建议谓词加 `bits.buy1|sell1` 限定（一类）或按复议结论定型后再同步，否则 C2 归因口径与裁定背书语义错配。
- **⑥ R1 确认语义独立登记（不并入本归因处置）**：7 案 event_missing 的机制（T5-OR 渐进累积在 t3 前死亡，§6）属 R1 口径域，按证据如实呈交，由编排者决定是否另立 R1 复议；本归因不以此要挟 T1 任何条款。

---

## 10. 边界与纪律声明

- 零生产源码改动；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入；零 git mutation；`rust/Cargo.toml` 零改动；`rust/src/theta_v0/` 零改动。
- 新增文件仅两件（均在 worktree `/tmp/kimi-nest-mainline` 内）：诊断探针 `rust/src/bin/p117_h4_attribution.rs`（只读仪器，autobins 自动发现）+ 本文档。
- cargo 使用：p116 禁令已解除（任务书明示）；编译/运行均在 worktree `rust/target` 内；后台 v3 重放未受干扰（其 dump 只读引用，as_of≤1,750,000 前缀）。
- 090：本文全部结论具名行锚——dump 行（`/tmp/p117_attr.out` 的 ATTR_CASE/ATTR_B/ATTR_GATES/ATTR_SEAL/ATTR_TURN/ATTR_PT/ATTR_LEFT/ATTR_RIGHT/ATTR_BPOINT/ATTR_HIT/ATTR_R1/ATTR_*_SUMMARY，`/tmp/p117_recheck.out` 的 RECHECK/P117_* 行，`/tmp/p92_ckpt_dump_v3.txt` 的 CKPT 行）与代码 `文件:行号`（`center.rs:209-217`、`recursive_tower.rs:734-744`、`level_view.rs:341-395/:497-521/:535-626/:665-668`、`signal.rs:211-218/:285-398/:833-863/:1076-1089/:1231-1324`、`nest.rs:476-553`、`types.rs:216-221`、`p117_s1a_level_recheck.rs:502/:794`）。复刻件全部带生产实测自校验（§1 账本）；4 行 zerobit 复刻不符与 6 行 hit 复刻 N/A 已按仪器局限如实登记（非生产分叉）。
- 未对「6 命中点是否应算合法背书」拍板——那是裁定权；本文呈交实测与语义分析（§5.2），处置按 §9-③ 三案请裁。

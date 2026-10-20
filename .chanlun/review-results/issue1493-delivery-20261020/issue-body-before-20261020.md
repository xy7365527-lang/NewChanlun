Part of #1323；已批准图内总单：[完整生产程序 SPEC](https://github.com/xy7365527-lang/NewChanlun/issues/1340)。承接 #1490 裁定（教义正本 `.chanlun/definitions/xianduan.md` v2.3 §7.1），修 #1487 查出的线段吸收态。挡 Destination ④（完整历史回放）。

## 为什么

全量 BTC 1m 回放中，线段在 raw 787,209 之后再不产出（#1487）。原因是线段状态机进了一个出不来的状态：第一种情况下实现以「当下有无分型」决定包含（v1「先试不合并」），在 071:42 两句上都做反了（E1：c ⊂ b 时不合并、过早确认；E2：a、b 互含时合并、把极值吞进首元素）；段端门（顶>底）失败后只会 skip 续扫。#1419 L3 尾窗「5k–30k 停在 24 段」是同一机制。

## 要交付的外部行为

第一种情况的终结候选在破坏笔 b 处有三态：后续同向笔严格破 b 终点 ⟹ `CONFIRMED`；此前原方向严格越过假设转折点 ⟹ `KILLED_BY_NEW_EXTREME`（不复活，旧段延续）；否则 `OPEN_AT_PREFIX_END`，不发布。a、b 不做包含；c ⊂ b 时 c 记为 b 的窗成员，b 区间不动。第一种情况唯一的确认判据是 071:30/:32 的先破判据（相等不是破）。流首段起点按定义取段确认前成员笔中反向最极端点。段端门是不变式，失败 fail-closed 上报，不回退、不改起点重判。规则版本加 `segment-turn/1`，payload `s-segment/3`，`CONSTRUCTION_RULE` 不改。

## 代码义务（文件:行，基线 main `7c6ee750`；落地时以届时 main 重核）

| # | 位置 | 义务 |
|---|---|---|
| 1 | `rust/src/theta_v0/parser/feature_seq.rs:526-610` `append`（互含分支 `:552-596`） | 删除「先试不合并」（`:556-575` 的 provisional push／scan／pop）。互含分支改为三路：**R1** 入元素 ⊃ 末元素且在段方向更极端（Up：`high > last_h`；Down：`low < last_l`）⟹ push，不合并，记 `action="boundary_pair"`；**R2** 入元素 ⊂ 末元素且末元素是峰候选（比其前一标准元素更极端）⟹ 记为末元素的窗成员（`window_members.push(stroke_idx)`），末元素区间与 `stroke_idx` 不变，记 `action="window_member"`；**R3** 其余互含 ⟹ 现行 G-2 方向性合并（`:578-596` 不动）。并列极值（入元素 ⊃ 末元素且段方向极值相等、末元素是峰候选）：a、b 无缺口 ⟹ push 并标 `tie_with=last.stroke_idx`，记 `action="tie_pair"`（由先破判据确认）；a、b 有缺口（第二种情况）⟹ 同 R2 记为窗成员。 |
| 2 | `feature_seq.rs:646-745` `scan_trigger` | 去 `provisional` 分支；第一种情况（a、b 无缺口，含缺口被 c 封闭）的确认改为**先破判据**：向上段 `b.high > a.high ∧ c.high ≤ b.high ∧ c.low < b.low`，向下段镜像——这是第一种情况唯一的确认判据，不得写成「067 谓词 + 并列例外」；第二种情况与 `is_fractal_and_gap`（`:177-216`）严格谓词一字不改；每次扫描为每个峰候选 b 产出 `turn_hypothesis_state`：有分型 ⟹ `CONFIRMED`；存在后续元素使 b 不再是分型中元素（更极端的 R1 元素出现）⟹ `KILLED_BY_NEW_EXTREME`；否则且 b 有窗成员或尚无第三元素 ⟹ `OPEN_AT_PREFIX_END`。`turn_hypothesis_pair` = `[a.stroke_idx, b.stroke_idx]`。 |
| 3 | `feature_seq.rs:100-131` facts 结构 | `FeatureElement` 加 `window_members: Vec<usize>`；`FeatureStep.action` 域扩展；`FeatureCheck` 加 `turn_hypothesis_state: &'static str`。`:443-444` `frozen_first`（#1414）：窗 OPEN 期间不得冻结以 b 为中元素的 FIRST 三元素（第三元素尚不存在），确认时才冻结；写一条测试锁。 |
| 4 | `feature_seq.rs:22`、`:521-525`、`:646`；`segment.rs:805` | 文档改写：去「bit-exact Python」「移植参考的增量算法以通过认证」，写「071:30/:32/:42 直落（#1490）；Python v1 为历史参考」。 |
| 5 | `rust/src/theta_v0/parser/segment.rs:835-842` 与 `:1158-1165` | 两处同改（#804）：(a) 流首段（`segments` 为空）的起点按定义：每次终结尝试时 `seg_start = argmin/argmax` 段方向反向极值的同向笔起点（`[seed, end_stroke]` 内，并列取最早），构造门在该起点上判；pending 快照的首段起点同样按定义随前缀变化（不发布）；(b) 门失败（所有段）⟹ 仍 `skip_trigger`，候选 `result="FAILED_GEOMETRY"` 之外，`SegmentFacts` 置 `parse_invalid_from = Some(seg_start)` 并附最小复现（seg_start、end_stroke、`construction.vector`、分型三元素）；**不得**失败后改起点重判。增量侧同样字段进 `IncrSegments` 快照。 |
| 6 | `segment.rs:380-520` `record_candidate` | `result` 加 `PENDING_CASE_ONE`（活动检查的 b 有窗成员或 state=OPEN 且无分型）；`waiting_reasons` 加 `first_kind_third_stroke_within_first`；`SegmentCandidateFact` 加 `turn_hypothesis_state`、`window_members`、`parse_invalid_from`。 |
| 7 | `rust/src/theta_v0/parser/tb02c_tests.rs:55-75` | 改写 `tb02c_turn_boundary_is_not_merged` → `tb02c_turn_window_stays_open`：链 `[0,10,4,20,5,18,7]` ⟹ `standard.len()==2`、末 step `action=="window_member"`、`result=="PENDING_CASE_ONE"`、`turn_hypothesis_state=="OPEN_AT_PREFIX_END"`、`segments` 空。 |
| 8 | `tb02c_tests.rs` 新增 | (a) `mini_points.json` 三件：e1_min ⟹ 零段、pending 从 s0、Up；e1_release ⟹ 零段、Up 延续到 80；e2_min ⟹ `U s0–s2 10→50`、`D s3–s5 50→25`，pending s6 Up。(b) 真实截片 `e1_real_59845_40.json` ⟹ 零段、Up 延续；`e2_real_3175_40.json` ⟹ 6 段，首两段 `U s0–s2 3766.44→3807.53`、`D s3–s5 3807.53→3770`（夹具从 `/private/tmp/goal1323-native-20261012/tb1487-diag/runs/fixtures/` 复制入 `rust/tests/fixtures/tb1490/`，入仓前判名分：测试夹具）。(c) 伪造缺口见证：构造「窗成员折进 b 会改变下一候选缺口」的笔序列（原型随机组 48 型：b=[8190,8249]、成员 [8215,8221]、下一候选 [8163,8201]），断言 b 区间不变、下一候选 `gap==Some(false)`。(d) 并列极值见证：无缺口并列（先破判据确认）与有缺口并列（窗成员，走第二序列；原型组 48 向下段 s107 型）各一。(e) **先破判据 ≡ 067 严格谓词性质锁**：对端点价互异的随机折线（≥400 组×300 笔）两种判据的终结序列逐位相同；对含并列的输入记录差异集并断言差异全部落在「c 的 P 侧极值 = P」的候选上。(f) 流首段起点定义见证：首段起点 = 段确认前成员笔中反向最极端点（并列取先者），首段门失败为零，非首段不适用。(g) 全量≡增量：对 (a)(b)(c) 及随机折线 `divide_segments_with_facts` 与逐笔 `IncrSegments::append` 的 `segments`、`pending_start` 逐字段相等。(h) 生产流锁：六处 DIAG 现场的截片（#1487 DIAG §4）在新口径下 `FAILED_GEOMETRY` 计数为 0 且 `parse_invalid_from.is_none()`。 |
| 9 | `rust/src/bin/s_structure_session.rs:63`；`rust/src/bin/s_session_v2/tb02c.rs:34` | `RULE_REVISION` 追加 `+segment-turn/1`；`schema_revision` → `s-segment/3`，payload 加候选态与窗成员、`parse_invalid_from`。 |
| 10 | `s_session/s_tb02_contract.py:329/:414/:424/:498-499/:615/:644-646/:668`；`s_session/s_query_integrity.py:343-352`；`s_session/browser/tb01c-client.js:241/:552-554/:577` | 规则串与 schema 映射；FIRST 序列动作集 `("append","merge","boundary_pair","window_member"[,"tie_pair"])`，scope `TURN_HYPOTHESIS` 只给后三者，`merge_direction` 只在 `merge`；`turn_hypothesis_pair` 校验改为与 `turn_hypothesis_state` 配套；`result` 加 `PENDING_CASE_ONE`；`parse_invalid_from` 非空时 Q 层标阻断、浏览器显示阻断。 |
| 11 | `s_session/tests/fixtures/tb02c/`（`hand-oracle.json`、`raw-ledger.json`、`ORACLE.md`）、`tb02d/` | 审计是否含 c ⊂ b、并列极值、a ⊂ b 更极端的输入；命中则按 `segment-turn/1` 重算期望，旧对象 AsKnown 保留（与 #1344 第三问同口径）。 |
| 12 | `src/newchan/a_segment_v1.py` 头注 | 带票号登记「历史参考；#1490 后 `append`（:320-367）与 L78 后置（:457-490）非正本」。不改逻辑。 |


## 验收

- [ ] **六处现场**（#1487 诊断 DIAG §4）：全量 v1405／x1420 末段现场（笔 59848）、v1405 第一次长停（笔 3178）、x1420 第一次长停（raw 70,351）、尾窗 5k–36k（笔 370）、尾窗 38k–43k（笔 3040）、x1420 尾窗 7k–29k（笔 508，并列极值）——每处在新口径下要么按原文出段、要么旧段延续，均不得停：对应候选 `turn_hypothesis_state ∈ {CONFIRMED, KILLED_BY_NEW_EXTREME}`，`FAILED_GEOMETRY` 为 0。
- [ ] **手构与真实截片**：代码义务第 8 行 (a)(b) 的应得结果。
- [ ] **全量 `btc_1m_full.json`**（重型验证窗口，照实标注）：从 raw 1 起段层产出到数据末尾；每 25 万根分桶无连续零桶；**最长空窗上界**以本次实测定值并写进 §16 V-15（本票不预填数字）；`parse_invalid_from` 全程为 None；记录流首段按定义得到的起点与种子起点的位移。
- [ ] **全量／增量对拍锁（#804）**：全量结果与逐 bar `ParseLayerIncr` 的 segments／pending／候选态逐字段相等（含六处现场与尾窗阶梯 5k/10k/20k/30k/40k）。
- [ ] **前缀确认不回退**：对尾窗逐前缀，已发射段序列是后一前缀的前缀；允许的例外只有 #1344/#1420 登记的第二种候选复活，且须逐例标注。
- [ ] **Python 参考处置**：不作为任何对拍基线；`segment_refsem`／`_attrib_*` 若仍引 v1，一并标历史。
- [ ] **真实数据读数重出**：尾窗 49,995 根（起点 5）段数、#1419 L3 的 32/16/35 三读数按新规则版本重出，并把旧读数标 AsKnown。
- [ ] **S 会话入口**：通过正式会话入口运行含 e2_real 型输入的非真空数据，公共查询与浏览器能看到 `PENDING_CASE_ONE` → `CONFIRMED` 的候选态变化与新段对象；不得只直调内部函数。

## Blocked by

#1405（同改笔层输入与规则修订合成）、#1420（同改 `feature_seq.rs`／`segment.rs` 主循环；第二种候选的 078:52 失效是段端门第二类残余的前置）。次序：#1394 → #1405 → #1393 → #1419／#1420 → 本票。

## 不在本票

Lean 对齐（`xianduan.md` X-31：行号注、「第一种情况确认 = 先破判据」定义式与无并列等价引理、转折点窗三态定义式），另开 Lean 跟进票，不挡本票。真实下单、资金、账户或生产启用不在授权内。`make_segment`（#1419 X-21）不动。


# #1490 受影响代码清单（线段第一种情况候选三态、转折点窗、段端门去向）

> 票：[#1490](https://github.com/xy7365527-lang/NewChanlun/issues/1490)；教义正本：[`xianduan.md`](../../definitions/xianduan.md) §7.1、§15 X-25～X-32、§16 V-5／V-15／V-16。
> 基线：**main `7c6ee7502656f337101f2f1169ececbf45278d48`**（`segment.rs`／`feature_seq.rs`／`tb02c_tests.rs` 与 #1405 头 `5d99234d16` 逐字相同）。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归 [#1493](https://github.com/xy7365527-lang/NewChanlun/issues/1493)（TB-02-X），Lean 对齐另开跟进票；本清单本身不改任何代码。

| X | 侧 | 文件:行 | 处置 | 归属 |
|---|---|---|---|---|
| X-25 | Rust | `rust/src/theta_v0/parser/feature_seq.rs:526-610 (互含分支 552-596)` | append 互含分支改 R1(更极端含末元素⟹push)/R2(c⊂峰候选⟹窗成员,b区间不变)/R3(其余⟹G-2 方向性合并)；并列极值：a,b 无缺口⟹push 标 tie_pair，有缺口⟹窗成员 | #1493（TB-02-X） |
| X-25 | Rust | `rust/src/theta_v0/parser/feature_seq.rs:521-525, 646, 22` | 文档：去「bit-exact Python append/scan_trigger」「移植参考的增量算法」，改为 071:30/32/42 直落 + 票号 | #1493（TB-02-X） |
| X-26 | Rust | `rust/src/theta_v0/parser/feature_seq.rs:100-131` | FeatureElement/FeatureStep/FeatureCheck/FeatureSequenceFacts 加 window_members、turn_hypothesis_state∈{OPEN_AT_PREFIX_END,CONFIRMED,KILLED_BY_NEW_EXTREME}；step action 集 append/merge/boundary_pair/window_member(+tie_pair) | #1493（TB-02-X） |
| X-26 | Rust | `rust/src/theta_v0/parser/feature_seq.rs:646-745` | scan_trigger：第一种情况（无缺口／缺口被 c 封闭）确认改为先破判据（Up: b.high>a.high ∧ c.high≤b.high ∧ c.low<b.low；Down 镜像），唯一判据；候选三态记录；turn_hypothesis_pair=(a,b)；provisional 机制退役 | #1493（TB-02-X） |
| X-26 | Rust | `rust/src/theta_v0/parser/feature_seq.rs:177-216 (188, 203)` | is_fractal_and_gap 严格谓词不改：只服务第二种情况与「先破判据 ≡ 067 严格谓词（无并列）」性质锁 | #1493（TB-02-X） |
| X-26 | Rust | `rust/src/theta_v0/parser/feature_seq.rs:515-519` | skip_trigger 保留；frozen_first(#1414) 与窗成员的交互须核（窗 OPEN 期间不冻结 FIRST 三元素） | #1493（TB-02-X） |
| X-27 | Rust | `rust/src/theta_v0/parser/segment.rs:835-842` | divide_segments_core：流首段起点按定义（每次终结尝试取 [seed,end] 内反向极值同向笔起点，并列最早）；门失败：fail-closed 诊断态（parse_invalid_from，附最小复现），不得失败后改起点重判 | #1493（TB-02-X） |
| X-27 | Rust | `rust/src/theta_v0/parser/segment.rs:1158-1165` | IncrSegments::append 同一门同改（#804 两处同改） | #1493（TB-02-X） |
| X-27 | Rust | `rust/src/theta_v0/parser/segment.rs:380-520` | record_candidate：result 加 PENDING_CASE_ONE；waiting_reasons 加 first_kind_third_stroke_within_first；诊断态字段 | #1493（TB-02-X） |
| X-27 | Rust | `rust/src/theta_v0/parser/segment.rs:147-157` | find_overlap_start 不改：只定扫描起点；流首段的起点由 §7.1 第 5 条定义给出 | #1493（TB-02-X） |
| X-27 | Rust | `rust/src/theta_v0/parser/segment.rs:805` | 注释去「bit-exact Python segments_from_strokes_v1」 | #1493（TB-02-X） |
| — | Rust | `rust/src/theta_v0/parser/segment.rs:159` | CONSTRUCTION_RULE 不改（构造条件未变），登记 | #1493（TB-02-X） |
| — | Rust | `rust/src/theta_v0/parser/segment.rs:180-211 (205-208)` | construction 顶>底门不改（不变式保留） | #1493（TB-02-X） |
| — | Rust | `rust/src/theta_v0/parser/segment.rs:1040-1049` | confirmed_bound 回退面不改（第一种窗不扩大回退面，原型前缀回退 0） | #1493（TB-02-X） |
| X-28 | Rust | `rust/src/theta_v0/parser/tb02c_tests.rs:55-75` | tb02c_turn_boundary_is_not_merged 改写：链 [0,10,4,20,5,18,7] 应得 standard.len()==2、result=PENDING_CASE_ONE、turn_hypothesis_state=OPEN_AT_PREFIX_END、无 CASE_ONE | #1493（TB-02-X） |
| X-28 | Rust | `rust/src/theta_v0/parser/tb02c_tests.rs:新增` | 锁：e1_min/e1_release/e2_min、e1_real/e2_real 夹具；伪造缺口见证；并列极值（无缺口/有缺口各一）；先破判据≡067 严格谓词性质锁（端点价互异输入）；流首段起点定义；全量≡增量；FAILED_GEOMETRY 生产流计数零 | #1493（TB-02-X） |
| X-29 | Rust | `rust/src/bin/s_structure_session.rs:63` | RULE_REVISION 加 +segment-turn/1 | #1493（TB-02-X） |
| X-29 | Rust | `rust/src/bin/s_session_v2/tb02c.rs:34` | schema_revision s-segment/2 → s-segment/3；payload 加候选态与窗成员 | #1493（TB-02-X） |
| X-29 | Python/JS | `s_session/s_tb02_contract.py:329, 414, 424, 498-499` | rule_revision→schema 映射加 segment-turn/1 → s-segment/3；expected_version 元组更新 | #1493（TB-02-X） |
| X-29 | Python/JS | `s_session/s_tb02_contract.py:615` | FIRST 序列 step.action ∈ (append, merge, boundary_pair, window_member, tie_pair)；scope TURN_HYPOTHESIS 只给后三者；merge_direction 只在 merge 上 | #1493（TB-02-X） |
| X-29 | Python/JS | `s_session/s_tb02_contract.py:644-646` | turn_hypothesis_pair = 候选 (a,b) 笔序号（不再是 elements[:2] 的机械映射）；新增 turn_hypothesis_state、window_members 校验 | #1493（TB-02-X） |
| X-29 | Python/JS | `s_session/s_tb02_contract.py:668` | result 集合加 PENDING_CASE_ONE；诊断态字段 | #1493（TB-02-X） |
| X-29 | Python/JS | `s_session/s_query_integrity.py:343-352` | _published_axes 加新规则串 | #1493（TB-02-X） |
| X-29 | Python/JS | `s_session/browser/tb01c-client.js:241, 552-554, 577` | 段规则/版本与规则串映射同步 s-segment/3 + segment-turn/1 | #1493（TB-02-X） |
| X-32 | Python/JS | `s_session/tests/fixtures/tb02c/ (hand-oracle.json, raw-ledger.json, ORACLE.md), s_session/tests/fixtures/tb02d/:—` | 审计含 c ⊂ b 或并列极值输入的期望，按 segment-turn/1 重算；旧对象 AsKnown | #1493（TB-02-X） |
| X-30 | Python/JS | `src/newchan/a_segment_v1.py:320-367, 457-490` | 不改写；头注带票号：历史参考，#1490 后非正本；「编排者裁定 v1 唯一口径」指范式 P2，不指此处实现 | #1493（TB-02-X） |
| X-31 | Lean | `formal/Origin/SegmentFeatureSeq.lean:265` | 注引「071:40」实为 071:42，行号漂移登记（内容不改） | Lean 跟进票（不挡 #1493） |
| X-31 | Lean | `formal/Origin/SegmentFeatureSeq.lean:270-271` | BoundaryNoInclusion 与 Q2-a 同义，不改 | Lean 跟进票（不挡 #1493） |
| X-31 | Lean | `formal/Origin/SegmentFeatureSeq.lean:290-298` | IsTopFractal/IsBottomFractal 严格版不改；登记补「第一种情况确认 = 071:30/:32 先破判据」定义式与无并列输入上的等价引理（跟进票，不挡叶片） | Lean 跟进票（不挡 #1493） |
| X-31 | Lean | `formal/Origin/SegmentFeatureSeq.lean:新增` | 转折点窗三态（071:32 待定态与两种结局）的定义式——Lean 无定义，登记义务 | Lean 跟进票（不挡 #1493） |
| X-31 | Lean | `formal/Phase2/Claim10_SegmentV1.lean:263, 269-271` | 注引「第71课:122」为另一语料行号；inclusionAllowedAtBoundary firstKind=false 与 Q2-a 一致，不改 | Lean 跟进票（不挡 #1493） |

登记不改：

- rust/src/theta_v0/parser/segment.rs:687-726 make_segment（#1419 X-21，不改）
- rust/src/theta_v0/parser/feature_seq.rs:299-398 second_sequence（#1420 范围；TB-02-X 须在其后或同树验收）
- rust/src/theta_v0/classifier/*（中枢层只消费段对象；读数重出归 #1419 回填）

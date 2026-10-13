# #1344 受影响代码清单（第二种候选的语义终点与扫描窗口名分）

> 票：[#1344](https://github.com/xy7365527-lang/NewChanlun/issues/1344)；教义正本：[`xianduan.md`](../../definitions/xianduan.md)（线段；落点 §5.1、§7 第 78 课、§15 X-12／X-16～X-20）。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**。行号于该基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码（本次提交仅有的一处代码改动是 #1344 的一行零行为注释，见 #1344 清单）。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/theta_v0/parser/feature_seq.rs:298` | main `465174240b` | `second_sequence` 文档：本次提交在 `:298` 之后加一行带票号的零行为注释（本片未实现 `078:52` 失效条件，归 #1420） | 本票落文档（已落） |
| Rust | `rust/src/theta_v0/parser/feature_seq.rs:143-153` | main `465174240b` | `SecondSequenceFacts` 加语义状态与越界水位字段 | TB-02-W #1420 |
| Rust | `rust/src/theta_v0/parser/feature_seq.rs:299-396` | main `465174240b` | `second_sequence` 加「分型检查后、越界检查」分支与提前返回 | #1420 |
| Rust | `rust/src/theta_v0/parser/feature_seq.rs:395` | main `465174240b` | `resource_truncated` 语义保留；失效时为 false | #1420 |
| Rust | `rust/src/theta_v0/parser/segment.rs:264/:444/:474/:491` | main `465174240b` | `second` 字段、`PENDING_CASE_TWO`、`matched`、`resource_truncated` 的读取处识别失效态 | #1420 |
| Rust | `rust/src/theta_v0/parser/segment.rs:924-936`、`:1036-1040` | main `465174240b` | `earliest_unsealed_from` 字段与 `append` 的 `confirmed_bound` 回退：失效候选不复活；`:928-930` 级联复活注释改引 #1344 | #1420 |
| Rust | `rust/src/theta_v0/parser/segment.rs:961-966` | main `465174240b` | `from_full` 不重建跳过史的限制，改为按 K1 游标恢复 | #1420 |
| Rust | `rust/src/theta_v0/parser/feature_seq.rs:278-311/:596-613/:834-879` | main `465174240b` | 旧扫描实现与「提前停 ≡ 全扫」性质测试换含失效的 oracle | #1420 |
| Rust | `rust/src/theta_v0/parser/second_kind.rs:37-41/:109-111` | main `465174240b` | 补失效语义说明 | #1420 |
| Rust | `rust/src/theta_v0/config.rs:54-57/:65` | main `465174240b` | `second_seq_scan_window` 名分 ＝ 调度预算（不是语义），文档注明 | #1420 |
| Rust | `rust/src/bin/s_session_v2/tb02c.rs:146/:164/:297-299` | main `465174240b` | scan 记录加语义状态；`deferred_scope` 是否增 `second_kind_kill_by_new_extreme` 随 #1420 定。#1414 侧不改输出（根裁定第三问） | #1420 |
| Rust | `rust/src/segment.rs:410-413/:416-448/:592-597` | main `465174240b` | 族I `MAX_SECOND_SEQ_SCAN = 50`：不改造、不参准入；带票号注释；登记 TB-10 旧入口 | TB-10 |
| Rust | `rust/src/orchestrator.rs:53/:104-112/:205-219` | main `465174240b` | 同上；`trigger_k + 1 + 50 ≤ n` 余量判据只在截断口径下成立 | TB-10 |
| Rust | `rust/src/segment_layers.rs:499/:530-545/:828-838` | main `465174240b` | 同上 | TB-10 |
| Rust | `rust/src/lib.rs:946-993` | main `465174240b` | PyO3 `RecursiveOrchestrator` 消费面点名登记 | TB-10 |
| Lean | `formal/Phase2/Claim10_SegmentV1.lean:46-47/:184-185/:198` | main `465174240b` | 补「候选失效」形式定义，或明写有效域 | #1420 或 Lean 跟进票 |
| Lean | `formal/Origin/SegmentFeatureComplete.lean` | main `465174240b` | 未逐行读；需核是否承载第二序列 | #1420 |
| Python | `src/newchan/orchestrator/recursive.py:1` | main `465174240b` | 「口径 A 正式路径」自称与 #1323 owner 不符；登记 TB-10 | TB-10 |
| Python | `src/newchan/a_segment_v1.py:369/:372-399` | main `465174240b` | 参考实现的 50 窗：名分对照，不进准入 | — |
| Fixture | `s_session/tests/fixtures/tb02d/` | main `465174240b` | 审计输入与 oracle 是否含「越界先于分型」的输入；命中则以新 `rule_revision` 重算，旧对象保留为 AsKnown 历史 | #1420 |
| Doc | `.chanlun/definitions/xianduan.md` | main `465174240b` | 三行头、§5.1、§7 第 78 课、§15 X-12／X-16～X-20（本次提交已落） | 本票落文档 |
| Doc | `CONTEXT.md` | main `465174240b` | 词条「第二种候选的语义终点」（本次提交已落） | 本票落文档 |

## 本轮复核订正（相对工位草稿）

- 工位草稿以 TB02D `03239158`（PR #1477 head）为部分行的基线；该 head 已合入 main `465174240b`，`git diff 03239158 465174240b` 在 `rust/src/theta_v0/parser`、`rust/src/bin/s_session_v2`、`s_session/tests/fixtures/tb02d` 上为空，本清单统一标 main。
- `s_session/tests/fixtures/tb02d/` 是目录（main 上有 `MANIFEST.json`、`generate-input.py`、`hand-oracle.json`、`raw-ledger.json` 四个文件），审计对象是其中的输入与 oracle；工位行号核对脚本按文件处理，把它误报为缺失。
- 根裁定第三问收窄了 #1414 侧的动作：只在 `feature_seq.rs` `second_sequence` 文档加一行带票号注释。工位草稿拟随 #1414 关票改的 `tb02c.rs` `deferred_scope`，以及 `segment.rs` 的注释，改随 #1420／TB-10 落。

基线说明：工位草稿以 main `f95a7f3e` 为基线（#1344 部分行以 TB02D `03239158` 为基线）；本清单不靠差异推断，全部行号直接在 main `465174240b` 上用 `git show` 逐行核过。

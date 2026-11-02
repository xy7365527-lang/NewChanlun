# #1488 S2：MIN12 oracle 重建报告（D2）

- 分支 `codex/1488-accept-s2-20261020`，起点 main `6badaa6049`；本目录为 MIN12 oracle 的**单一来源**（D2：#1488 与 #1419 共用，各票引用本目录，不另持副本）。
- 状态：oracle 已生成、双跑逐位一致；非作者独审 0055 结论 APPROVED_WITH_FOLLOWUPS（oracle 本体可作 D2 单一来源，无 P1）。本版按 0056 订正 F1–F10，oracle 文件与清单未动、未重跑（订正记录见 `workers/s2-fix-0056/REPORT.md`）。

## 0. 结论速览

| 项 | 结论 |
|---|---|
| 输入固定 | 52 行 `input/messages.jsonl` sha256 `d61371c9…c6cd`，`input/clock.json` `779dcb85…fb36f`，与归档原件一致 |
| 二进制 | main `6badaa6049`，`--features s_session`，WSL 构建，sha256 `c944bd6bed40dd7e75332d66851ad050178b7ccb31860c1068d749de2726e040`（放 G:，不入仓）。按 SHA 冷重建可逐位复现（§6 第 7 条） |
| rule_revision | `s2-axis-quantifiers+new-bi/2+segment-second/2` |
| 摄入 | 两跑各 52/52 `Committed`，head g=52 |
| oracle | g=1..52 逐代 AsKnown 全量公共切面；每代服务端 `cut_projection_digest` = 本地复算 = 落盘文件 sha256（104/104 一致） |
| 双跑 | run-a / run-b 新进程新库：52 个 oracle 文件逐字节相同，`STRUCTURAL.jsonl` 相同；最终库**主库文件 + 逻辑内容**相同（主库 `4f185399…dedc`；WAL checkpoint 后 `f83b5add…d107`、iterdump 逻辑摘要 `b585f5ea…5b79`，§2.3） |
| 与旧 known-15/31 | 笔/段/种子/特征序列等全部 fact_key 与数值读数**一致**；差异只有三类：①new-bi/2 的版本标签与字段改形；②segment-second/2 的版本标签与字段改形；③旧二进制的 center-seed/1 对象（CC-056.center_version、CC-022.unit_assignment）、其 replaces 关系、以及 fixed_cut/catalog 中的 center-seed 轴在 main 上不存在。三类均预期，逐项见 §3（semdiff v2 全量复核）。**意外差异 0 项** |
| FORMED seed / EXTENDING 中枢 | **main 不产出**：main `6badaa6049` 没有 center-seed/TB-02-E 代码，g1..52 公共切面中心族对象数为 0。**主控已裁定（A③，2026-10-20 05:18Z）**：A③ 改为「MIN12 52 根全程在 10s 帽内、读数与 new-bi/2 oracle 逐位一致」，中枢出现条件转 #1419 验收（§4；裁定所称“(a)”= 本报告 §4 的选项②，**不是**“先合 #1419”） |
| R3 第 46 条台阶 | 旧台阶 op-0045→op-0046 5420→9471 ms，期间写入量没有跳变，Q 也空闲。main 本次两跑**在同一位置未观测到台阶**（run-a 6458→6919 ms），但宿主噪声下**不可判**：run-a 在 op-0041→op-0042 自己就有 5267→9322 ms 的同量级跳变，run-b 有 11 条 >10 s。不能据此说“main 不复现”。10 s 帽证据须由 S6 在安静宿主上取得（§5） |

## 1. 输入与旧读数（入仓）

| 文件 | sha256 | 说明 |
|---|---|---|
| `input/messages.jsonl` | `d61371c95036eaaaacc94c9780955555271be41a49ad4d5f242424a1c447c6cd` | 52 行冻结流（原请求帧，逐条原样发送） |
| `input/clock.json` | `779dcb8539bc9278933bde14d852e9d34493d956cb2c2647cb1db3ead9dfb36f` | 冻结时钟计划 |
| `input/archive-config.json` | `21ca402d…` | 旧探针 launcher 配置原件（只用来复用身份字段） |
| `archive-probe/S0B-PROBE.json` | `95e7cedc…53b7` | 旧探针读数（48e2386463 / 二进制 38c61eee） |
| `archive-probe/events.jsonl` | `cc218bd6…c967` | 旧运行事件日志 |
| `archive-probe/PROBE-RESULT.json`、`S0B-PLAN.json` | 见文件 | 旧结果为 INCOMPLETE：sent 48，停在 op-0047，原因是 DeliveryUnknown |

归档原件在 `G:/AgentStorage/mac-archive-20261019/issue1419-s0b-min12-20261014/`；旧库 `MIN12-per-bar/session.sqlite`（188 MB）留在 G:，没有拷回。设计文件 `s0b-MIN12.json` 已丢失（MAP 已记），旧侧可对照的读数只有 known-15/known-31 两个候选和 PROBE 片段。

## 2. oracle 生成

### 2.1 方法
- 驱动：`tools/min12_oracle_run.py`，WSL 入口为 `tools/run_wsl.sh <label> <port>`。
- 正式入口：`s_session/launch_s.sh service start`（s-launcher/2，S 常驻写者 + Q 只读）。每跑使用新工作目录、新库、新进程（`~/nc-remote/accept-s2/run-<label>`，ext4），跑完整目录拷到 `G:/AgentStorage/1488-accept/s2/run-<label>/` 并写 `SHA256SUMS`。
- 逐事件摄入：`s_socket_client.exchange` 一条一条发，收到完整回执并通过 `validate_reply` 后再发下一条。
- 公共查询：摄入完成后，对 g=1..52 逐代 `POST /api/v2/snapshot`（AsKnown、`as_of_generation=g`、CompleteCut、`page_size=31`），用 `snapshot_token` 翻完所有页，分页间切面摘要必须不变。拼成 projection（fixed_cut/catalog/counts/scope/omissions/order_version/rows），用 canonical 方式落盘，本地 sha256 必须等于服务端 `cut_projection_digest`。

### 2.2 产物
| 位置 | 内容 |
|---|---|
| `oracle/ORACLE.json` | **引用入口**：二进制、输入、查询口径，以及每代 `cut_projection_digest`、两跑文件 sha、bytes、counts |
| `oracle/READINGS.jsonl` | 每代结构读数摘要：笔（起止 raw 锚、价、state）、段（方向、起点、几何终点、result、形成代/终止代）、种子、segment_version 等待理由、中心族清单 |
| `runs/run-{a,b}/` | RUN.json、INGEST.jsonl（逐条耗时与回执）、ORACLE-INDEX.jsonl、SHA256SUMS、config.json |
| G: `run-{a,b}/oracle/g-0001..0052.json` | 全量切面（g52 为 31.5 MB；全部入仓体积太大，所以只在仓里放 sha） |
| G: `run-a/STRUCTURAL.jsonl` | sha256 `eb75995a4ee9745674ef7216c1a83cdaa2e0b41b35ab968d5a432939f2997761`（17.6 MB，两跑相同） |

关键代摘要（两跑相同）：

| g | total | objects | withdrawn | relations | witnesses | observations | raw_history | digest（前 8 位） |
|---|---|---|---|---|---|---|---|---|
| 16 | 681 | 94 | 165 | 289 | 81 | 36 | 16 | `74d8df32` |
| 32 | 1696 | 198 | 475 | 742 | 177 | 72 | 32 | `383df8bc` |
| 47 | 2734 | 296 | 808 | 1210 | 267 | 106 | 47 | 见 ORACLE.json |
| 52 | 3162 | 332 | 959 | 1405 | 297 | 117 | 52 | `c15e3fb5` |

### 2.3 双跑一致性
- 52 个 oracle 文件逐个 sha256 相同；ORACLE-INDEX 中 digest、bytes、pages、counts、kind_counts、rule_revision、structure_cut 逐代相同（只有 `query_ms` 不同）。
- `STRUCTURAL.jsonl`、`READINGS.jsonl` 相同；最终 `session.sqlite` 的**主库文件 + 逻辑内容**两跑相同（0055/F9 订正）：
  - 只比主库文件不够：两跑都留有未 checkpoint 的 WAL（各 6381912 字节，盐不同，字节必然不同），主库 sha `4f185399…dedc` 相同只说明 checkpoint 前的页相同。
  - 用 `tools/db_logical_digest.py` 在 G: 原件的副本上复核（WSL，结果 `G:/AgentStorage/1488-accept/s2-fix-0056/db-digest-run-{a,b}.json`）：两跑 `wal_checkpoint(TRUNCATE)` 后主库 sha 均为 `f83b5add5ead…d107`（与 0055 独立复核值一致），iterdump 逻辑摘要均为 `b585f5ea100d…5b79`，各表行数相同（relations 1405、objects 99、structure_facts 1192 等）。0055 的 dump 摘要 `6f9126bc…` 与此不同，原因是两边序列化方式不同（本工具逐行加 `\n`）；同一工具下两跑一致即为判据。
  - 疑点如实记录：`PRAGMA wal_checkpoint` 返回 `log_frames=0`，但 checkpoint 后主库 sha 已不同于原主库，且与审阅者值一致，说明 WAL 内容确已并入；该返回值读数原因未追查，不影响结论。
  - 驱动已改为服务停止后对原库做 checkpoint 并在 RUN.json 记 `db_digest`（今后的跑生效，本次未重跑）。
- INGEST 中语义字段（kind/ok/accepted_seq/publication/receipt_id）0 差异，只有时间戳不同。

### 2.4 结构读数（main，new-bi/2）
- 笔：g52 有 12 笔，端点价依次为 12000→32000→22000→52000→27000→47000→17000→37000→22000→42000→30000→40000→24000，前 11 笔 `CONFIRMED`，末笔 45→49（40000→24000）为 `FORMED_UNCONFIRMED`。
- 段：`segment:1:1` UP 12000→52000（g15 形成，g27 终止，CASE_ONE）；`segment:1:13` DOWN 52000→17000（g27 / g39）；`segment:1:25` UP 17000→42000（g39 / **g51**）；`segment:1:37` DOWN 从 42000 起（g51 形成，NO_FIRST）。
- 段端点序列 12000, 52000, 17000, 42000，加上末笔端点 24000，与 PROBE 记录的 `endpoints_designed [12000,52000,17000,42000,24000]` 一致。旧 known-31 时点的 got `[12000,52000]` 与 main g32 也一致。
- g51 是第三段终止、第四段起始的代，第 3 个完整段在这一代出现。这正是旧 center-seed/1 起种所需的前提：旧 known-31 的 `CC-056.center_version`（objects 族）`payload.data.scan_cursor` 写着 `seed_start_unit_id segment:1:1, awaiting_offset 2`（0055/F5 订正：初版误记为 `CC-022.unit_assignment`）。所以 A③ 的「第 51 条起」在段层面的触发条件 main 已具备，**中心对象本身没有**。
- g52 的 `CC-056.segment_version`：delivered `seed_strict_first_and_second_terminal`，deferred `[general_equal_identity, global_G006_bridge]`，waiting `fewer_than_three_standard_elements`。

### 2.5 回复耗时（只做记录，10 s 帽的判定留给 S6/A③）
- run-a：最大 9322 ms（op-0042，前一条 op-0041 为 5267 ms，跳变 +4.05 s，与旧第 46 条台阶同量级），52 条全部 ≤10 s，合计 160 s。op-0045→op-0046→op-0047：6458→6919→7478。
- run-b：最大 14282 ms（op-0051），**11 条 >10 s**（op-0041 起），合计 220 s。
- 旧 Mac（48e2386463）：op-0045 5420，op-0046 9471，op-0047 10001（DeliveryUnknown）。
- **口径偏差**：WSL 宿主是共享的（8 核，采样时 load 5–8，同时有其他工位的 `cargo test` 占 300%+ CPU，S1 的 s_structure_session 也在跑），两跑耗时相差 1.4 倍，所以本切片的耗时**不能用作帽内证据**。为了让 oracle 不因帽中断，S 的 `response_timeout_ms` 和客户端期限放宽到 120000 ms，Q 资源也放宽了（`tools/query-resources-oracle.json`：verify/capture deadline 60000、bytes 1 GiB）。这些是 oracle 生成口径，不是验收口径。

## 3. 与旧 known-15 / known-31 逐项对照

方法（0055/F1 订正后为 v2）：`tools/semdiff.py`（schema `semdiff/2`）。只把身份哈希（obj/batch/wit/obs/rcpt）解析为 `<kind|fact_key|r修订|g首知代|lifecycle>`，**不剥任何版本字段，不改写 `raw_position`，保留空容器叶子**；行按 family:kind 做规范化多重集，残差按身份键逐对配对（同键多条不折叠），再去 fact_key 做第二轮配对（覆盖槽位改名的同一对象）；fixed_cut / catalog / counts / scope / omissions / order_version 逐路径比对。结果：`compare/semdiff2-known15.json`（sha `2ef5d95f858f…`）、`compare/semdiff2-known31.json`（`2ec7f79fd74e…`）；旧 known-N 对应 main g=N+1。初版 v1 输出 `compare/semdiff-known{15,31}.json` 保留作对照，**已被 v2 取代**：v1 递归剥除 semantic/policy/schema/domain、空容器不出叶子、按 fact_key 用 dict 配对（重复折叠）、不比 fixed_cut/catalog，正好漏掉了 0055 F1 列出的那些项。补证脚本在 `compare/fix-0056/`（`check_toplevel.py` 对 catalog/fixed_cut 列表做多重集复核；`check_raw.py` 核 raw 坐标）。

### 3.1 行级：配对后的字段差（逐 kind）

计数写作 objects/withdrawn（known-15→g16；known-31→g32）。两个时点的**取值集合完全相同**，只有条数不同。

| # | kind | 字段差 | k15 | k31 | 归因 |
|---|---|---|---|---|---|
| R1 | CC-008.endpoint | 版本三件套 N①；`raw_position`→`raw_first/raw_last` | 4/4 | 8/8 | new-bi/2 |
| R2 | CC-008.new_bi_pair | N①；old/new 的 `raw_position`→`raw_first/raw_last`；新增 `evidence: []` | 3/3 | 7/7 | new-bi/2 |
| R3 | CC-010.bi | N①；start/end 的 `raw_position`→`raw_first/raw_last` | 3/5 | 7/13 | new-bi/2 |
| R4 | CC-055.change_event | N①；`causes[]` `new-bi-dual-coordinate/1→/2`；before/after 的 start/end `raw_position`→`raw_first/raw_last` | 2/6 | 2/18 | new-bi/2 |
| R5 | CC-055.relation | N① | 23/35 | 55/91 | new-bi/2 |
| R6 | CC-056.version | N① | 1/15 | 1/31 | new-bi/2 |
| R7 | CC-054.knowledge_state | `domain`：`established_direction_without_extreme_identity_competition`→`established_direction_with_extreme_root_sets` | 1/15 | 1/31 | new-bi/2 测域扩展 |
| R8 | CC-011.seed_candidate | 版本两件套 S① | 0/5 | 0/5 | segment-second/2 |
| R9 | CC-011.segment_seed | S① | 1/1 | 2/2 | segment-second/2 |
| R10 | CC-012.feature_sequence | S① | 1/1 | 2/10 | segment-second/2 |
| R11 | CC-013.segment | S① | 1/1 | 2/10 | segment-second/2 |
| R12 | CC-055.segment_change | S① | 1/1 | 1/11 | segment-second/2 |
| R13 | CC-055.segment_relation | S① | 2/2 | 7/27 | segment-second/2 |
| R14 | CC-056.segment_version | S①；`fact_key`/`slot` `TB-02-D→TB-02-W`；`delivered_scope` `seed_strict_first_and_finite_second→seed_strict_first_and_second_terminal`；`deferred_scope` 去掉 `general_scan_continuation`；新增 `scan_budget {policy: second-seq-scan-budget/1, round_strokes: 0}`；**新增 `scan_cursors: []`**（0055/F6）；`object_id`/`superseded_by` 只随槽位改名而变 | 1/15 | 1/31 | segment-second/2 |
| R15 | relations `replaces`（配对部分） | subject/object 只随 R14 槽位改名而变（全量核 15/31 条，0 例外） | 15 | 31 | segment-second/2 |
| R16 | CC-056.center_version（TB-02-E） | main 无此对象（未配对） | −1/−15 | −1/−31 | main 无 center-seed/1 |
| R17 | CC-022.unit_assignment | main 无此对象（未配对） | −1/−1 | −2/−10 | main 无 center-seed/1 |
| R18 | relations `replaces`（未配对） | 两端全是 R16/R17 的中心对象 | −16 | −41 | main 无 center-seed/1 |
| R19 | witnesses / observations / raw_history | 无差异 | 0 | 0 | — |

- N①（new-bi/2 版本三件套）：`schema_revision s-new-bi/1→/2`、`semantic_version new-bi-dual-coordinate/1→/2`、`policy_version standard_raw_gap_3` → `standard_raw_gap_3;same-price/keep-earlier+extreme-region/1`。
- S①（segment-second/2 版本两件套）：`schema_revision s-segment/2→/3`、`semantic_version segment-second/1→/2`（这些 kind 不带 policy_version）。
- raw 坐标：旧 `raw_position` 与新 `raw_first`/`raw_last` 多重集相等（k15 62/62、k31 150/150），且新侧全部 `raw_first == raw_last`（`compare/fix-0056/raw-coords.json`）。本流数值不变。
- 未配对项只有 R16–R18；其余所有差异行都已逐对配上。表外 kind（笔包含、分型、关系等）规范化后逐字段相同。
- 行数：g16 715→681（identical 513；−34 = 1+15+1+1+16），g32 1781→1696（identical 1274；−85 = 1+31+2+10+41），全部由 R16–R18 解释。

### 3.2 顶层：fixed_cut / catalog / counts（逐项）

fixed_cut 与 catalog 里的同名字段差异相同，下表合并列出；k15、k31 两处相同的只写一次。

| # | 位置 | 差异 | 归因 |
|---|---|---|---|
| T1 | `rule_revision` | `s2-axis-quantifiers+new-bi/1+segment-second/1+center-seed/1` → `s2-axis-quantifiers+new-bi/2+segment-second/2` | new-bi/2、segment-second/2、去 center-seed/1 |
| T2 | catalog_evidence 轴 CC-016 / CC-017 / CC-022（TB-02-E 中心轴：deferred/delivered_subobligations、waiting_reasons `UNIT_NOT_COMPLETED`〔k31 另有 `NO_LEFT_CENTER_IN_WINDOW`〕、impl `not_implemented`、proof `not_proved`、run waiting/waiting/run） | main 上整段缺失 | main 无 center-seed/1 |
| T3 | CC-056 轴 evidence | object_ids −1；replaces k15 34→32、k31 38→36；withdrawals k15 36→34、k31 40→38；多重集复核，减少的全是中心相关项 | main 无 center-seed/1 |
| T4 | `classified_objects` / replaces / withdrawals 总数 | k15 96→94、34→32、36→34；k31 201→198、38→36、40→38 | 同 R16–R18 |
| T5 | `catalog.items` | CC-016、CC-017 `waiting→not_run`，CC-022 `run→not_run`，三者 evidence 变为 `{}` | main 无 center-seed/1 |
| T6 | catalog `counts` | run 17→16、not_run 97→100 | 随 T5 |
| T7 | 顶层 `counts`（行） | k15 objects 96→94、relations 305→289、withdrawn 181→165、total 715→681；k31 201→198、783→742、516→475、1781→1696 | 同 R16–R18 |
| T8 | 其余 catalog 项 | 只有 object-id 重新哈希，解析为名字后相同 | 身份哈希随二进制变化 |

复核产物：`compare/fix-0056/toplevel-known{15,31}.json`。

### 3.3 结论
- 所有差异都落在三类之内：new-bi/2（R1–R7、T1）、segment-second/2（R8–R15、T1）、main 无 center-seed/1（R16–R18、T1–T7）。T8 是身份哈希，不算语义差异。
- **意外差异：0 项。** MIN12 流里没有同价极值竞争，所以 new-bi/2 的同价规则在本流上不改变任何读数，只改变标签和字段形态。旧 known-31 的 counts（objects 201 / relations 783 / total 1781）在 main 上**不应再作为期望值**，应以本 oracle 为准。

## 4. 缺口：FORMED seed / EXTENDING 中枢（A③）—— 已裁定

- A③ 原文要求「第 51 条起出现 FORMED seed 与 EXTENDING 中枢，读数与 MIN12 oracle 一致」。
- 事实：main `6badaa6049` 的 `rust/src/bin` 中没有 `center-seed`、`CC-022`、`CC-056.center` 的实现。`tb02e.rs`、`center_scan.rs` 只存在于 `48e2386463`（不是 main 的祖先）和 `codex/1419-center-seed-20261013`、`codex/1424-center-pair-20261014`、`codex/1426-prep-*`、`codex/1429-prep-*` 等分支。
- 所以本 oracle 的中心族为空，这是 **main 的真实读数**，不是漏抽。
- 本报告初版（b6a5a3e46c，先于裁定入仓）提出的三个选项，为避免与裁定原文的 “(a)/(b)” 相撞，**改用 ①②③ 编号**：
  ① 先合入 #1419 center-seed，再在合后 main 上重跑本 oracle；
  ② 把 A③ 的中心半句移交 #1419，#1488 只验笔/段与帽；
  ③ 用 #1419 分支二进制另出一份中心 oracle（不推荐，破坏 D2「main 全量二进制、单一来源」）。
- **主控裁定**（#1488 评论，2026-10-20T05:18:17Z，https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6283982007）原文：
  > 主控裁定（A③）：取 (a)。(b) 不可行：#1488 阻塞 #1419（边 #1488→#1419），等其中枢种子合入会成环。A③ 改为「MIN12 52 根全程在 10s 帽内、读数与 new-bi/2 oracle 逐位一致」，原字面保留并注明改写原因；「第 51 条起出现 FORMED seed 与 EXTENDING 中枢」转为 #1419 的验收项（在增量路径上用同一 MIN12 oracle 验）。
- **映射**：裁定中的 “(a)” = 本报告选项 **②**（中心半句移交 #1419）；裁定中被否决的 “(b)”（等 #1419 合入）= 本报告选项 **①**。下游请勿按初版编号把裁定读成“先合 #1419”——那会与 #1488→#1419 的阻塞边成环。
- 因此，本 oracle 对 #1488 的用途是“读数与 new-bi/2 oracle 逐位一致”；“10 s 帽内”由 S6 取证（§5）；中心读数由 #1419 在增量路径上用同一 MIN12 输入验收。
- 从段层面可以确认：第 3 个完整段在 g51 出现，三段为 12000–52000、52000–17000、17000–42000。center-seed 所需的输入条件在第 51 条确实成立，与 A③ 原字面的时点描述吻合，可供 #1419 引用。

## 5. R3 第 46 条台阶离线预判

- 编号：MAP 所说的「46→47 条」= 0 基 op-0045→op-0046，对应 generation 46→47。
- 旧库逐代写入量（`tools/r3_offline_churn.py`，immutable 只读，结果在 `archive-probe/r3-offline-churn-old-db.json`）：g47 是常规的「确认 1 笔 + 1 段」代，与 g19/23/31/35/43 同型，发布 39、撤回 23。batch 4.63→4.98 MB（+7.6%），delta 4.12→4.44 MB，增幅与前几代相当，**没有写入量跳升**。累计约 154 MiB，低于 audit cache 的 256 MiB，越界假设不成立。
- 旧 events/q-sidecar：op-0046 窗口内 Q 没有任何请求，排除 Q 争用。
- 行情：bar45 高点 40000，bar46 回落到 36000，op-0046 是顶分型确认点。g47 新增笔 41→45（30000→40000）。
- main 同库逐代（`compare/r3-churn-main-run-a-wal.json`，WAL 已并入，52 代；见 §6 第 4 条）：g46→g47 batch 4574098→4924102 字节（+7.7%），发布 37、撤回 21，写入形态与旧库几乎相同。
- **耗时结论（0055/F3 降级）**：main 本次两跑在 op-0045→op-0046 **未观测到**台阶（run-a 6458→6919 ms，run-b 10512→10868 ms），但这**在宿主噪声下不可判**，不能写成“main 不复现”：
  - run-a 自己在 op-0041→op-0042 就有 5267→9322 ms（+4.05 s）的跳变，与旧台阶（+4.05 s）同量级，却不对应任何写入量变化；
  - run-b 从 op-0041 起有 11 条 >10 s，两跑总耗时相差 1.4 倍（§2.5 宿主负载）。
- 写入量侧的排除仍成立：旧台阶与结构写入量不同步，**不属于 R2**。耗时侧的候选（旧二进制 center-seed/1 的 TB-02-E 扫描、Mac 宿主因素等）本切片无法区分。
- **10 s 帽证据须由 S6 在安静宿主上取得**（独占宿主、记录负载，按 D11 带 phase trace 定因）。本切片的耗时数据不构成帽内或帽外证据。S6 需要知道的前提：#1419 center-seed 合入 main 之前，旧二进制独有的中心路径在 main 上不存在。
- 局限：没有旧二进制的 phase trace，上述只是写入量侧的相关性排除，不是根因证明。

## 6. 偏差与未决

1. run-b `stop_exit=1`，stderr 未记录（初版驱动没有保存 stop 的 stderr）。已核实 WSL 上没有残留进程，两跑最终库主库文件与逻辑内容相同（§2.3），oracle 不受影响。按 0055/F8 修后的驱动口径，这一跑会被判 `ok=false`（见第 7 条）；本次未重跑，原因见第 6 条。
2. 回复期限和 Q 资源为了 oracle 生成而放宽（§2.5），本切片**不出帽判定**。
3. 宿主共享负载使耗时有噪声（run-a/run-b 相差 1.4 倍），第 46 条台阶在 main 上不可判（§5）。
4. （0055/F4 订正）初版称 `r3_offline_churn.py` 在 main 库上只输出 g1..51 是“脚本按增量对计数”，**原因写错了**：实因脚本用 `mode=ro&immutable=1` 打开，忽略了未 checkpoint 的 WAL（g52 只在 WAL 里）。脚本已改为：旁有非空 `-wal` 时先在临时副本上 checkpoint 再只读打开，原库不动，输出记 `wal_applied`。在 G: run-a 原库上重跑（原库 sha 前后不变）：52 代，g1..51 与初版 `compare/r3-churn-main-run-a.json` 逐项相同，g46/47 不变（4574098→4924102，37/21），g52 batch 6019116 字节；与 0055 独立结果 `r3-churn-main-run-a-with-wal.json`（e726930e…）的 generations 完全相同。新结果 `compare/r3-churn-main-run-a-wal.json`（sha `b08ced9273c1…`），初版文件保留。
5. （0055/F7）`tools/run_wsl.sh` 在 `set -e` 下 `rc=$?` 捕获失效，驱动失败时不会拷回 G:。已改为 `驱动 || rc=$?`，无论成败都拷回 G:、写 `DRIVER-EXIT` 与 `SHA256SUMS`，并以驱动退出码退出。WSL 上用假 `python3.12`（exit 7）实测：脚本 exit 7，G: 有 RUN.json、`DRIVER-EXIT`=7、`SHA256SUMS`（测试目录已清理）。
6. （0055/F8、F9）驱动 `tools/min12_oracle_run.py` 已改：`digest_match=false` 立即失败；RUN.json 记 `error`、`stop_stdout`/`stop_stderr`（各留末 4000 字符）、服务停止后对原库 checkpoint 并记 `db_digest`（`tools/db_logical_digest.py`）；`ok = 摄入完成 ∧ 查询完成 ∧ stop_exit==0 ∧ checkpoint 成功`，否则退出码 1。本机用假 launcher/假 Q 实测失败路径：摘要不符 → 退出 1、`ok=false`、`stop_stderr` 留存。**未重跑全量 oracle**：F8 的判据（104/104 摘要一致）已由既有产物满足，F9 的逻辑比对可在 G: 原件副本上离线完成（§2.3），两者都不需要新跑。
7. （0055/F10）构建来源复证：WSL 上按 SHA `6badaa6049` 新开 worktree、全新 `CARGO_TARGET_DIR`、`--locked` 冷构建 `cargo build --release --features s_session --bin s_structure_session`（rustc/cargo 1.99.1，worktree `git status --porcelain` 为空），产物 sha256 = `c944bd6b…e040`，与 S2 原件逐位一致；S1 此前在另一 worktree、另一 target 目录独立构建同一 SHA，也得到同一哈希。说明该二进制可由 SHA 复现，构建树 clean 的残余疑点已消除。日志：`compare/fix-0056/f10-*.log`（S2 原构建日志末尾 `EXIT=1` 来自追加的 `ldd | grep -i python` 无匹配，构建与拷贝本身已成功）。今后驱动在 RUN.json 记 `repo_status_porcelain`。
8. 独审：0055 已完成（APPROVED_WITH_FOLLOWUPS）。本版订正项的审读入口：`compare/semdiff2-known*.json`、`tools/semdiff.py`、`compare/fix-0056/`、`compare/r3-churn-main-run-a-wal.json`。

## 7. 复现

```bash
# 二进制（WSL，按 SHA 冷构建可逐位复现 c944bd6b…e040）：
#   git checkout 6badaa6049 && cargo build --locked --release --features s_session --bin s_structure_session
# 运行（WSL，ref 已 fetch 到 NewChanlun）：
bash .chanlun/review-results/1488-acceptance/min12/tools/run_wsl.sh c 18xxx
# 对照（v2）：
python3 .chanlun/review-results/1488-acceptance/min12/tools/semdiff.py <known-31/committed-candidate.json> <run-x/oracle/g-0032.json>
python3 .chanlun/review-results/1488-acceptance/min12/compare/fix-0056/check_toplevel.py <known-N> <g-(N+1)> .chanlun/review-results/1488-acceptance/min12/tools
python3 .chanlun/review-results/1488-acceptance/min12/compare/fix-0056/check_raw.py <含 k15/k31/g-0016/g-0032 的目录>
# 库逻辑内容（默认在副本上做）：
python3 .chanlun/review-results/1488-acceptance/min12/tools/db_logical_digest.py <run-x/session.sqlite>
# 逐代写入量（自动并入 WAL）：
python3 .chanlun/review-results/1488-acceptance/min12/tools/r3_offline_churn.py <run-x/session.sqlite>
python3 .chanlun/review-results/1488-acceptance/min12/tools/readings.py <run-x/STRUCTURAL.jsonl>
```

# #1419（TB-02-E）交付报告：中枢 strict seed、Zn 外缘、C 口径续扫与 S 事实层

> 名分：工作草稿，绑定 [#1419](https://github.com/xy7365527-lang/NewChanlun/issues/1419)，票关后随归档出仓。实施工位 `/root/tb1394_support_r4_review`，分支 `codex/1419-center-seed-20261013`（基线 main `465174240b`，已两次合入 main，最近一次 `9cd64da5cd` 合 `65d522346f`）。本报告随本提交入仓，所述头为本提交的父提交 `48e2386463`。收据在本目录 `receipts/`，原件在仓外 `tb1419-impl/`。

## 关票状态

代码、Lean、合同与教义正本已落齐，L3（塔对 S 写者）在真实数据上 9/9 切点逐位通过。关票前还差三道门，都不在本报告里自行放行：

| 门 | 状态 | 由谁 |
|---|---|---|
| S9 正式运行验收 | **未跑**。S0b、F1a 与最小流探针已跑完（下节）；根裁 D-E 按 (B) 口径收窄正式运行覆盖面，harness/runtime/verify/浏览器脚本待实施，浏览器槽由根排 | 本工位跑，根判 |
| 独立终审 | **未做**。S4 之后的增量 `1472b7bb..1bf54857` 与 `9cd64da5..48e23864` 没有独立终审过；根定于探针跑完后派审查员做 #1419 全量终审 | 根派 |
| 合入 | 按定序在 #1394 之后，rebase 义务见末节 | 根 |

## 提交

| 片 | 提交 | 内容 |
|---|---|---|
| S0 | `e49efb7a38` | 独立 oracle 与工作例冻结（`ORACLE-DESIGN.md`、`oracle_check.py`、`worked-examples.json`，20,849 项断言 0 失败） |
| S1 | `7cf13e7315` | R8：结算后自 `last_member + 2` 续扫，开放窗口游标不越界 |
| S2 | `65cf45e937` | 唯一 `center::is_zn`；`compute_gg/dd` 与延伸循环只聚合 Zn |
| S3 | `0cff16b33a` | Lean 中枢外缘同拍改 Zn 并带方向；`ref_v1.rs` 改 C 口径；parity 夹具重导出 |
| r2-1 | `ee739fbea6` | 走势区间与中枢外缘分开（缓存字段保住 O(1)）；Lean `Move.interval` 同拍 |
| r2-4 | `51887e8bff` | chain 族三条锁升到 160 段并断言 `skip_edges > 0` |
| r2-5 | `af92b9b8a0` | zhongshu.md 落码同步（中枢外缘不是走势区间、§9.5、§9.6） |
| R4 | `1472b7bb6b` | parity 夹具只对拍已结算中枢的 `break_up` |
| S1′ | `639c063769` | seed 唯一判定 `seed_facts`，两个 build 改为其投影 |
| S4 | `bd1d3aa0ad` | 共用扫描内核 `scan_center_facts` 与事实侧车（#804 L2） |
| INFO-1 | `399c3a3bac`、`7fc327c07d` | Lean `OpenTail.currentExtreme` 改取走势区间，修 `foldr 0` 哨兵；§9.6 补 L14 |
| S8 | `aeed7ee037` | Q 合同与浏览器校验；Python/JS 共享语料逐条对拍；CI 接入 |
| S7 | `10f951889b`、`1bf5485767` | S 写者发布中枢事实（schema v1.1）；§9.6 P16 |
| E4 | `9b7e29a054`、`bcd1e7db22` | 非 UPGRADE 中枢每成员恰一条 `member_of` 边；schema v1.2 入仓 |
| L3 | `ecc9b196d3` | L0 单元区间改取成员笔方向极值（根裁定 L3，schema v1.3）；真实 BTC 判别夹具 |
| L3 文档 | `d252807bdc`、`48e2386463` | xianduan §8「段区间」与 X-21～X-24；zhongshu §9.6 R30、L15；读数订正 |
| 本提交 | — | 本报告与收据；CI 加跑 `test_tb02e_real_units` |

期间两次合入 main（`60b59b479a`、`9cd64da5cd`），无冲突。

## 验收

票面四条：

| # | 验收 | 状态 | 证据 |
|---|---|---|---|
| T1 | 三个连续次级单元到 strict seed 有见证，正式核心非退化 | 通过（单元、Lean、真实数据层） | `seed_facts` 小域穷举与 E1a/b/b′/c；Lean 判别夹具；真实数据 L3 seed 尝试 S 与 oracle 各 11 次逐位相同 |
| T2 | 核心与全部动态外缘同时保留；后续成员只改适用角色；连接与未归属尾部不偷吞 | 通过（单元与合同层） | `tb02e_k10_*`、`tb02e_k11_k12_*`、`tb02e_inner_extrema_*`；Q 合同共享语料（含连接段作成员、+1 复用、D/G 冒充核心反例） |
| T3 | 正式会话入口跑非真空输入；公共查询与变化；浏览器核对 | **部分**：S 二进制集成测试 `tb02e_input_contract_mismatch_rejects_whole_cut_and_keeps_previous_facts` 等过；正式四臂与浏览器未跑（S9） | `receipts/l3-3-s-session.txt` |
| T4 | 独立 oracle；同输入新进程重放；恢复对拍 | **部分**：oracle 先于实现冻结（S0）；同输入两次 build 逐字节相同；a/b 新进程对拍与 K10/K12 恢复未跑（S9） | `tb02e_same_input_rebuilds_byte_identical` |

细项 A-* 的射程按计划 §1.2（正式运行的覆盖面另按下文「探针读数与正式运行的覆盖边界」收窄）：线段级全部由单元、合同与 L3 证据覆盖；上级路径、≥9 子窗、三类买卖点、扩展新生按版本对象 `deferred_scope`（`global_G009_upper_levels_1429`、`ge9_upgrade_1429`、`third_class_and_break_1434`、`expansion_newborn_1424`、`connector_upper_construction_1429`）延后，目录轴 `impl_status` 保持 `not_implemented`。需要运行证据的细项（A-CC-016-03/017-03/022-03 的 a/b 重放、A-OB-008/016 的 delta 与重连、浏览器卡片）随 S9 给出。

本头读数（收据 `receipts/l3-*`）：

| 项 | 读数 |
|---|---|
| `cargo test --all-targets --no-fail-fast` | 55 个结果行，3171 过 / 3 败 / 173 忽略；lib 2981 / 3 / 162。3 败与基线 3 红逐名相同：`tick::seam_btc_tick_bars_yield_non_trivial_structure`、`tick::calibration_sweep_finds_non_collapsed_segments`、`l3_delta_r_alpha::m3_partition_synthetic_holds` |
| `--features s_session --bin s_structure_session` | 93 过 / 0 败 / 1 忽略 |
| Python | 九个模块 60 例 OK；tb02c 三模块 14 例 OK（ijson venv） |
| node | `test_tb02e_contract.cjs` 2、`test_tb02c_contract.cjs` 6、`test_tb01c_browser.cjs` 1，全过 |
| `cargo fmt --check` | 净 |
| Lean | INFO-1 后 `lake build` 158 jobs 通过，无 sorry（`tb1419-impl/receipts/info1-lake-all.log`）。fixture 漂移检查最近一次在 r1（三夹具无漂移，`r1-drift.out`），**早于 `1472b7bb6b` 对 `CenterConstruct.lean` 导出器与 `theta_v0_center_parity.json` 的同拍改动**；之后只跑过 parity 测试本身（`r4-center-parity.log`），没有重跑漂移检查。本地 wip 树 `formal/.lake/build` 是冷的，补跑要先全量 `lake build`，排在探针之后、终审之前（CI `fixture-drift` 腿也覆盖） |

变异核对：S1–S4 各片先红后绿并做回退变异（`tb1419-impl/receipts/s*-red.*`、`mut-*.log`）；Q 合同的 Python 侧与 JS 侧变异各被共享语料对拍抓到（`s8-mutation*.log`）；L3 几何端点变异体 Rust、Python、JS 三侧各自变红（`receipts/l3-mut-*-geometric.txt`）。

## 受影响代码清单

正本在 `.chanlun/definitions/zhongshu.md` §9（R21–R30、L9–L15、P14–P16，§9.6 每行行号对其提交列）与 `.chanlun/definitions/xianduan.md` §15（X-21～X-24）。本票落码与登记如下：

| 侧 | 已落 | 登记未改（带退场条件或归属票） |
|---|---|---|
| Rust | R8 `recursive_tower.rs:603`；R21 `:537`；R23 `center.rs:176/184/193`；R25 `ref_v1.rs:165/172/203`；R26（输入已是 Zn 外缘）；R27 parity 夹具；R28 走势区间缓存 `recursive_tower.rs:148/320/775`；R29 `nest_lifecycle/window.rs:345`；R30 S 写者 `tb02e.rs:82/111`；`seed_facts`、`scan_center_facts`、`center_scan.rs` | R22 ≥9 子窗外缘（§9.5 #804 例外，退场 = #1429 落上级 Zn）；R24 旧线 `zhongshu.rs` 归 TB-10；X-21 `segment.rs:687-726` 不改（已是成员笔极值，L0 单元区间唯一来源） |
| Lean | L9 `CenterConstruct.lean:395/415`；L10 `:452`；L11 `CenterConstruction.lean:108/133/137`；L12 `RecursiveConstruction.lean:56/60`、`PromQual.lean:292`；L14 `DivergenceNesting.lean:436/654` | L13 `CenterDerivedAt` 并入 §9.5 同一例外；**L15／X-24 `CenterConstruction.lean:52/56` `segHigh/segLow`**：须在定义处写明「端点价即标准化后的段内极值」为定义前提，或让 `Segment` 带成员笔——归 Lean 跟进票 [#1486](https://github.com/xy7365527-lang/NewChanlun/issues/1486) |
| Python/JS | P16 `s_tb02_contract.py` `validate_center_*`、`_member_extremes:1195`、`_segment_units:1205`；`tb01c-client.js` 同名 JS（`memberExtremes:821`、`segmentUnits:827`）；`s_query_integrity.py` 中枢轴；`index.html` logicalSlot | P14 `a_zhongshu_v1.py`、P15 `a_center_v0.py` 归 TB-10 |
| 文档 | zhongshu.md v2.2（§3「中枢外缘不是走势区间」、§6.1 落码现状、§9.5、§9.6）；xianduan.md v2.2（§8「段区间 = 成员笔方向极值」，`078:58`／`:60`／`:42` 四步核） | — |

## schema

`tb1419-schema.md` 现为 **v1.3**：v1.1 经合同复核 PASS 冻结（`tb1419-review-r2/schema-v1.1/RESULT.json` 4726a10e…），v1.2 按复核 E1–E4 改措辞，v1.3 只改 §1 一句：完成单元区间由几何端点 `sort(stroke_refs[0].start_price, geometric_end.price)` 改为成员笔方向极值（`stroke_refs` 中下标属于 `construction.stroke_indices` 的笔）。文首列有各版差异与根裁定指针。v1.3 未另经合同复核；改动的唯一一句由真实数据 L3 证据与三侧变异体锁住，复核归全量终审。

## L3 真实数据证据（基于 #1405 未合入头）

本节证据全部来自本地组合树：#1419 `48e2386463` 的代码，加 **#1405 未合入头 `199b820c9d`**（不提交、不推送）。原因是本分支自身的 parser 在真实 BTC 1m 上不成笔（同价域问题，#1405 修），所以真实数据上的塔对 S 写者对拍只能借 #1405 的 parser 做。**#1405 合入后须在合入态复跑一次**（见末节）。

数据为 BTC 1m 尾 49,995 根（`btc_tail50k.json` sha256 `4c005de4…`），切点 600、1k、2k、5k、10k、20k、30k、40k、49,995。结果：

- S 输入合同 9/9 PASS；S 单元与塔单元全量切点 72/72 相同；
- S 写者对塔、S 对独立 oracle、塔对 oracle，三组比较 9 个切点全部零差异；
- Q 合同（Python）与浏览器合同（JS，node 中运行）各 9/9 ACCEPT；
- 全量切点：塔 16 个 L0 中枢 = 8 个成员 <9 的窗口（三方逐位相同）+ 两个成员 ≥9 的窗口里塔切出的 5 + 3 个子中枢（S 标 `UPGRADE_PENDING_TB02H`，子中枢语义归 #1429）；
- 比较器变异四个全被抓（`receipts/mutate-l3-compare.txt`）。

判别力：每个切点都有几何端点区间与成员笔极值不同的段（全量 35/72；按方向计 32 段起点、16 段终点不是段内极值），所以 9 个切点都是 v1.2 口径会拒、v1.3 口径须过的判别例。裁定前（v1.2）同一数据上 S 写者 9/9 被输入合同拒绝，那份读数留在 `receipts/REALDATA-L3.md`「v1.2 读数」节。全文、方法、产物 SHA 见 `receipts/REALDATA-L3.md`，逐切点摘要 `receipts/l3-readout-summary.json`。

## 探针读数与正式运行的覆盖边界（D-E/D-F，2026-10-14）

三次探针都在根开的槽内各跑一次、不重试，属树外诊断、不进准入证据。全文在本目录 `probes/`，原件在 `/Volumes/AgentStorage/issue1419-{s0b,f1a,s0b-min12}-20261014/`。计时背景：1 分钟负载 7.2–9.6，没有 cargo/rustc/lake/S/Q 在跑，背景是用户自己的 Chrome 154、agent-browser 的无头 Chrome、WindowServer、CLIProxy（`probes/probe-background-load*.txt`）。

- **S0b 规模**（写者 `48e2386463` release，sha256 `38c61eee…`）：逐根单事件发送，UP 与 SHORT11 两臂都在第 48 条（下标 47）回复超过 10 s 帽。第 40–47 条耗时 4.2→5.5 s，第 46 条跳到 9.5 s，第 47 条过帽。拟合单条约 1.34·n^2.16 ms，与 TB-02-D 一致。batched 两臂作废：S 协议要求一条 ingest 只带一个事件，属探针前提错误。
- **F1a 浏览器打点**（TB-02-D 冻结包副本，Chrome 155.0.8059.40，冻结时为 154.0.8037.99）：两臂通过，浏览器窗 29.55 s 与 28.39 s，**贴着 30 s 帽，余量 0.45 s 与 1.61 s**。时间主要花在等提交（约 14.8 s）和点击（约 9.1 s，首次点击 4.7–4.8 s）；页内 JS 只占约 2.5 s。**Q 冷 worker 首次 verify 4.5 s，贴着 5 s 期限**。帽不放宽。
- **最小流 MIN12**（根裁 (A)）：12 笔、52 根，每笔 4 根即新笔下限；按规则最少要 51 条才有 3 个完成段。离线 parser 实测与独立 oracle 逐项相同：第 51 条起 seed FORMED，ZD 17000 < ZG 42000，中枢 EXTENDING，DD 12000，GG 52000。正式会话单臂在第 48 条过帽，耗时曲线与 S0b 逐条重合，所以单条代价只随根数变、与段结构无关。

**正式运行覆盖边界（根裁 D-E 取 (B)）**：S9 正式会话只跑到第 47 条以内的前缀，即 2 个完成段加 1 个进行中段。正式运行能证明的是 TB-02-E 在正式入口、公共查询与浏览器上非空（CC-022 单元归属、CC-056 版本与游标、等待原因），以及 a/b 新进程重放与恢复；**CC-016 seed 与中枢、CC-017 角色在正式运行里不出现**。E2/E3 等需要 14 段的工作例，以及 seed、角色、续扫、归属的全部断言，改由三部分承担：单元测试（`tb02e_*`、`center_scan_1419`）、oracle 全切面核对（`tb02e_writer_output_passes_python_center_contract_on_all_oracle_cuts`）、真实数据 L3（上节）。原因：S 摄入每个 cut 全量重算，代价随历史超线性。根已确认这与 #1481 阶段 0 测得的「非审计路径 ∝ g^1.56–1.84」同一根因，将另开构造降阶票，本票只登记指向；MIN12 流已冻结（`probes/min12/`），供降阶后作验收输入。服务路径上第 46 条附近的单条跳变（5.4 s → 9.5 s，三条不同的流在同一位置出现；离线 CLI 逐条调用时没有）原因未查，一并登记给该票。

## 登记：段数在 5k–30k 根停在 24

真实数据上段数从 5,000 根到 30,000 根一直是 24，这 25,000 根上没有新的已确认段。推断是某个第二种候选长期等不到分型。组合树不含 #1420 的 `078:52` 失效语义，**本票不修**，留待 #1420 合入后在合入态复测。复测入口同 L3（`tmp_tb1419_realdata_l3_dump` 的切点阶梯），比较段数序列。

## 合入前义务

**次序**：#1394 先合，#1405/#1419/#1420 后合，后合者 rebase（同 #1405 交付报告）。#1419 与 `origin/codex/1394-integration-20261013`（`0fc0c165df`）共同改动的文件有八个：`.github/workflows/ci.yml`、`tb02a.rs`、`tb02a_facts.rs`、`s_structure_session.rs`、`bsp.rs`、`index.html`、`tb01c-client.js`、`s_query_integrity.py`。其中 #1394 的改动量远大于本票（如 `tb01c-client.js` +7248/−735、`s_structure_session.rs` +4599/−680），rebase 时以 #1394 的结构为底移植本票改动。

1. **RULE_REVISION 三段合成。** 本票 `s_structure_session.rs:65` 为 `s2-axis-quantifiers+new-bi/1+segment-second/1+center-seed/1`；#1405 为 `…+new-bi/2+segment-second/1`，#1420 为 `…+new-bi/1+segment-second/2`，#1394 整合分支在 `:97` 仍是 `…+new-bi/1+segment-second/1`。按实际已合的票合成一个值，三票都在前时为 `s2-axis-quantifiers+new-bi/2+segment-second/2+center-seed/1`。随之：
   - `s_tb02_contract.py:23`、`tb01c-client.js:148` 的 `CENTER_RULE` 由 `SEGMENT_SECOND_RULE` 派生，跟随即可；
   - #1405 的 `RULE_CONTRACTS`（py/js）加中枢规则一行，取合成后的新笔、段合同代际与知识域代际（组合树的取法：`s-new-bi/2`、`s-segment/2`、知识域 V2）；cut 校验并用 #1405 的代际绑定与本票的 RULE_MIX 门；
   - `s_query_integrity._published_axes`：段规则取 `SEGMENT_SECOND_AXES`，中枢规则取 `CENTER_AXES`；
   - `index.html` logicalSlot 同时认 `s-new-bi/2` 与 `s-center/1`。
   组合树里这些冲突的实际解法记在 `receipts/REALDATA-L3.md`「组合树合并」节，可直接参照。
2. **浏览器合同移植。** #1394 把 `tb01c-client.js` 改为 async，且 `q3-typed.js` 内嵌了 TB-02 kinds（含段族 `SEGMENT_KINDS` 与 `validateSegmentPayload`）。本票的 `CENTER_KINDS`、`validateCenterPayload`/`validateCenterLinks`/`validateCenterCut`、`memberExtremes`/`segmentUnits` 须移植进新结构，typed 路径若校验 TB-02 kinds 也要纳入中枢 kinds。移植后 `test_tb02e_contract.cjs` 与 `test_tb02e_real_units.py` 的 Py/JS 逐条对拍须照常通过。
3. **bsp 白名单重锚。** `bsp.rs` 的 `no_stray_bsp_binding_rewrites_outside_registry` 按行号登记。本票只动了一条：`pipeline_geometry.rs:1720 → 1698`（`bsp.rs:765`，#1394/#1405/#1420 都不改该文件，rebase 后应保持 1698）。`tb02a_facts.rs` 那组锚以 #1394 整合分支收成的 `:564` 一处为准，丢弃 #1405 F 片的 231/241/267 三行；本票在 `tb02a_facts.rs` 有 +20/−2，若落在 `:564` 之前会把它推后，**以 rebase 后该测试实报的行号为准重锚**，逐条注明「内容不变、行号漂移」。
4. **夹具与 a3_oracle 重生成约定。** 一律重跑生成器，不手改产物；断言不放宽、不删除。
   - `s_session/tests/fixtures/tb02e-contract/corpus.json`：`generate-corpus.py:13` 的 `CENTER_RULE` 字面量改为合成值后重跑生成器，Py 与 JS 两个语料测试同过。
   - `s_session/tests/fixtures/tb02e-real/btc-tail50k-first600.json`：现为 #1405 未合入头 parser 的冻结输出（见其 provenance）。#1405 合入后在合入态用同一导出路径重导，`discriminating_units` 若变则随之更新；测试已断言至少一个判别单元。#1420 若改变前 600 根的段层，同样重导。
   - `rust/tests/fixtures/theta_v0_center_parity.json`：Lean 导出件，只在 Lean 改动时经 `scripts/check_fixture_drift.py` 重导（退出码 1 才是真漂移，3/4/5 是工具链失败）。
   - `backtest::incremental::a3_oracle_pop_rescan_empty`：合成夹具的覆盖断言（`t_eq1 > 0`、`t_gt1 > 0`、`empty_break > 0`）依赖段层输出。本票 R8 已把它从 12 簇 / `take(267)` 调到 14 簇 / `take(316)`（读数 T==1=239 / T>1=2 / empty_break=4，理由见 `S1-FAILURE-RULINGS`）。#1420 的 `078:52` 失效语义改段层，#1420 自己也改了同文件 `:167` 起的另一个夹具。rebase 后若任一覆盖归零，照 S1 的做法扫簇数与截断长度重新取参，在测试 doc 注释写明新读数与原因；`a3_oracle_minparts_reentry` 与 chain 族 160 段锁（`skip_edges > 0`）同样复核。
   - 其它绑定会话规则串的夹具（`r12_consumer.json`、`test_tb02c_verifier.py`、`test_tb02d_*`）属各自的票，按根要求随规则串一起重生成。
5. **合入态复跑。** rebase 后在合入态跑 all-targets、S 二进制、Python/node 合同；并用真实数据复跑一次 L3 与段数阶梯（替代本报告「基于 #1405 未合入头」的那份证据）。

## 其它登记

- #1478、#1429 的票内登记（依根裁定 r2 第 5 条，草稿在 `tb1419-impl/comments/`）已于 2026-10-13T18:20Z 发出，根已核。
- Lean 跟进票已开 [#1486](https://github.com/xy7365527-lang/NewChanlun/issues/1486)（L15／X-24 `segHigh/segLow` 对齐义务）。
- 回测基线差异（D-K，附件不作验收门）：`tb1419-impl/BASELINE-DIFF-S1S2.md`，合成段流三种子；S1/S2 不改 L0 中枢窗口与核心，只改外缘与中心定理二关系，上级塔变浅、中枢变少，上级验收归 #1429/#1424。
- S1 的 16 条变红逐条判定：`tb1419-impl/S1-FAILURE-RULINGS.md`，全部是口径回归。
- 生产 parser 在真实 BTC 上不成笔（本分支自身）：由 #1405 修，本票不修。

## 收据（`receipts/`）

`REALDATA-L3.md`（L3 全文）、`l3-readout-summary.json`、`analyze-l3-v1.3.txt`、`mutate-l3-compare.txt`、`js-validate-l3.txt` 及三个比较脚本（`analyze_l3.py`、`mutate_l3_compare.py`、`js_validate_l3.cjs`，路径指向仓外组合树，复跑需该树）；`l3-1-tb02e-tests.txt`、`l3-mut-{rust,py,js}-geometric.txt`、`l3-2-all-targets-summary.txt`、`l3-3-s-session.txt`、`l3-4-py-node.txt`。40 MB 的 `l3-dump.json` 与 3 MB 整数 K 线不入仓，SHA 见 `REALDATA-L3.md`。

探针（`probes/`）：`S0B-RESULT.md`、`F1A-RESULT.md`、`README.md`（含订正）、`s0b-probe-summary.json`、`f1a-timeline-summary.json`、背景负载与 release 构建收据；`probes/min12/`：`MIN12-RESULT.md`、生成器 `min_gen.py`、冻结输入 `s0b-MIN12.json`、离线 parser 跑法 `min12_parser_run.py` 与逐条耗时 `parser-run-LOG.json`、oracle 读数 `ORACLE-MIN12.json`、正式会话单臂摘要 `min12-probe-summary.json`。

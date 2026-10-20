# #1405（TB-02-V）交付报告：落 #1343 同价裁定

> 实施会话 `/root/tb1405_same_price_impl`，分支 `codex/1405-same-price-20261014`（基点 `0d4dc644` = main `3da0b2c8` + #1343 文档）。独立评审尚未进行，由根另排会话。收据在本目录 `receipts/`，原件在仓外 `tb1405-impl/receipts/`。

## 结论

生产新笔状态机已按 #1343 裁定处理同价：相邻同型同价保留在先者（只记证据，不是等待态）；多根组端点以极值根集合为身份，条件 2 的 raw 间隔按极值区计；`blocked`、直通与同价弹笔整体退役。真实 BTC 1m 尾 5 万根从 0 笔 0 段变为 **4,016 笔 / 72 段**，笔序列指纹 `4fd87a7e429962ac` 与 #1343 的 Python 复算逐项相同；`tick::seam_btc_tick_bars_yield_non_trivial_structure` 转绿。S 发布合同升 `s-new-bi/2`、会话规则升 `…+new-bi/2+…`，正式会话入口 + HTTP 公共查询 + 真 Chrome 页面三层都能读到组锚、极值根集、`raw_first`/`raw_last`、两段 policy 与同价 KEEP 证据。按根裁定 B，TB-02-A 里同一「同价 = 未定」前提的三处（CC-004/CC-005 请求、CC-006 发布门）一并退役。HEAD `ac9ef97c03` 实测 `cargo test --lib` 2960 过 / 2 败 / 162 忽略，失败集恰为两个基线既有红，均已分诊且与同价取法无关；D 片曾让 bsp 白名单因行号漂移多出一个红（独立评审查出），已在 `c823155175` 重锚。

## 提交

| 片 | 提交 | 内容 |
|---|---|---|
| A | `355c1d8b46` | `stroke.rs` 唯一 transition 落裁定；`Endpoint.raw_first/raw_last`；`RULE` /2 + `SAME_PRICE_POLICY`；行为锁 `stroke_tb02v_tests.rs`；六见证夹具；tb02b 手算账簿 v2；`mod.rs:611`；env 登记两键 |
| B | `1f90227633` | S 写者 `tb02b.rs`（`s-new-bi/2`、两段 policy、成员范围覆盖两端极值区、event_at 取终点极值区首根）；py/js 合同 v1/v2 并存；`tb02b_verify`/harness/独立对照脚本跟随 |
| C1 | `1bfb91a0e1` | 正式会话入口验证脚本 `s_session/tests/tb02v_runtime.py`；`stroke_tb02v_tests.rs` rustfmt 订正 |
| C2 | `12d1f1223f` | 会话级 `RULE_REVISION` → `s2-axis-quantifiers+new-bi/2+segment-second/1`；`RULE_CONTRACTS` 按会话规则绑定合同代际；顺序敏感测试按 #1419/#1420 同款修正 |
| D | `93e9097915` | TB-02-A 退役同价「选根」门（根裁定 B）；知识域升级；TB-02-A 手算账簿 v2 与 `check_root_set` |
| E | `199b820c9d` | 本报告与收据（初版） |
| F | `c823155175` | bsp 白名单随 `tb02a_facts.rs` 行号漂移重锚 237/247/273 → 231/241/267（D 片删窗口请求块后整体上移 6 行，构造与理由不变） |
| G | `ac9ef97c03` | 会话规则→新笔合同代际绑定的执行点反例：py `validate_fact`、js `validateTB02Candidate` 各两条（new-bi/2 会话拒 `s-new-bi/1` 对象，及反向），拒因同为「新笔合同与cut规则版本不符」；为此导出 `validateTB02Candidate` |
| H | 本提交 | 按独立评审订正本报告与收据 |

## 验收矩阵

| # | 验收 | 结果 | 证据 |
|---|---|---|---|
| 1 | 六见证经 Rust 生产入口得 WITNESS-WALK 输出 | 通过（stride 1/7 两种坐标，`parse_layer` 与 facts 入口逐字段相同，全程无等待） | `tb02v_six_witnesses_through_production_entry` |
| 2 | 尾 5 万根读数 | merged 30,863 / 分型 14,017 / 笔 4,016 / **段 72** / 指纹 `4fd87a7e429962ac` / keep_eq 53；首三笔 T@15→B@26、B@26→T@42、T@42→B@55，末笔 B@49979→T@49996；事件 formed 4016 / replace 2649 / keep 2298 / hetero_not_formed 5000 | `receipts/tail50k.txt`（`#[ignore]` 外部数据测试，数据先验 sha256 `4c005de4…601f`） |
| 3 | seam_btc_tick 转绿；另两红实测分诊 | seam 转绿（不改断言、不改夹具）；另两红见「红项分诊」 | `receipts/lib-tests.txt` |
| 4 | 前缀不回退与 full ≡ 增量 | 六见证逐前缀通过；九切点阶梯全过（已确认前缀逐字段相同、末笔起点相同、终点只延伸且严格更强、从不 pop），每个切点增量 `StrokeFacts` 与全量重建逐位相同 | `receipts/tail50k.txt` 的 `TB02V_LADDER` 行 |
| 5 | 选择无关 / A/B 同值 / 双进程哈希 | 160 例随机多根输入打乱并列根到达序，`StrokeFacts`（根集按集合比）逐位不变；每个异型对的间隔 = 对两端任意指定并列根取最小（尾 5 万根 9,016 对零不符）；无同价单根输入极值区读数与单根序位同值，tb02b 账簿其余 14 例 v1 期望不变；两个新进程重放哈希相等 | `tb02v_tied_root_order…`、`tb02v_single_root…`、`tb02v_replay_hash…` |
| 6 | 变异体 | Rust 9/9 全杀：删 KEEP 分支、同价不记证据、单根最早/最晚、恢复 blocked、恢复 TB-02-A 门、组/步骤请求重新等待、旧知识域。合同变异（独立评审九个）在 G 片后 7/9 杀：C3（删 py 会话规则绑定）只红两条新 py 反例、C8（删 js 绑定）只红两条新 js 反例；存活的 C5（近似等价）与 C9（js `before` 代际分派，同会话不混代时不可达）是评审登记的 INFO | `receipts/mutants.json`、`receipts/contract-mutants.json` |
| 7 | 正式会话入口 + 公共查询 + 浏览器 | real_C 27 根：会话 27 cut PASS（三轮：B 片、C2、D）；根批的一次浏览器槽 PASS（2 分 16 秒，0 page error，五轴页面原件字段齐全，进程/端口已回收） | `receipts/session-realC.json`、`receipts/browser.json`、`receipts/browser-excerpts.md`、`receipts/browser-final.png` |
| 8 | 失败集 / fmt / all-targets / DevSkim | HEAD `ac9ef97c03` lib 2960 过 / 2 败 / 162 忽略，失败集恰为 m3 / calibration（基线 2955/3/161 的子集；`199b820c9d` 曾为 2959/3/162，多出的 bsp 红已由 F 片重锚）；py 合同十模块全过（`test_tb02b_contract` 14 例），node 四文件 36/36；fmt 净；`--all-targets` 默认、`backtest_bin`、`s_session` 三档能编；S 二进制 81/0；DevSkim 预登记见 `DEVSKIM-PREDELTA.md`，CI 增量待根开 PR 后按 #1404 先例补 | `receipts/lib-tests.txt`、`receipts/s-session-bin.txt` |

## 逐提交可编译声明

五个代码提交 `355c1d8b46`／`1f90227633`／`1bfb91a0e1`／`12d1f1223f`／`93e9097915` 各自 `cargo check --all-targets --features s_session` 0 错，由独立评审逐提交实跑（`tb1405-review/runs/per-commit-check.txt`，已 touch 全部文件排除 cargo 误判 fresh）。`c823155175` 只改 `bsp.rs` 测试模块内三个字符串字面量，`ac9ef97c03` 不动 Rust，二者之上的 lib 测试目标在 HEAD 实编实跑（`receipts/lib-tests.txt`）。`199b820c9d` 与本提交只含文档与收据。

## 实施中自定的形状（根已逐条采纳）

- `policy_version` = `standard_raw_gap_3;same-price/keep-earlier+extreme-region/1`，规则版本留在 `semantic_version`。票面「并列记 RULE 与 SAME_PRICE_POLICY」读作同一 payload 并列，保留 raw 间隔门。
- 发布合同升 `s-new-bi/2`（照 `s-segment/2` 先例）；`change_event.before` 按自身代际形状校验。会话级 `RULE_REVISION` 随之升 `new-bi/2`，旧规则会话只读（accept 即 VersionMismatch），同一 rule_revision 下不混两代对象。
- IMPL-TICKETS 列的 hand-oracle「`mirror_gap2_raw3`」是标签笔误：按行号 240 实为 `mirror_tie_top`，本票按行号改。
- 根集为空的不变式：`extreme_region` 先 `debug_assert!`，任何构建都 panic（fail-loud），不回退。时间先后两端点的极值区不相交由严格递增坐标域保证；裸调用方违反坐标域时 `raw_between` 按 0 计（条件 2 判 false），不另开等待态。

## 红项分诊（不在本票修）

- `l3_delta_r_alpha::m3_partition_synthetic_holds`：合成鞅 3000 根现在得 merged 1,998 / 分型 1,023 / 笔 293 / 段 24 / 塔 [24, 5]，但 ledger 与 records 仍为 0。空转发生在笔层之后的交易/候选层，非同价域。
- `tick::calibration_sweep_finds_non_collapsed_segments`：笔中位尺寸在 gap 5、10 都是 $0.01（gap 100 才到 $8.09）。用诊断 port 复算，裁定规则、keep+最早根、keep+最晚根三者在 gap 5/10 的中位同为 $0.01（replace 两个反事实只有 66 笔）。与同价取法无关，是 #973 定标时的笔实现前提不再成立。

## ④ TB-02-A 同价门退役（根裁定 B）

同一个「同价 = 未定」前提在 TB-02-A 有三处：CC-004 步骤请求、CC-005 组请求把 `equal_extreme_identity` 计成 DOMAIN_PROOF_MISSING/INCOMPLETE；`s_structure_session` 的发布门让任一组多根的三组窗口不发 CC-006/CC-007，另挂 subject=null 的窗口请求。#1343 Q2 后这三处与 TB-02-B 对同一分型一宽一严。现在多根窗口照常发布，两类请求理由为空、COMPLETE/IN_DOMAIN，`equal_extreme_identity` 只作证据标签留在步骤/组载荷（字段名 `waiting_reasons` 沿用旧名），知识域升 `established_direction_with_extreme_root_sets`（旧值只在旧规则会话里可读）。

catalog evidence 计数照实变化（real_C 27 根，终 cut）：CC-006 local_shape 10 → 17、CC-007 描述 6 → 10、`classified_objects` 115 → 126、`windows_total` 17 不变、subject=null 请求 7 → 0；最小例中 CC-006 轴 `run_status` 由 `waiting` 变 `run`。TB-02-A 手算账簿 `wait_equal_high/low/both` 三例的 v1 期望在新二进制下按预期失败（「知识状态必须保留缺口」），v2 `check_root_set` 经 S CLI 冒烟三例 PASS（`receipts/tb02a-root-set-smoke.json`）。TB-02-A 的 20 次正式双跑未重跑。

## 冲突点与合入前义务

合入次序已定：#1394 先合，#1405/#1419/#1420 后合，后合者 rebase。

| 文件 | 本票改动 | 他票 |
|---|---|---|
| `rust/src/bin/s_structure_session.rs:63` | `RULE_REVISION` → `…+new-bi/2+segment-second/1` | #1420 → `…+segment-second/2`；#1419 → `…+center-seed/1`；#1394 改 :60 附近。后合者合成三段组合值并重生成受影响夹具 |
| `s_structure_session.rs` 形状循环（原 :2234-2244 门） | 删门 | #1394 在 2235+7 有改动；#1420 在 2157+8 |
| `s_structure_session.rs` 测试（原 :4802 起） | 改名并按新行为断言 | #1419/#1420 在 4360 附近，#1394 未及 |
| `s_structure_session.rs` `large_predecessor…`（:6898 起） | 按集合核对 supersedes | #1419 逐字相同；#1420 同款（措辞待 rebase 时核） |
| `rust/src/bin/s_session_v2/tb02a_facts.rs` | 步骤/组请求、删窗口请求、知识域 | #1394 在 126+52 / 213+7 / 357+26 大改；#1419 在 375–497 |
| `s_session/s_tb02_contract.py` | `RULE_CONTRACTS`、新笔 v1/v2、知识域 | #1420 段合同多处；#1419 在 17/130/321 附近。`RULE_CONTRACTS` 须补 #1420 的段合同代际与 #1419 的组合值 |
| `s_session/browser/tb01c-client.js` | `validateBiPayload`、`RULE_CONTRACTS`、知识域 | #1394 重写为 async（另有 `q3-typed.js` 内嵌同一份 TB-02-B 校验）；#1420 段合同 |
| `s_session/s_query_integrity.py:347` | 按规则发布轴补新键 | #1419/#1420 同处 |
| `s_session/tests/tb02b_harness.py`/`tb02b_verify.py` | 规则串、新字段 | #1394 同段改动 |
| `rust/src/theta_v0/parser/mod.rs` | :166 注释、:611 测试 | #1420 有改动但不同 hunk |

合入前义务：rebase 到 #1394 之后把 v2 新笔校验与知识域两代移植进 `q3-typed.js`；`RULE_CONTRACTS`（py/js）与 `_published_axes` 合入最终组合规则串；按根要求重生成受规则串影响的夹具；**bsp 白名单再重锚一次**——#1394 整合分支（`origin/codex/1394-integration-20261013` @ `0c6f73b0d5`）已把 `tb02a_facts.rs` 这组锚收成 `:564` 一处，rebase 时以它为准、丢弃本票 F 片的三行，按 rebase 后实报行号核 `no_stray_bsp_binding_rewrites_outside_registry` 转绿。

## 陈旧但未改（登记）

- `s_session/profiles/ohlc_integer_tb02a_v1.json` 的 `note`「同价极值身份保留等待；不裁初始化或tie政策」：#1343 后已陈旧，但该文件受 `profile_hash` 绑定，不动；以本票为准。
- `s_session/catalog/signed-catalog.json` 的 CC-009 规则文字「EQ价格唯一但raw端点身份tie需单独说明选择证据」：签名目录，不动；本票以 `same_kind_equal_price_keep_earlier` 证据回应。
- `parser/inclusion.rs:729-733` 的 `equal_extreme_identity`：按票面保留为证据；`tb02a_facts.rs` 与 `s_structure_session.rs` 审计后均不再作准入门。
- Lean：`formal/Origin/ChanlunElements.lean:29-40,130-137` 无同型归约与 raw 域条件 2 的定义式，跟进义务照 #1343 登记，不在本票。族 I（`src/newchan/a_stroke.py`、`rust/src/stroke.rs`）未改。

## 限制

- 尾 5 万根读数锁是 `#[ignore]` 外部数据测试（数据 3.5 MB 不入仓），CI 不跑；入仓的常跑锁是六见证、选择无关、单根同值、双进程哈希与 tb02b 账簿。
- ~~浏览器只跑了根批的一次（real_C 27 根，D 片之前的二进制）；D 片的 TB-02-A 改动经正式会话 + HTTP 公共查询（含 JS 客户端校验）验证，未再起浏览器。~~ **订正（2026-10-20，#1405 关票门补记）**：恢复部分（PR #1571）在终版二进制（`c4bbdd9f8a` 构建，sha256 `08dfc651…7bcb`，规则 `new-bi/2` 组合串）上重跑了 real_C 27 根：正式会话检查 PASS，真 Chromium（Playwright headless shell 156，Linux）页面检查 PASS、0 page error、应显字段零缺失、退役等待态零出现；runtime 总判记 FAIL，只因收尾 `stop-s` 超时，不影响页面判定。收据与边界见 `recovery-20261019/RECOVERY-DELIVERY.md`。macOS Chrome 应用未跑。
- 独立评审 INFO（历史，已由上条终版复核替代）：根批那一次浏览器跑在 B 片 `1f90227633` 的二进制上，会话规则仍是 `s2-axis-quantifiers+new-bi/1+segment-second/1`，对象却是 `s-new-bi/2`；终版 `RULE_CONTRACTS` 绑定会拒这个组合（G 片的 `new-bi/1 会话 cut 里出现 s-new-bi/2 对象被拒` 正锁此例）。页面字段可见性证据（组锚、极值根集、`raw_first`/`raw_last`、两段 policy、KEEP 证据、退役等待原因零出现）仍有效；终版合同状态由 r3 轮（`93e9097915` 工作区，规则 new-bi/2，正式会话 + HTTP + JS 客户端校验）承担。
- 未开 PR、未跑 CI；DevSkim 实际增量待 CI SARIF。

# #1414 / PR #1477 CI 与 DevSkim 增量核验

固定 PR head `cea15bd735bc`（CI run `38397240254`、DevSkim run `38397240180`），对比 main `f95a7f3ebc0c`（CI run `38201220739`、DevSkim run `38201220747`）与上一版提交面 `787f5cf1fe84`（CI run `38344720471`、DevSkim run `38344720431`）；中间两个提交 `aa4039cc`（run `38385269425`）与 `179412c3`（run `38387197713`）的 rust-check 一并登记。只读取 GitHub check-runs、作业日志与已下载的 `devskim-sarif` 工件，不重跑扫描器或测试，不修改产品。逐项核验由增量整体独审（`/root/tb1394_r6_contract`，`PASS_INCREMENTAL_SURFACE_READY_FOR_MERGE_REQUEST`）完成，本文件登记其 IR-M1、IR-M2 两条要求随合入请求附带的事实。

## rust-check

五次运行的 `rust-check` 都在「cargo test — default targets」步失败，其后的 s_session bin 全量与 `--lib --features backtest_bin` 两步未执行；`test`、`s-observer`、`fixture-drift` 在这五个提交上都成功。

| 提交 | CI run | default targets 读数 | `incremental_tower_scaling_dominates_full_synthetic` |
|---|---|---|---|
| `cea15bd7`（C5 报告） | 38397240254 | 2955 passed / 3 failed / 161 ignored | ok |
| `179412c3`（P2 测试） | 38387197713 | 2954 passed / 4 failed / 161 ignored | FAILED：增量/全量比 @n=400 = 0.723，门 < 0.7 |
| `aa4039cc`（P2 修正） | 38385269425 | 2955 passed / 3 failed / 161 ignored | ok |
| `787f5cf1`（上一版提交面） | 38344720471 | 2955 passed / 3 failed / 161 ignored | ok |
| main `f95a7f3e` | 38201220739 | 2950 passed / 3 failed / 161 ignored | ok |

五次运行共有的 3 个失败恰是获批基线例外的三项：`theta_v0::backtest::tick::tests::seam_btc_tick_bars_yield_non_trivial_structure`、`theta_v0::backtest::tick::tests::calibration_sweep_finds_non_collapsed_segments`、`theta_v0::backtest::l3_delta_r_alpha::tests::m3_partition_synthetic_holds`。它们在 main 上同红，登记口径沿用 [#1392 关票评论](https://github.com/xy7365527-lang/NewChanlun/issues/1392#issuecomment-6027480656) 与 [#1404 关票评论](https://github.com/xy7365527-lang/NewChanlun/issues/1404#issuecomment-6117508188)，后续归 #1343/#1405；完整证据链见 `ACCEPTANCE-REPORT.md` §12。同一次运行的 `s-observer` 在 CPython 3.14.6 下打印解析池布局匹配 `True`，TB-02-D 五模块 Ran 40 OK。因为 bin 全量一步被跳过，`cea15bd7` 字节上 s_session bin 的全量读数只有增量整体独审在本机那次：81 passed、0 failed、1 ignored。

`179412c3` 上多出的红判为计时抖动，不是 #1414 引入。该测试（`rust/src/theta_v0/classifier/tests/incremental_parity.rs:1422-1484`）在 debug 构建下对 n=100/200/400 的合成输入测墙钟时间，`:1481` 断言 n=400 时增量/全量比小于 0.7；那次读数 full_times=[0.0228, 0.0552, 0.1585] 秒、inc_times=[0.0078, 0.0154, 0.1145] 秒，比值 0.723。判定依据有三条：其一，库字节在 `787f5cf1`、`aa4039cc`、`179412c3`、`cea15bd7` 四个提交上逐字节相同，P2 在 `rust/` 下只改了 `src/bin/s_session_v2/tb02c.rs` 一行；其二，该测试直接构造带合成段的 `ParseLayer` 调 `classify`/`classify_incremental`，不走本 PR 改过的分段与特征序列代码，`git diff f95a7f3e cea15bd7 -- rust/src/theta_v0/classifier` 为空，classifier 目录不引用 `FeatureSeqState`；其三，同一份库字节在 `aa4039cc` 与 `cea15bd7` 上通过，main 与 `787f5cf1` 上也通过。0.7 门不放宽；该计时测试的抖动建议另开票跟踪，本提交不开票。

报告提交 `cea15bd7` 的 committer 时间是 12:24:59Z，晚于 `179412c3` 的 rust-check 完成时间 11:07:51Z，上一版报告仍写「预计 3 个既有红」与「CI 实际结论 PENDING」，漏登了这次多出的红（IR-M1）；`cea15bd7` 自身的 rust-check 在 12:33:30Z 完成。

## DevSkim

三次运行的扫描器均为 `devskim 1.0.90+fb2d676ce4`。SARIF 结果数：main 41,730 条，`787f5cf1` 42,552 条，`cea15bd7` 42,903 条。按 `scripts/devskim_sarif_gate.sh` 原样对 `.github/devskim-baseline.json`（8201 键，`df25ff2ec9a0`，本 PR 未改）跑门，基线外告警为 main 33,443、`787f5cf1` 34,265、`cea15bd7` 34,616，三者「Fixed since baseline」都是 14。门在 main 上本就红，PR 归因只能看相对 main 实测 SARIF 的差集。

门口径（规则|路径|行）下，`cea15bd7` 相对 main 新增 1176 键、消失 3 键，相对 `787f5cf1` 新增 600 键、消失 249 键。按 #1404 先例的多重集口径（忽略行列号）比较，相对 main 新增 1173 条、消失 0 条；相对 `787f5cf1` 新增 393 条、消失 42 条，增删全部是 DS173237；`787f5cf1` 相对 main 为新增 822 条、消失 0 条。

新增全部是密钥规则误报、路径名误报或 loopback/固定回调提示，需修 0 条。下表是 `cea15bd7` 相对 main 的多重集新增：

| 规则/级别 | 路径 | 新增次数 | 性质 |
|---|---|---:|---|
| DS173237/error | `.chanlun/review-results/issue1414-tb02d-20261012/run-package/CURRENT-SOURCES.json` | 547 | 运行来源集「路径 → SHA-256」映射中的证据指纹；密钥规则误报 |
| DS173237/error | `.chanlun/review-results/issue1414-tb02d-20261012/EVIDENCE-INDEX.json` | 531 | 收据 SHA-256 与本仓提交号；密钥规则误报 |
| DS173237/error | `.chanlun/review-results/issue1414-tb02d-20261012/run-package/PLAN.json` | 56 | 运行计划绑定的 SHA-256；密钥规则误报 |
| DS173237/error | `.chanlun/review-results/issue1414-tb02d-20261012/run-package/ROOT-P2-RERUN-AUTHORIZATION.json` | 12 | 根授权绑定的 SHA-256 与提交号；密钥规则误报 |
| DS173237/error | `.chanlun/review-results/issue1414-tb02d-20261012/run-package/REBUILD-RECEIPT.json` | 11 | 前门重建收据的 SHA-256 与提交号；密钥规则误报 |
| DS117838/error | `.chanlun/review-results/issue1414-tb02d-20261012/run-package/CURRENT-SOURCES.json` | 5 | 路径名含 key（`key.rs`、`key_truth.rs`、`key_reuse.py`、`two_pass_evidence.rs`）的「路径 → SHA-256」行；规则误报 |
| DS173237/error | `s_session/tests/fixtures/tb02d/MANIFEST.json` | 4 | 夹具清单中的 SHA-256；密钥规则误报 |
| DS162092/note | `.chanlun/review-results/issue1414-tb02d-20261012/run-package/remaining-driver.py` | 2 | 一次性驱动副本中本机 `127.0.0.1` 的 `connect_ex`/`bind`；loopback 提示 |
| DS197836/error | `.chanlun/review-results/issue1414-tb02d-20261012/run-package/remaining-driver.py` | 1 | `runplan['source_sha256'] == expected_runtime`，把 `runtime` 中的 time 当作时间值；实为比较源集合 SHA，规则误报 |
| DS172411/note | `s_session/browser/tb01c-client.js` | 1 | `setTimeout(() => abort.abort(), …)`，回调是固定箭头函数；提示 |
| DS173237/error | `s_session/tests/fixtures/tb02d/generate-input.py` | 1 | 生成器中的 SHA-256；密钥规则误报 |
| DS173237/error | `s_session/tests/fixtures/tb02d/hand-oracle.json` | 1 | 手算 oracle 中的 SHA-256；密钥规则误报 |
| DS162092/note | `s_session/tests/tb02d_runtime.py` | 1 | `:98` 本机验收脚本传给浏览器 runner 的 `http://127.0.0.1:` 地址；loopback 提示 |

相对 `787f5cf1` 的增量里，新增 393 条为 `EVIDENCE-INDEX.json` 306、`run-package/PLAN.json` 56、`run-package/ROOT-P2-RERUN-AUTHORIZATION.json` 12、`run-package/REBUILD-RECEIPT.json` 11、`run-package/CURRENT-SOURCES.json` 8；消失 42 条为 `EVIDENCE-INDEX.json` 25、`run-package/CURRENT-SOURCES.json` 11、`run-package/explicit_composite.py` 4、`run-package/reuse_state.py` 2，后两件随 run-package 替换移出。

DS173237 命中逐键取 `cea15bd7` 与 `787f5cf1` 源行核对：相对 `787f5cf1` 增减的 595+244 个键与相对 main 新增的 1164 个键，命中值全部是 64 位 SHA-256 或 40 位十六进制；40 位的 10 个不同值逐个 `git cat-file` 为本仓对象（9 个 commit、1 个 blob）。所在字段只有 sha256 类、commit/head/base 类与「文件路径 → SHA」映射，没有 token、secret、password 或 key 类字段。未发现新增实际凭据或新的安全风险；此结论只覆盖上述增量。

非 DS173237 项在 `cea15bd7` 与 `787f5cf1` 之间只是行号迁移，多重集口径下增删都是 0：`remaining-driver.py` 的 DS162092 从 101、156 行移到 185、241 行，DS197836 从 170 行移到 255 行，成因是运行包驱动整体替换（`595fa4c00937` → `ab80440cdebd`）；`tb01c-client.js` 的 DS172411 从 647、742 行移到 652、747 行，成因是 `aa4039cc` 在 `:244` 后加 1 行、`:286` 后加 4 行。main 已有的三条只随 PR 下移：`s_session/s_readonly_server.py` 的 DS162092（`--host` 默认 `127.0.0.1`）1193 → 1234，`tb01c-client.js` 的 DS172411 583 → 747 与 DS173237 118 → 146。

DS162092 指向的都是本机验收面：`remaining-driver.py` 是 run-package 里绑定 #1414 的证据副本，不被任何构建、测试或运行入口调用；`tb02d_runtime.py` 是正式验收入口，loopback 是既定隔离范围。本增量不含生产联网地址或调试服务外放。

按 #1404 先例以本文件登记增量，未改 `.github/devskim-baseline.json`。若日后改为登记基线，基线键含行号，`EVIDENCE-INDEX.json` 与 run-package 每改一次就整体迁移（上表 170 → 255 即一例），只能在最终 head 上生成。

## 收据

完整输入是三次运行的 `devskim-sarif` 工件（`gh run download` 取得），原件不入仓，存于控制目录 `CTRL-20261012:tb1414-incremental-surface-review/devskim/`：

| head | DevSkim run | SARIF SHA-256 | bytes |
|---|---|---|---:|
| `cea15bd7` | 38397240180 | `45087a1f845c03c59d5b2d57f1f40a0ea5a040ef6e7791835b569498cb5e74f3` | 136917470 |
| `787f5cf1` | 38344720431 | `61910412d7b2a71604231a6773edc4fc811810613fd7c7ca559199f991a41ac8` | 136654664 |
| main `f95a7f3e` | 38201220747 | `a96283895b71d031499eccb8c20a3a2591255943f169335fff1ebc5fb1a44455` | 136032044 |

逐项核验工作原件与 CI 读数在同一控制目录：

| 文件（`CTRL-20261012:tb1414-incremental-surface-review/` 下） | SHA-256 | bytes |
|---|---|---:|
| `REVIEW.md`（增量整体独审） | `c6b0adf14e108d95b17916dd95e85d9ab99f4ec0f5c1bde813faa8d8d6520c5a` | 13202 |
| `RESULT.json` | `8d8d2a800b60eeb4970628581ef397021b2a6e7c0fa08da8bf6981cb2501c917` | 8854 |
| `BINDING.json` | `1b82b5dc3acd57385165289b32846bc78a7434e772497fa96f9a74c82ea0b2f8` | 9800 |
| `ci/CI-RUNS.json` | `c8caa3c022624079c85988ffc69e82f8bf238f767511744277c95c3feb9508d8` | 3411 |
| `ci/rust-check-cea15bd7-run38397240254-job115295473313.log` | `74c95167258f8589ee9b97aadb81b69f21482ac27fa925c872af4a1b331cb653` | 899106 |
| `ci/rust-check-179412c3-run38387197713-job115261791575.log` | `af93267c684e447ef98e988a54b01bc66058ece7664b79353495875ae2ba5ec5` | 898574 |
| `ci/rust-check-aa4039cc-run38385269425-job115255377178.log` | `ee4e2c6b53788b0a00a5ae667b88887bb50a9b6d4674214491f18cfb04f95b04` | 892637 |
| `ci/s-observer-cea15bd7-run38397240254-job115295473111.log` | `473d8b98b88efbf2ca6ee55ba57ec38081b3f9e2fd2de07b495674427ca0e86b` | 63516 |
| `devskim/DEVSKIM-DELTA.json` | `c948d9ee1c1dd8c27a42d0ac6994f30ed80af33d0dc64f8d06973ec46dc9dc0f` | 13775 |
| `devskim/DEVSKIM-KEYS.tsv`（逐键清单与判定，2028 行） | `85d0f5c473972c14f8b2983e25076e70c8ac3d71f7f82bfc95aa5f2ede826b25` | 568435 |

`787f5cf1` 与 main 的 rust-check 日志是上一轮提交时取得的，在 `CTRL-20261012:tb1414-commit-pr/` 下：`ci-rust-check-115123329353.log`（`653b73fc91a2…`，1587707 B）与 `ci-main-f95a7f3-rust-check-114654853529.log`（`b5b88d23b969…`，1154100 B）。此处是入仓结论报告，固定 `cea15bd7` head，不冒充后续提交的新增扫描结果；本文件与同次改动的报告、索引新增的 SHA 字符串会在下一次 DevSkim 运行中产生新的 DS173237 命中，不在本结论内。

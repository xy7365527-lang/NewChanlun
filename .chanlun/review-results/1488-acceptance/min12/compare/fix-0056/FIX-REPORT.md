# #1488 S2 跟进 0056（按独审 0055）

- 工位 `accept-s2-checkout`，分支 `codex/1488-accept-s2-20261020`，起点 `b6a5a3e46c`，只追加、未 push。
- 对象目录：`.chanlun/review-results/1488-acceptance/min12/`（下称 `min12/`）。本文件的副本入仓为 `min12/compare/fix-0056/FIX-REPORT.md`。
- **未重跑全量 oracle**：oracle 文件、`runs/`、`oracle/` 与清单均未改动。F8 的判据（104/104 摘要一致）已由既有产物满足；F9 的逻辑比对在 G: 原件副本上离线完成。两者都不需要新跑。

## 逐条处理

| # | 处理 | 证据 |
|---|---|---|
| F1 | `tools/semdiff.py` 重写为 v2（`semdiff/2`）：不剥版本字段，不改写 raw_position，保留空容器叶子；按身份键多重配对（不折叠），再去 fact_key 二轮配对（槽位 TB-02-D→TB-02-W）；比较 fixed_cut/catalog/counts/scope/omissions/order_version。REPORT §3 重写为 §3.1 行级 R1–R19 与 §3.2 顶层 T1–T8，逐项归入 new-bi/2、segment-second/2、无 center-seed/1 三类，意外差异仍为 0 | `min12/compare/semdiff2-known15.json` `2ef5d95f858fdf16…`、`semdiff2-known31.json` `2ec7f79fd74e1160…`（用当前 semdiff.py 重算逐字节一致）；`compare/fix-0056/check_toplevel.py` → `toplevel-known{15,31}.json`；`check_raw.py` → `raw-coords.json`（k15 62/62、k31 150/150 多重集相等，raw_first==raw_last 全部成立） |
| F2 | REPORT §0、§4 记录主控 A③ 裁定原文（2026-10-20T05:18:17Z 评论链接）。初版选项改为 ①②③，并写明映射：裁定 “(a)” = ②（中心半句移交 #1419），被否决的 “(b)” = ①（先合 #1419） | REPORT §4 |
| F3 | §0、§5 改为“本次两跑未观测到台阶；宿主噪声下不可判”。补上噪声证据：run-a op-0041→0042 5267→9322 ms，run-b 11 条 >10 s。写明 10 s 帽证据须由 S6 在安静宿主上取得 | `runs/run-{a,b}/INGEST.jsonl` 的 reply_ms |
| F4 | §6 第 4 条更正原因：是 `immutable=1` 忽略 WAL，不是“按增量对计数”。`tools/r3_offline_churn.py` 改为有非空 WAL 时先在临时副本上 checkpoint，并输出 `wal_applied`。在 G: run-a 上重跑得 52 代：g1..51 与初版相同，g46/47 为 4574098→4924102、37/21，g52 为 6019116；generations 与 0055 的 e726930e… 完全相同。原库 sha 前后不变 | `min12/compare/r3-churn-main-run-a-wal.json` `b08ced9273c1798a…` |
| F5 | §2.4 订正：`seed_start_unit_id segment:1:1 / awaiting_offset 2` 位于 `CC-056.center_version.payload.data.scan_cursor`，不在 CC-022 | 本地对 known-31 全字段扫描，只在 CC-056.center_version 命中 |
| F6 | `scan_cursors: []` 并入 §3.1 R14 | semdiff2 `payload.data.scan_cursors` |
| F7 | `tools/run_wsl.sh`：驱动改为 `\|\| rc=$?`；无论成败都拷回 G:，写 `DRIVER-EXIT` 和 `SHA256SUMS`，最后 `exit $rc` | WSL 假 python3.12（exit 7）实测：脚本 exit 7，G: 上有 RUN.json、DRIVER-EXIT=7、SHA256SUMS；测试目录已清理 |
| F8 | `tools/min12_oracle_run.py`：`digest_match=false` 时直接 raise。RUN.json 记 `error`、`stop_stdout`/`stop_stderr`（末 4000 字符）。`ok = ingest ∧ queries ∧ stop_exit==0 ∧ checkpoint_ok`，不成立时退出 1 | 本机假 launcher/假 Q 实测：摘要不符时退出 1，`ok=false`，`stop_stderr` 留存。按新口径，既有 run-b（stop_exit=1）会被判失败，已在 REPORT §6 第 1 条写明 |
| F9 | 新增 `tools/db_logical_digest.py`，输出主库文件 sha、WAL 字节、checkpoint 后 sha、iterdump 逻辑摘要和表行数。驱动在服务停止后对原库执行 checkpoint，并记录 `db_digest`。REPORT §0/§2.3 改为“主库文件 + 逻辑内容相同” | `min12/compare/fix-0056/db-digest-run-{a,b}.json`：两跑 checkpointed `f83b5add5ead…d107`（与 0055 一致）、logical `b585f5ea100d…5b79`、表行数相同。疑点：checkpoint 返回 log_frames=0，但 sha 确已变化且与审阅者一致，原因未追查 |
| F10 | 在 WSL 用 nc-remote 按 SHA 冷重建：新 worktree、全新 `CARGO_TARGET_DIR`、`--locked`，porcelain 为空，rustc/cargo 1.99.1。产物 sha `c944bd6b…e040` 与 S2 原件、S1 独立构建三者一致。驱动 RUN.json 今后记录 `repo_status_porcelain` | `min12/compare/fix-0056/f10-rebuild.log`、`f10-s1-build.log`、`f10-s2-original-build.log`（原构建 EXIT=1 来自尾随的 `ldd \| grep python` 无匹配）。重建件在 `G:/AgentStorage/1488-accept/s2-fix-0056/rebuild/` |

## 新增/改动文件 sha256（提交时）

| 文件 | sha256 |
|---|---|
| `min12/tools/semdiff.py` | `0b7322d5600d2fd7b335cd38d397280a37513ec9f7f299f788f8b18d9f4ca4ef` |
| `min12/tools/db_logical_digest.py` | `45033aafcbabc9736c15a235ee8aa8946502a97f0743c8c0e44154650120ac9d` |
| `min12/tools/r3_offline_churn.py` | `bbca0f769df73c44be4f1c48ddf6b81ca58ed77b935468fc0e0e704abd122fd4` |
| `min12/tools/min12_oracle_run.py` | `2c666b8eb10849e55c23ee0d17f9819ed2784db5934ad12738225af27b73c374` |
| `min12/tools/run_wsl.sh` | `e4d495b15bb299024ed03fe8712e83c3dae63df82c2aa9b7300551a45706222d` |
| `min12/compare/semdiff2-known15.json` | `2ef5d95f858fdf1635da9d4b911642b4d75fe2722783d27669a84c388db1a3fd` |
| `min12/compare/semdiff2-known31.json` | `2ec7f79fd74e11607ccfb9b972747cfa233fd4d96b983f5f5ad97261719cf606` |
| `min12/compare/r3-churn-main-run-a-wal.json` | `b08ced9273c1798a9dc07fa5590af8ba2c7f8e42e3d792a960bbf45e2d993def` |

semdiff 的输入是 /tmp/rv0055 下的本地副本，sha 已与 G:/ORACLE 核对一致：k15 `9c252834…`、k31 `36ee2da0…`、g-0016 `74d8df32…`、g-0032 `383df8bc…`。

## 留给主控

- #1488 的 S2 评论（05:18:55Z）写了“第 46 条台阶在 main 上不复现”，应按 F3 降为“未观测，宿主噪声下不可判，帽证据由 S6 在安静宿主上取得”。本工位不发评论。
- S6 取证前提：REPORT §5。

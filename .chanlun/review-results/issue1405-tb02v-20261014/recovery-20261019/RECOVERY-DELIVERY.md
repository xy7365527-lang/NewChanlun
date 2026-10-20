# #1405 恢复部分交付补记（PR #1571，验收 4c 与验收 3 的终版二进制复核）

> 补记会话 `/root/close_1405_gaps`（主控 `/root`），2026-10-20。原实施会话 `/root/owner_1405_recovery`，分支 `codex/1405-recovery-v4-onmain-20261019`，HEAD `1870e95895`，基于 main `5baeb030a6`；PR #1571 合入为 `b34b0278ee`。本补记只把当时留在仓外的收据入仓并照实写明结论边界，不重跑、不改判据。仓外原件：`/Volumes/AgentStorage/issue1405-rv4-20261019/`（测试日志、`mutant.log`、398 MB 浏览器原包）与 `/tmp/o1405/`（运行目录）。缺口来源：#1405 issuecomment-6277278144。

## 结论

所改持久边界（旧规则 S 库只读、热回滚日志拒开、恢复入口版本门且不建库）由 5 个 `tb1405_*` 测试锁住，`1870e95895` 上 `cargo test --features s_session --bin s_structure_session` 86 过 / 0 败 / 1 忽略；变异对照（去掉只读判规则）使 hot-wal、hot-journal 两例转红，证明测试会咬。终版二进制（`c4bbdd9f8a` 构建）上 real_C 27 根走正式 launcher + socket 接纳 + HTTP 公共查询，session 检查 PASS；真 Chromium（headless shell 156）页面检查 PASS，0 page error，应显字段无缺失、退役等待态零出现。runtime 自身的 RESULT 记为 FAIL，查明只来自收尾的 `stop-s` 超时，不影响验收 3 的判定，见下文「runtime FAIL 的成因与影响」。

## 收据清单（本目录）

| 文件 | 内容 |
|---|---|
| `test-s_session-bin-1870e95895.txt` | `1870e95895` 全量 s_session bin 测试（WSL Linux，`nc-remote`），86/0/1；原名 `20261019-230224-1870e95895.log`，sha256 `4b548d50…ecd62`，逐字节相同 |
| `mutant-be4268a1af.txt` | 变异对照：`be4268a1af`（父 `c4bbdd9f8a`，仅把 `open_db` 的 `if db.exists()` 改为 `if false && db.exists()`）跑 `tb1405` 过滤集 2 过 2 败；原名 `mutant.log`，sha256 `64808b91…a879` |
| `browser.sh` | 浏览器复核所用脚本（构建、prepare、session、browser、check-browser、回收核对、打包） |
| `browser-225737/` | 采用的那次复核（ID `20261019-225737`）：`RCS.txt`、`TB02V-SESSION-CHECK.json`、`TB02V-BROWSER-CHECK.json`、`PLAN.json`、`config-browser.json`、三个 stdout；`session-run/RESULT.json`；`browser-run/RESULT.json`、`query-oracle.json`、`browser-command.json`、`browser-process.json`、`control-events.jsonl`（从 `events.jsonl` 摘出的全部 control 事件，含 `stop-s` 原始返回）；`browser-run/browser/` 的 `RESULT.json`、CC-008/009/010/056 可见文本、`final.png` |
| `browser-225314-failed/` | 前一次未成的尝试（见下），`RCS.txt`、`check.stdout`、`RESULT.json`、`browser.stderr` |

未入仓：398 MB 原包 `tb1405-browser-20261019-225737.tgz`（sha256 `4e00c594…dee53`，仍在仓外原处）、`semantic-core.jsonl`（390 MB）、`network.har`（8 MB）、`source-before-run/`、sqlite 库及调试残留（`/tmp/o1405/exp*`、`e3/`、`dbg.sh`）。

## 测试与变异对照

5 个新测试：hot `-wal` 现场下 6 种规则 × 读/写/恢复入口全过且主库与 `-wal` 字节不变；hot `-journal` 现场（断言日志头 magic）全入口拒绝且字节不变；已正常关闭的旧库主库字节不变；当前规则库正常关闭重开与崩溃现场重开后 catalog/snapshot/meta 逐位一致且之后可 recover/accept/advance；recover 不建缺失的库。收据见 `test-s_session-bin-1870e95895.txt` 末段五行 `tb1405_* ... ok` 与 `test result: ok. 86 passed; 0 failed; 1 ignored`。

变异对照跑在 `c4bbdd9f8a` 之上（当时只有前 4 个 tb1405 测试，干净关闭例是 `1870e95895` 才补的），结果 `tb1405_old_rule_store_with_hot_wal_is_never_rewritten` 报「历史库连接须在 SQLite 层拒写」、`…hot_rollback_journal_is_refused_unchanged` 报「热日志现场不得打开…journal_now=false」（热日志已被回滚删除）。另两例（当前规则库重开、recover 不建库）不依赖只读探测，按设计不受该变异影响。

## 终版二进制上的浏览器复核

二进制：WSL 上从 `c4bbdd9f8a` 构建的 release `s_structure_session`，sha256 `08dfc651…7bcb`（`RCS.txt`）。`1870e95895` 相对 `c4bbdd9f8a` 只改测试、一处注释和 `open_historical_readonly` 热日志拒绝分支的报错文案（`git diff c4bbdd9f8a 1870e95895`，+34/−2，单文件）；该分支在 real_C 运行中不可达（新建库，无热日志），所以这份二进制对终版运行时行为有代表性。

- session（`Tb02aRun`，正式 launcher + socket 接纳 + HTTP live/AsKnown/重访）：`session-run/RESULT.json` 状态 `CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER`，cleanup 零错误；`TB02V-SESSION-CHECK.json` PASS，27 cut，末笔延伸替代链与首笔不被同价弹出均核过。
- browser（`tb02b_runtime.py`，另起一套 S/Q）：`browser-run/browser/RESULT.json` `status: passed`、`page_errors: []`；`TB02V-BROWSER-CHECK.json` `status: PASS`，CC-008/009/010/056 应显字段零缺失、退役等待态零出现，四轴可见文本 sha256 已记录。`query-oracle.json`（真实库 27 cut 前端合同定点验证）passed。
- 边界：浏览器是 Playwright 自带 Chromium headless shell 156.0.8078.4（Linux），不是 macOS 上的 Chrome 应用；macOS fullfsync 路径与原生 Windows 未跑。

前一次尝试 `20261019-225314` 未成：浏览器启动失败（`browser.stderr`：默认 `chromium-1248/chrome-linux64/chrome` 不存在），故 check-browser 读不到 `browser/RESULT.json`。脚本加 `TB02B_CHROMIUM_EXECUTABLE` 指向 headless shell 后重跑得到 `225737`。该次 session 部分同样 PASS。

## runtime FAIL 的成因与影响

`browser-run/RESULT.json` 的 `status: FAIL` 只来自收尾。依据是 `tests/tb02a_runtime.py` 的 `execute()`：主体（启动、27 次输入、live/AsKnown/重访查询、`collect_core`——在 `tb02b_runtime.py` 中它先跑 cursor 检查、`query-oracle`、真浏览器，浏览器子进程返回码非 0 即抛错——再落 semantic core 与源码 hash 复核）全部成功才把状态置为 `CAPTURED_…`；`finally` 里 `cleanup()` 有任何错误再把状态改成 `FAIL`。本次 `RESULT.json` 无 `error` 字段且有 `core_records: 154`，说明主体走完、浏览器返回 0；唯一错误是 `cleanup.errors = [{"action": "stop-s", ...}]`。

`control-events.jsonl` 末条记录：`stop-s` 退出码 1，stderr「s 尚未退出，不自动升级信号」，耗时 10.08 s。对应 `s_session/s_service_control.py` 的 `stop()`：对经身份复核的 S 发一次 SIGTERM，在 `startup_timeout_ms`（本配置 10000）内轮询，超时即抛此错，按设计不升级到 SIGKILL。脚本随后 `sleep 3` 复查：无 `s_structure_session`/chrome 残留进程，18960/18961 端口空闲（`RCS.txt`）。收据只能证明「10 s 观察窗内未观察到 S 退出、3 s 后已无残留」，S 实际何时退出、是否慢退出均未证实。

对验收 3 的影响：浏览器与页面判定在收尾之前已经完成并落盘，`check-browser` 只读这些原件，与收尾无关；session 检查来自另一次运行，其 cleanup 零错误。所以 `stop-s` 超时不改变验收 3 的 PASS。上一次尝试的浏览器运行里 `stop-q` 报了同样的错（`browser-225314-failed/RESULT.json`），同一 S 二进制在 session-run 收尾 `stop-s` 零错误，所以问题更像出在观察侧而非 S 本身：S 未安装 SIGTERM 处理（默认处置立即终止，SQLite 关闭/检查点不运行），而 `s_service_control.py` 的 `stop()` 会把轮询期间 `ps` 查询失败吞为 `unknown` 继续等，超时后只报「尚未退出」、不带观察记录（独立评审 WARN-2）。根因未查，归后续，不在本票改判据。runtime 总判照原样记 FAIL。

## 评审

#1571 原「非作者 Opus 5.5 可合」评审无原件，且在 `1870e95895` 之前。本次由 claude-opus-5-5 非作者对 `5baeb030a6..1870e95895` 全量补评（后台 CLI，经本机 CLIProxyLocal；只读检出 `299485822f`），报告 `REVIEW-1571-full-opus.md`，末行 `VERDICT: PASS`，BLOCK 0 / WARN 3 / INFO 8；机读结果 `REVIEW-1571-full-opus.RESULT.json`，评审提示 `REVIEW-1571-brief.md`。WARN-1（本指针悬空）与 WARN-2（本节慢退出措辞）已在本文订正；WARN-3（旧规则库带未决 Begin 时 serve 指引 recover 而 recover 已拒，成死路）属实现行为，登记于 #1405 交主控裁定。

审查完成。结论先行：**PASS-WITH-NOTES**，没有合入前必须改的项。

## 一、rebase 一致性与 main 上的调用上下文

- `atomic_init.rs`、`atomic_init_tests.rs` 两个新文件与整合分支 `f73bfa60e6` 上的版本**逐字节相同**（`diff` 空输出）。`mod.rs` 的 +/- 行集合与 `3ddd2323..f73bfa60e6` 的 patch **完全一致**，唯一差异是整合分支多出的 admission 模块声明上下文，这正是 #1568 评论里 split 审计预告的冲突点。
- main 上的 `open_db`（`rust/src/bin/s_structure_session.rs:320`）没有 admission 门，只做「存在则判旧规则只读」+ WAL/FULL；暂存路径在 `open_db` 时不存在，直接走写连接，建库语义不变。
- `process_lock`（`rust/src/session_transport.rs:43`）在目标不存在时按「父目录 canonical + 文件名」取锁 `<db>.writer.lock`，锁文件不匹配暂存命名格式，不会被 `sweep_stale` 误删；`staging` 在 `_lock` 之后声明，因此 Drop 清扫发生在持锁期内（`mod.rs:417-440`）。
- 生产调用方只有 CLI 入口 `mod.rs:285`，其余全是测试里的全新路径；对两态语义透明。`refuse_reset_of_new_domain`、`cmd_reset`、v2 分派均未触碰。

## 二、逐条验收

| 验收 | 判定 | 证据 |
|---|---|---|
| 1. 磁盘满 / fd 耗尽 / 写入中被杀后，目标不存在或为可识别可接续的中间态 | **满足** | 建库全程在 `.<库名>.s3-init.<pid>.<nanos>.tmp` 上（`atomic_init.rs:97-113`，`mod.rs:425-439`）；`conn.close()` 后才发布（`mod.rs:690`）；发布前拒绝非空 `-wal/-journal`（`atomic_init.rs:127-146`）；不覆盖 rename（`:229-248`）。进程内失败由 Drop 删暂存（`:172-179`），被杀只留暂存文件，下次 init 锁内清扫（`:64-87`）。测试断言：磁盘满 `tests.rs:220-226`，被杀 `:238-249`，fd `:275-281`，两态判定 `:89-108` 对存在的目标要求能只读打开且 `s_protocol_meta`/`catalog` 完整。 |
| 2. 负控：残留路径直接重试成功；不误删用户已有库 | **满足** | 重试：`assert_retry_from_scene`（`tests.rs:111-127`）在每个失败现场原路径 `run_init` 并要求成功且暂存清零。不误删：目标存在时在清扫前就拒绝（`mod.rs:414-420`），测试 `tests.rs:302-314` 用 4096 字节伪残留（即 #1527 的形态）验证字节不动且暂存不被清；`sweep_only_removes_this_databases_staging_names`（`:317-345`）5 个 decoy 不被删；`foreign_wal_beside_absent_target_is_refused_and_kept`（`:348-361`）外来 WAL 原样保留。 |
| 3. 本机不跑 cargo，验证走远端 | **满足** | 三份日志头行均为远端 `nc-remote` worktree，sha 与分支 HEAD 一致（绿 `0a1adbffa6`，红 `17e991631c` = `codex/1568-red-oldimpl-20261021` 的 HEAD）。PR #1656 的 GitHub CI 已有 rust-check / default / backtest / shard 2、3 等绿，shard 0、1 与 extras 在我查看时仍在跑。 |

## 三、红绿是否成立

**红成立。** 红分支的 diff 只是在 main 的 `mod.rs` 加了 `#[allow(dead_code)] mod atomic_init;` 和测试模块声明，两个新文件与绿分支逐字节相同，`init_v3` 本体未动。红日志编译成功（`Finished`，`existing_target…` 1 passed），5 个失败的 panic 位置：
- `tests.rs:221` `fsize:4096 失败后目标不得存在`、`:245` `fsize-kill:4096 被杀后目标不得存在`、`:276` `nofile:6 失败后目标不得存在：err 设置 WAL/FULL 失败：unable to open database file`——**三条直接就是旧实现在目标路径留残留**，且 nofile:6 那条正好复现了「文件已建、WAL 没设成」的 #1527 形态。
- `:339`（暂存不被清扫）、`:354`（外来 WAL 没被拒绝）——是旧实现缺这两个行为，属于合理的功能性红，不是编译或无关错误。

**绿没有假绿。** 6 passed / 1 ignored，被 ignore 的只有子进程入口 `s1568_child_init_under_rlimit`，它没有环境变量时本就该空跑（`tests.rs:168-178`），父测试会带变量重新执行它。容忍分支只有两处：fd 扫描里 `None => {}` 和 `Some(_) => 完整` (`:284-286`)，但末尾 `:291-298` 硬性要求至少一次建库阶段失败，且每轮都走 `assert_retry_from_scene`；磁盘满用例对 5 个上限全部要求 `err` 且目标不存在、暂存为空；SIGXFSZ 用例要求子进程确实死于 `SIGXFSZ` 并留下暂存文件。`green-init.log` 还显示既有的 `v3_init_refuses_existing_domains_without_touching_bytes` 等 v3 init 测试在新实现下仍通过。新文件在绿日志中没有任何编译警告。

## 四、发现

**BLOCK**：无。

**M1：macOS 发布路径在本 PR 的 CI 里没有被编译或执行。** `renamex_np`/`RENAME_EXCL` 只在 `target_os = "macos"` 下编译（`atomic_init.rs:241-242`），而 PR CI 的 `macos-session` 作业状态是 SKIPPED（它只在手动触发或 push main 时跑）。我核对了 `libc 0.2.189` 确有 `renamex_np(from, to, flags: c_uint)` 和 `RENAME_EXCL: c_uint`，签名与调用匹配，编译失败的可能性很低；但 SIGXFSZ/RLIMIT 子进程测试在 macOS 上的实际行为（含 `fullfsync`）没有读数。建议合入前手动触发一次 `ci-macos.yml`，或在票上注明「macOS 运行读数待 push main 后由 ci-macos 补」。

**L1：按时 SIGKILL 循环没有「命中 init 中途」的断言**（`tests.rs:253-262`，前轮 L4 未处理）。子进程是整个测试二进制，启动有固定开销，0–160 ms 的落点可能全部落在进入 `init_v3` 之前，那部分就变成空测。因为 SIGXFSZ 用例是确定性的并已覆盖「被杀留暂存」，这不构成假绿，只是覆盖面弱于名字暗示的程度。

**L2：「已发布却报错」的两条路径仍存在**（前轮 L2/N2 未处理）：rename 成功后目录 fsync 失败（`atomic_init.rs:159-161`），以及 link 回退里 `hard_link` 成功但删除暂存名失败（`:212-213`）。两者目标都已是完整库，但命令返回 Err，再试会得到「库已存在」。两态性质不受影响，报错语义有误导；后者还会把一个硬链接暂存名永久留在目录里（目标存在后不再进入清扫）。

**L3：link 回退分支无测试**（`:209-217`，前轮 N1 未处理），只能靠文件系统 errno 触发。另外 `rename_flag_unsupported` 把 `EINVAL` 也当「不支持」（`:219-225`），退到 `hard_link` 仍是不覆盖语义，不会造成覆盖，但会把一些真正的参数错误伪装成「文件系统不支持」。

**L4：绿日志看不到 fd 扫描实际触发了哪些失败。** 通过的测试 stdout 被 libtest 捕获，`S1568_NOFILE_FAILURES` 没有出现在 `green-atomic_init.log` 里。断言本身保证了至少一次命中，但要留档「哪些上限命中了哪一阶段」需要加 `--nocapture` 重跑一次。

**L5：`existing_db_error` 文案**（`mod.rs:363-369`）仍写「请换新路径」。修复后目标路径只会在真有完整库时存在，这句对用户已有库是对的，但模块文档已改成「原路径可直接重试」，建议顺手把 F2 时代的「不自动清理」注释与这条文案对齐，避免读者以为残留仍需换路径。

```json
{"verdict":"PASS-WITH-NOTES","sha":"0a1adbffa6","blocking":[],"notes":["M1: macOS 分支(renamex_np/RENAME_EXCL, atomic_init.rs:241-242)在 PR CI 中被 SKIPPED，libc 0.2.189 符号已核对存在，建议合入前手动触发 ci-macos.yml 或票上注明待 push main 补读数","L1: 按时 SIGKILL 循环(tests.rs:253-262)无命中-init-中途断言，覆盖靠确定性 SIGXFSZ 用例兜底","L2: rename 后目录 fsync 失败(atomic_init.rs:159-161)与 link 回退删暂存名失败(:212-213)会「已发布却报错」，两态不受影响但报错误导、后者留永久硬链接暂存名","L3: link 回退分支(:209-217)无测试；EINVAL 被归为不支持(:219-225)，退到 hard_link 仍不覆盖，无安全问题","L4: 绿日志不含 S1568_NOFILE_FAILURES 输出(stdout 被捕获)，fd 扫描命中明细未留档","L5: existing_db_error(mod.rs:363-369)仍写「请换新路径」，与模块文档「原路径可直接重试」口径需对齐","红绿成立：红分支=main init_v3+逐字节相同的新文件，5 失败中 3 条直接是目标残留(tests.rs:221/245/276)、2 条为缺清扫/缺外来WAL拒绝；绿 6 passed/1 ignored(子进程入口)，无空测","rebase 净改动与 f73bfa60e6 逐字节一致；main 的 open_db 无 admission 门、process_lock 锁文件不匹配暂存格式、Staging 在锁内 Drop"]}
```

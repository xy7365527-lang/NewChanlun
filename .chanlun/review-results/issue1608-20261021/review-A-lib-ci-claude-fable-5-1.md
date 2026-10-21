## 结论：PASS-WITH-NITS

范围 A 内没有发现会改变行为或让门禁假绿的问题。删除全部是编译器能证明的无用 import / 两个无调用的小辅助；`cfg(test)` 门控的模块都是私有 `mod`，`backtest_bin` 构建不会失去任何可达内容；`-D warnings` 与负例脚本我在远端 HEAD 64b8bc5e35 上独立复跑过，均通过。需要处理的是几条「allow 背后藏着的真缺陷要有票号」和 CI 的一处成本提示。

### 我独立核过的事实

- **signal.rs 上涨臂**：`git show 1b27e492b0` 的改动行与本分支逐字相同（`(Some(Direction::Up), Direction::Up) => c.zg < end.price`），本分支已把原来挂在不可达臂上的 `allow(unreachable_patterns)` 撤掉。PR #1611 合入后两边不会冲突。
- **远端复跑**（日志 `/tmp/nc-remote/20261021-102315-64b8bc5e35.log`）：`RUSTFLAGS="-D warnings" cargo check --all-targets --features backtest_bin` rc=0；`check-compile-negatives-1394.sh` 两个负例均 PASS（rc=101，1 条 / 2 条 E0308，落点在 cfg 行 +20 内）。作者之前只在 d071f3d022 跑过该脚本，之后 w2/w3 改动了那两个文件，这次是在最终 HEAD 上补的验证。
- **try_update**：`AtomicUsize::try_update(set_order, fetch_order, f)` 与 `fetch_update` 签名、返回值（`Result<T, T>`，Ok=旧值/Err=当前值）完全一致，是纯改名；`Cargo.toml` 没有 `rust-version` 钉旧版本，CI 用镜像自带 stable，不会因此失配。
- **Cargo.toml `[lints.rust] unexpected_cfgs = { level = "warn", check-cfg = [...] }`**：写法正确（Cargo 1.80+ 语法），两个 cfg 名与源码一致。
- **l3_* / open_ledger 的 `#[cfg(test)]`**：四个 l3 模块和 open_ledger 都是私有 `mod`，bin 本来就访问不到；`cargo test --lib --features backtest_bin` 仍带 test cfg，所以这些模块的测试照常编译运行，零覆盖损失。l3 文档本来就写着「继承 cfg(test)」，这是补回而非新增门。
- **runner.rs / fill.rs 的 import 删除**：七档 check 0 error 即证明删掉的符号在 test / 非 test 两种构建下都没被引用，不涉及行为。
- **allow 形态**：范围 A 新增的 allow 全部是 item 级或 `mod` 声明级，每条带 `reason = "#1608…"`；没有新增 `#![allow]`、`cap-lints`、`let _ =`。
- **负例脚本判据**：fail-closed——找不到 cfg 行、cargo 不存在、`--locked` 锁文件不匹配、出现非 E0308 错误、`-->` 行解析为空，全部走 FAIL 分支；没有找到假绿路径。

### 逐条发现

**1.（中）divergence.rs:1593 — #990 的 ForceL 专项锁从未执行过，reason 写「已上报」但 tracker 里没有票**
`confirm_divergence_forcel_semantics` 嵌在外层测试函数体内，测试框架收集不到，所以 #990 声称落地的锁实际是「只写不执行」（#799 裁定十一口径按无锁计），而且它的注释 `L(b)=-8` 与函数内推导的 -12 对不上，搬出来后可能直接红。本票不动它是对的（搬出来会改变测试集结果）。但我用 `gh issue list --search "confirm_divergence_forcel_semantics" --state all` 查了，没有任何 issue。**建议：合入前开票，把票号写进两处 reason 字符串**，否则「已上报」是空话。

**2.（低）finance.rs:1570 — `stage_token` 改成 `_stage_token` 把信号抹掉了，而且没有 reason**
调用方 finance.rs:1550 认真传了 `Some(&token)`，函数体一行都没读。我看了 `content_store::publish`（content_store.rs:603 起），它按 digest 而非 token 提升暂存行，所以当前不读 token 不会丢数据；但这个参数的设计意图（按 token 隔离暂存）到底是故意废弃还是漏接，需要 economic_session 的 owner 定。同样是 `gh issue list --search "stage_token"` 零命中。**建议**：要么改为 `#[allow(unused_variables, reason = "#1608 疑点：…")]` 挂在参数上（lint 属性允许挂函数参数），要么并入上一条的跟进票；二选一，别只留下划线。

**3.（低）ci.yml:212–245 — `RUSTFLAGS` 只挂在三个 check 步骤上，后面的 `cargo test` 步骤没有**
Cargo 把 RUSTFLAGS 算进指纹：不带 `--target` 时 build script 和 proc-macro 也吃 RUSTFLAGS，所以 check 步骤（`-D warnings`）和 test 步骤（无 flag）会各编一套 build script / proc-macro 产物。这是纯磁盘和时间成本，不是正确性问题，但这个 job 有 #1141 爆盘的前科。两个可选处理：把 `RUSTFLAGS` 提到 job 级 `env:`（test 步骤本来也该零告警；**注意负例脚本那一步必须显式 `RUSTFLAGS: ""`**，否则负例块里的告警会变成非 E0308 错误让脚本误判），或者先合入、看该 job 新增的 `du -sx target` 读数再决定。另外新加的 s_session check 步骤比兄弟步骤少了 `free -b` / `du` 两行遥测，顺手补齐。

**4.（低）coverage/mod.rs:1404 — 孤儿注释**
原来跟在 `pub(crate) use compose::{TwStepCtx, VoiceVerdict};` 后面的「#913 清理时误加 test 门——fill.rs 生产区也消费，订正见 #1022」现在挂在了 `t1_target_*` 探针的 `pub use` 块末尾，而它说的那两个符号已经拆到上面各自的 cfg 行。建议把注释移到 `TwStepCtx` 那一行，`VoiceVerdict` 现在只剩 test 消费（shadow.rs:56 已是 `#[cfg(test)]`）也和这条注释的措辞不再一致。

**5.（低）p940 / p944 / p946 — `#[allow(dead_code)]` 挂在 `#[path = "../X.rs"] mod X;` 上**
语法上是 item 级，效果上是对共享文件在该 bin 内的整文件 allow，是本票所有 allow 里范围最宽的一类。rule ③ 明确允许，而且那些文件在 lib 构建里照常被 lint，我接受；只是记一笔，以后别拿它当「item 级 allow」的范本。

**6.（信息）ledger.rs:190–235 — `OpsemEntrySnapshot` 的 `chi_filter_active` / `b1_sizing_available` / `voice_tree_depth` / `w_depth_at_entry` 从未被读**
这四个字段是 L1-P1 / L2-P4..P6 的插桩，写入后没有任何读出点，说明这些维度从来没进过 opsem dump。allow 处理本身没问题，但这是 #913 口径的「测量装置装了没接线」，建议在跟进票里点名，免得以后有人以为 dump 里有这些列。

**7.（信息）fill.rs:867 — `step_center_oscillation_observed` 文档过期**
文档说「仅供 `OPSEM_DUMP_DIR` 已启用的生产循环与事务落盘测试使用」，现在它是 `#[cfg(test)]`；生产唯一调用点是 `_historical`（p938 文件头也这么写）。改一句文档即可。

### 不需要在本票处理、但别被 reason 文案带偏的两点

- pan_div_short.rs 五处 allow 的理由「生产驱动源归 #575 后续票」引的是 retrace_ledger/mod.rs 里 2026-07-29 的裁定 B，我核对过那段原文，口径一致。
- orchestrator.rs `buysellpoints`「下游消费方尚未接线」属实：lib.rs:933 的 pyo3 元组 `LevelSnapshotTuple` 不含该字段，所以 level≥2 买卖点算完就丢；这是既有状态，不是本票造成的。

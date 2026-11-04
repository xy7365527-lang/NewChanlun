# #1419 M8C：native delta 确定性分片

名分：绑定 #1419 的阶段工作草稿。开工基线 `sandcastle/issue-1419@c744de09213aff923bfdcdb4345ac64acb678c1d`。本阶段只改 native delta 的 CI 执行编排；沿用原 debug profile、测试特性、线程栈、断言、测试源码及每个 job 的 60 分钟门。Rust 编译、真实测试清单和执行耗时均待 CI。

## 1. 取消收据与片数

原件为 `/Volumes/AgentStorage/w2-h1-integrate46x/ci-2c6cb4e3/native.log`，对应 PR #1855 的 `2c6cb4e3`、[run 41434766679](https://github.com/xy7365527-lang/NewChanlun/actions/runs/41434766679)、native job `125241726240`。以下时间均为原日志的 UTC 时间。

| 原件位置 | 实测 |
|---|---|
| `:703` / `:769` | lib delta=60、removed=0；60 passed，0 failed，执行用时 0.57s。 |
| `:938` | bin delta=571、removed=0。两 target 合计 631 项。 |
| `:943` 至 `:2583` | bin 可见首条 ok 为 12:02:13.6109289，末条 ok 为 12:37:01.3668570。全日志 212 条 ok 中 lib 占 60、bin 占 152；没有 FAILED。212 不是 bin 的已完成项数。 |
| `:2612` | 12:37:41.3648840，bounded2 冷锁报告运行超过 60s，尚无 ok。 |
| `:2627` | 12:38:01.3678554，C/D 冷锁报告运行超过 60s，尚无 ok。 |
| 日志尾部 | 12:54:13.4842816，`The operation was canceled.`；随后清理仍在运行的 cargo 和测试进程。 |

主控提供四条 W2 冷锁 debug 各约 10 至 20 分钟的背景估计。本份日志没有四锁的完成时间，不能据此逐条确认该估计，尤其不能把 E/F、W 写成已在此 run 完测。原 delta 步 11:54:12 开始，到 12:02:13 开始 bin 执行约 8 分钟，期间包括清单编译、枚举及 lib 执行；这也不是每个新片必耗 8 分钟的测量。

取 8 片。四条 W2 重锁各独占一片，另外 4 片接其余项。按原 631 项且四锁均在 bin delta 的清单推算，普通项为 627，四片的项数为 157、157、157、156，其中覆盖原 60 项 lib delta。该计数是旧清单规模下的推算，新 CI 每片将打印实际清单与项数。普通 bin 已完成 152 项约历时 35 分钟，但余下测试耗时未知；4 个普通片用于留出执行余量，不能由此承诺每片一定在 60 分钟内完成。真实耗时和覆盖执行收据仍由 CI 给出。

## 2. 分片规则与接口

脚本为 `.github/scripts/test-feature-delta.sh`，从 `rust/` 调用：

```sh
../.github/scripts/test-feature-delta.sh s_capture_native_testonly
../.github/scripts/test-feature-delta.sh s_capture_native_testonly 0 8
../.github/scripts/test-feature-delta.sh --verify-partition s_capture_native_testonly 8
```

第一种保留原单参数完整执行入口；第二种执行 index 指定的一片；第三种只枚举并核验分区，成功后不执行测试。index 从 0 起，count 至少为 5，index 必须小于 count。整数参数拒绝前导零，最多 4 位，避免 Bash 的八进制和整数溢出。

每个 target 分别枚举 `s_session` 与 `s_session,<feature>` 两份 `cargo test --list` 清单，再以 `LC_ALL=C comm` 计算新增和移除。分片模式先收齐 lib、`s_structure_session` bin 两 target；任一清单失败、任一 target `removed != 0` 或两 target 新增全集为空均失败，不执行任何片内测试。

分片单位是 `target<TAB>完整测试名`。bin 中下列四个完整名称固定分派，lib 中即使出现同名项也走普通规则：

| index | 固定 bin 测试名 |
|---|---|
| 0 | `v3::audit_owner::construction_tests::w2_h1_cold_bounded2_shape_development_window_matches_online` |
| 1 | `v3::audit_owner::construction_tests::w2_h1_cold_cd_second_cursor_continuation_matches_online` |
| 2 | `v3::audit_owner::construction_tests::w2_h1_cold_ef_seed_pair_qualification_and_canonical_matches_online` |
| 3 | `v3::audit_owner::construction_tests::w2_h1_cold_w_undetermined_prior_revision_matches_online` |

其余项按整个 `target<TAB>完整测试名` 做 C 排序，排除上述固定项后，从 n=0 编号，归属片为 `4 + n % (count - 4)`。这条规则只取清单文字和显式 count，不取时间、随机值、测试执行顺序或运行时测量。固定 0 至 3 片不接普通项，因此四锁始终分散，普通项也不会增加这些片的执行负担。

脚本为每片生成临时清单并输出 `feature_delta_manifest shard=... count=...` 和全部条目，CI 日志保留清单；退出时删除临时目录。非空全集中的合法空片在核验成功后打印 `empty=true` 并成功返回，不调用无测试名过滤的 cargo。空全集在生成分片前已经拒门，所以不能借空片掩盖。

## 3. 机器核验与实际执行

每个分片入口都重新枚举全集，计算所有片的清单后才执行本片。核验模式走相同路径，到核验成功为止返回。

核验把所有片清单连接后 C 排序，先用 `uniq -d` 检查重复，再用 `cmp` 要求排序后的连接清单与原 delta 全集逐字相同。重复非空或比较不等均非零退出。连接清单等于全集且无重复，同时证明全覆盖与两两互斥；以 target 作为键的一部分，lib/bin 同名测试仍为两个不同成员。该重复门也会拒绝输入清单中的同 target 重复测试名。

成功收据为 `feature_delta_partition verified=true shards=<count> total=<n> overlap=0`。执行时按原 target 顺序 lib、bin 取本片完整名，并沿用 `--locked --features s_session,<feature>`、原 target selector、`--jobs 1 -- --exact <完整名...>`。某个 target 在本片为空时跳过该 target。

原单参数路径没有转入分片函数。`list_tests`、从 `total=0` 开始的原 lib/bin 循环、两份差集、removed 门、日志、执行命令和最终空全集门均保留原代码及顺序。因此原入口仍先完成 lib 的枚举/执行，再处理 bin；分片新增的“先核两 target 再执行”没有改变它。新增参数解析只选择新入口，不改变正常单特性完整执行的 cargo 参数或测试集合。原入口与分片入口的 stub 调用收据另见 §5。

## 4. CI 编排与重复范围

`.github/workflows/ci.yml` 把原 extras 矩阵的 `native` 替换为 `native-0` 至 `native-7`，每项携带对应 shard。native delta 步调用三参数接口，count 固定为 8；脚本内部的全分区核验就是每片执行前的机器门。

矩阵仍用 `max-parallel: 1`、`fail-fast: false`、`ubuntu-latest`、`timeout-minutes: 60`、`CARGO_INCREMENTAL: "0"` 和既有 `rust-job-setup`。因此同一 extras 矩阵同时最多占一个 runner，单片超时值不变。content、economic、owned、release-locks 的条件与命令保留，owned/release-locks 未改。

原 native job 只有这一个 cargo 测试步。拆分后 checkout 和 setup 在 8 个 job 中重复，各片也都重新枚举两 target 的 base/feature 清单，可能重复编译。native 测试全集按分区执行一次，没有另一个原 native cargo 步被遗漏或随矩阵复制。其他组的测试不会命中 `startsWith(matrix.group, 'native-')`，仍按原组执行。

## 5. 本机检查与 CI 待验

本机按任务约束只执行 shell、Python 及 mock 演练，不运行真实 cargo/rustc。独立验证的临时目录为 `/tmp/issue1419-m8c-mock/`，stub cargo 仅返回固定清单并记录参数，不调用 Rust 工具链。实际执行 `bash /tmp/issue1419-m8c-mock/verify.sh`，16 个检查组全部通过；收据为该目录的 `receipt.log`。临时脚本、fixture 和收据均未入仓。

- 合成 delta 共 30 项，lib 13、bin 17，包含跨 target 同名及四条 W2 完整名。8 片实际执行集的并集等于 delta、无交集；四锁独占 0 至 3，完整同名的 lib 项只进普通片一次；base/feature 清单反序后分派逐字相同。
- 核验模式只调用四次 `--list`，执行测试次数为零。合法空片成功且零执行；空全集在分片、核验及旧入口三模式全部拒门。lib/bin 各自 removed 在新两模式均于执行前拒门；四种 list 失败的退出码 29 原样传播；非法、越界、不足 5 片、过大数字及错误个数参数均以 64 拒门且零 cargo 调用。
- 单参数新旧脚本的 stdout、stderr、cargo 参数序列和执行集逐字相同，也覆盖单参数字面 `--verify-partition` 仍按 feature 处理。另只在 `/tmp` 脚本副本故意删掉一个分片成员、或跨片重复一行，两种故障都被机器核验拒绝，零测试执行。
- `bash -n .github/scripts/test-feature-delta.sh`、`shellcheck .github/scripts/test-feature-delta.sh` 均通过。Python `yaml.safe_load` 解析及 `python3 /tmp/issue1419-m8c-mock/verify-ci.py` 深比较通过，确认其他 job、全部非 native 步骤和 owned/release-locks 逐值不变，原 native 唯一测试步变为 8 片，runner、并发上限和超时保持。
- 主控另核原脚本 `list_tests` 函数及 `total=0` 起完整旧循环与 HEAD 字节全等。`git diff --check` 通过，新文档逐行尾部空白核对通过；M2-NOTES 只追加本阶段记录，原 260990 字节前缀逐字保留，SHA-256 为 `3a01f2974feb6f1d64dd08acc2dbfa17afd3f89bf2d758c8c5b8dc31a39e3e30`。

这些收据证明脚本分派、拒门和 CI 静态结构，不替代真实 Rust 编译、631 项清单在当前源码上的再枚举、测试执行或单片耗时。CI 待执行。

## 6. 风险与后续验收

8 片增加串行 job 总数，也重复 setup、枚举与部分编译，整体 CI 墙钟时间可能增长。每片是否低于 60 分钟尚待实跑；若普通项仍有未知重锁，本次按名称轮转无法预知其耗时。

固定指派以完整 target/name 精确匹配。重锁重命名后会成为普通项，更新测试名时须同步检查此表与脚本 case；每片日志给出实际去向，便于验收。普通项新增或删除会改变后续普通项的轮转归属，但同一份清单和相同 count 的输出固定，互斥/覆盖核验仍成立。

机器门证明当前编译配置下 delta 的分区性质，不证明跨配置清单恒等，也不证明每个测试已运行通过。CI 后续须核 8 个 native job 的 `verified=true`、各片实际清单及执行结果，并确认四锁各在指定片完成。M8C 不改变 #1419 原完整验收义务，也不把此前 Rust 锁或资源证明签成通过。

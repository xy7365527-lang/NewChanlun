# #1672 32 MiB 增量异家族独审 27

## 结论：df02 BLOCK

精确候选 `df02abb258586fd60b1c3a028caf682902281fe9` 判 **BLOCK**。设计方向符合用户批准的受限 32 MiB 规格，观测与归档的形态也基本正确。阻断点只有一个，但它是硬伤：新增的 macOS 测试代码在 macOS 上不能编译。我先按源码判定了这一点，随后 run 40380252050 在 18:57Z 前到达终态 `failure`，编译器原文证实了判定：FIN/2 步（第 7 步）与同一 job 的 G-1 库测试步（第 8 步）都因同一处错误编译失败，两个 ignored 测试一个也没有执行。

修复是三行内的整数类型转换，改完需要新精确 head 并重新走 macOS 实跑。其余发现均不阻断，列在后文供同一提交顺手处理或在报告里说明。

审查对象为三文件增量 `ccc115ddfb029adba8aa8fa40f65805328e58302..df02abb258586fd60b1c3a028caf682902281fe9`，共三个提交：`96e7cd7717`、`1bc593111a`、`df02abb258`。工位 `/Volumes/AgentStorage/issue1672-fin2-hfs-20261025/worktree` 的 HEAD 等于 df02，工作树干净，PR1702 headRefOid 等于 df02。作者为 GPT 实施 owner，本审为 Claude 家族。现场时间取自 `date -u`：开审时 2026-10-26T18:50:07Z，读到 run 终态时 2026-10-26T18:57:22Z。

throughput checkpoint: n/a, read-only investigation。

## 阻断：u32 乘 u64 在 macOS 上编译失败

`rust/src/economic_session/tb03c_fin2_tests.rs@df02` 新增的 `storage_stats` 有两行：

```rust
let free = vfs.f_bavail * vfs.f_frsize;        // :270
let capacity = vfs.f_blocks * vfs.f_frsize;    // :273
```

`rust/Cargo.lock@df02` 锁定 `libc 0.2.189`。该版本 `src/unix/bsd/apple/mod.rs:23` 定义 `fsblkcnt_t = c_uint`，即 u32。同文件 `:395-407` 的 `statvfs` 中 `f_blocks`、`f_bavail` 是 `fsblkcnt_t`，`f_frsize` 是 `c_ulong`，在 64 位 macOS 上是 u64。Rust 没有 `u32 * u64`，这两行在 macOS 上报类型不匹配。

这个错误逃过了作者的检查，原因可以从源码直接看出。`rust/src/economic_session/mod.rs:36-37` 用 `#[cfg(all(test, target_os = "macos"))]` 注册该模块。nc-remote 是 Linux，fmt 和回归都不编译它。decisions 记录里只有 "remote fmt and regressions running"，没有任何 macOS 编译证据。生产侧同款读数 `content_store.rs:121`、`:177` 用的是 `u128::from(vfs.f_blocks) * u128::from(vfs.f_frsize)`，所以生产代码不受影响。

run 40380252050 的 job 121774245820 日志给出编译器原文。第 7 步 18:54:52Z 报 `error[E0308]: mismatched types` 与 `error[E0277]: cannot multiply u32 by u64`，位置 `src/economic_session/tb03c_fin2_tests.rs:270` 与 `:273`；`:274` 因 `capacity` 被推成 u32 另报 `cannot divide u32 by u64`。18:55:08Z 收尾为 `error: could not compile newchan_rust (lib test) due to 6 previous errors`。第 8 步 G-1 在 18:55:40Z 报同样六处错误。本审没有在 macOS 本地编译。

**修法。** 二选一，同一提交内完成：

1. 最小改动：`let free = u64::from(vfs.f_bavail) * vfs.f_frsize;` 和 `let capacity = u64::from(vfs.f_blocks) * vfs.f_frsize;`。`:274` 的除法随之成立，无需另改。
2. 推荐：容量改用 `super::content_store::carrier_capacity(path)`。它已经是 `pub(super)`，只读，还核 statvfs 与 statfs 一致及挂载根。这样观测用的页帽公式与生产 `configure` 的 `content_store.rs:323-332` 共用同一个容量来源，以后不会漂移。空闲字节仍按方案 1 转成 u64。

改完后由实施 owner 推新 head、在 macOS 上实跑两个 ignored 测试，再交本增量复审。复审只需核这两行及运行原件。

## 概念层质询：通过

本增量不涉及缠论教义定义。对照的合同是原 FIN/2 spec `/Volumes/AgentStorage/issue1673-fin2-spec-revision-20261025/spec-candidate.md` §7 与用户批准文本 `1672-SPEC-GATE26BK.md`，未发现矛盾。

载体尺寸只改了一处。`drive_tb03c_fin2_hfs.sh:64` 由 `-size 16m` 改为 `-size 32m`，仍是 UDIF、HFS+、四卷 E/b0/b1/b2、挂载根、设备号互异。32 MiB 镜像的实际 HFS 容量必小于镜像名义 33,554,432 B，而 `store.rs:170-173` 与 `content_store.rs:325` 只在容量**大于** `max_store_bytes`/`max_staged_bytes`（各 33,554,432 B，`tb03c_fin2_tests.rs:42-43`）时拒绝。run 40380252050 已实测四卷 `df -k` 总容量均为 32728 KiB，即 33,513,472 B，低于该上限 40,960 B，与旧提案假设的文件系统开销一致。按 `content_store.rs:332` 算出页帽 floor(33,513,472 / 32,960) = 1016。这是由实测容量推算的页帽，`configure` 实际设置的 `max_page_count` 要等测试真正跑起来才有读数。

保持不变的项目已逐项核对：三文件之外无改动，`content_store.rs`、`store.rs`、`service.rs` 与 fixture 在 ccc115 到 df02 之间零差异。8 倍页帽公式 `content_store.rs:332`、cleanup 余量 `:151-190`、`reader_resources` 全部字段 `tb03c_fin2_tests.rs:28-46`、`response_timeout_ms=30000`、`max_elapsed_ms=30000` 均未动。没有自动扩卷或失败重试路径，32 MiB 不足时测试 panic、driver 以非零退出。

观测遵守 26BK 的约束。`observe_storage` 只做 PRAGMA 读、`SELECT` 最新 commit 图像长度、`metadata` 与 `statvfs`，不调用 `service::handle`，不提交，不改夹具时钟。旧的 `apply_image_sizes` 仍只在失败分支 `:415-427` 构造下一图像，没有被移到成功路径。证据写入 `TB03C_FIN2_EVIDENCE_DIR`，CI 中指向 `runner.temp`，不占载体空间。

观测里复写了一份 cleanup 预算公式（`:274-298`）。它只记录不准入，属于 #799 限定词 2 的诊断口径，不构成宽严两档。上面推荐的修法 2 能把它的容量来源收回到生产函数。

原合同的正例断言全部保留且未改：6 Capture、3 Resolve、12 Apply 逐轮 HAND 对拍 `:530-584`；HAND 第 3 轮即 `[2,5,5]`、`[204,506,506]`，已从 `HAND-ORACLE.json@df02` 读出；21 原件与 12 个唯一 allocation `:586-595`；四域关闭、recover、重开、audit、图像逐字一致、暂存为零 `:603-627`；21 次原信封重送、仅 epoch 两字段变化、seq 与 messages、receipts、applications 不变 `:643-701`。

## 工程层发现

| 级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 1 | macOS 编译失败，BLOCK |
| HIGH | 0 | |
| MEDIUM | 2 | 不阻断，建议同一提交处理 |
| LOW | 4 | 记录 |

**MEDIUM 1：`max_page_count` 读的是测试连接，不总是生效页帽。** `service.rs:1419` 在每次 `handle` 时对该连接调用 `configure`，`PRAGMA max_page_count` 是连接级设置。B 域 `initialized`、四域 `after_recover`、每域第一条 `before_resend` 读到的连接尚未经过 `handle`（`store::open` 在 `store.rs:508-545` 不设页帽，`audit` 也不设），这些行会显示 SQLite 默认值而不是约 1016 的载体页帽。每条原件 op 之后的读数是生效值。建议把该字段改名为 `connection_max_page_count`，生效页帽以 `carrier_page_limit` 为准。至少要在实施报告里说明，避免验收时误读。

**MEDIUM 2：两个 ignored 测试的通过行只在 GitHub job 日志里。** `RESULT.json` 只在正例全部通过后写入，负控测试不留结果文件。"2 passed、0 skipped" 这条验收只能靠 job 日志，而 26BK 记录过旧 job 原始日志端点 HTTP 404。建议 driver 用 `"$@" 2>&1 | tee "$TB03C_FIN2_EVIDENCE_DIR/cargo-test.log"` 把完整输出落进证据目录。`set -o pipefail` 已在 `:3`，退出码保持。

**LOW 1：detach 失败现在会让成功的测试变成失败。** 这符合"回收失败不可伪成功"。run 40380252050 的失败路径里四卷都 detach 成功，但 e 卷用了约 10 秒（18:55:09Z 到 18:55:19Z），其余不到 1 秒。成功路径上卷里有真实 DB，`cp -R` 后占用时间可能更长。若实跑出现 busy，可加有界重试并把每次尝试写入 `cleanup.txt`，仍失败则保持非零。

**LOW 2：`rm -rf "$ROOT"`（`:42`）没有 `||` 保护。** errexit 在 trap 内仍生效，rm 失败时 trap 直接退出，`cleanup.txt` 不会写出。退出码仍非零，不会伪成功，只是少一份证据。改为 `rm -rf "$ROOT" || cleanup_failed=1` 即可。

**LOW 3：拷贝失败时载体原件只留在 `$ROOT/*.dmg`。** 该目录不在上传路径内，runner 回收后即消失。可在 `cleanup_failed=1` 分支把 `$ROOT` 的 dmg 一并移进证据目录。

**LOW 4：增量里有一项未声明的测试新增。** 负控测试新增了"同一 HFS 卷内非挂载根"用例 `:760-771`。它覆盖原 spec §7.3 的"非挂载根"负控，旧 APFS 用例只断言 `HFS || mount root`，实际命中的是 HFS 分支。错误串来自 `content_store.rs:95-97` 经 `store.rs:169` 包装，包含 `mount root`，断言成立；目录在正例之前删除。这是原合同内的必要补强，不是扩规格，但应写进实施报告。

**注释。** 按 no-comments 的 Comment Sicko 口径过了三文件增量。增量没有新增注释，`ci-macos.yml:76` 的 16 改 32 是事实更新。增量内删除 0 条，`MUST KILL` 0 个。增量外有一条过期注释 `content_store.rs:147` 仍写 "16 MiB HFS trial"，属生产文件，不在本次写集，登记待后续票处理。

**CI 文件。** `ci-macos.yml` 只给 FIN/2 步加了 `env`，并在 job 末尾加了 `if: always()` 的 `actions/upload-artifact@v4`，`if-no-files-found: error`，名字带 `github.sha` 和 `run_attempt`。与仓内 `ci.yml:114`、`:302`、`devskim.yml:66` 的用法一致。其他步骤与其他 workflow 文本未改。

失败路径已在 run 40380252050 上实际走通。第 9 步上传 success，artifact `issue1672-fin2-df02abb258586fd60b1c3a028caf682902281fe9-1` 含 `cleanup.txt`、`carriers-before-detach.txt`、`provenance.txt` 与 driver 副本，driver 副本与 df02 对象逐字相同。`cleanup.txt` 记 `command_exit=101`、`cleanup_failed=0`、`root_exists=no`，driver 把 cargo 的失败码原样传出。`provenance.txt` 记 `source_sha=df02…`、源码状态干净、macOS 26.6.2，挂载选项含 `noowners`，所以 `cp -R` 不受卷内 root 属主影响。

artifact 里没有 `carriers/` 目录。原因是编译失败时卷内只有隐藏系统目录，而 `actions/upload-artifact@v4` 默认不上传隐藏文件与空目录。成功运行的验收须确认 `carriers/*/owner.db` 确实在 artifact 中。

## 运行证据缺口

以下各项都没有结果，不计 PASS：

1. run 40380252050（workflow_dispatch，headSha 等于 df02）终态 `failure`：第 6 步 success，第 7、8 步编译失败，第 9 步上传 success。它只提供了四卷实测容量与失败路径回收证据，不提供任何业务 op、页数或 32 MiB 是否够用的读数。
2. 修复后的新 head 上，两个 ignored 测试须实际执行并通过，不接受 0 passed 或 skipped。
3. 逐 op 的生效页帽、page_count、freelist、DB/WAL/SHM 分配、cleanup 与 next-write 余量、恢复和重送前后读数，全部来自新 run artifact 的 `storage.jsonl`。32 MiB 是否足够只能由这些读数回答。
4. 成功路径的 `cleanup.txt` 仍须 `cleanup_failed=0`、`root_exists=no`，且 `carriers/*/owner.db` 在 artifact 中。
5. 恢复内部峰值不在观测内，只有恢复前后两点。批准文本要求的是"恢复/重送前后"，这一项满足；峰值超限会由生产 cleanup 检查失败暴露。
6. 新 head 的非作者整票独审。

## 集成门（与容量验收分开）

PR1702 当前 `mergeable=CONFLICTING`，`mergeStateStatus=DIRTY`。df02 没有 PR 常规 CI 与 DevSkim 结果。本地 `origin/main` 与远端一致，为 `499841b59265c5395ccdd40b92684b5864802f4f`，合并基为 `13ed4d45c8ff85175819ca987690328ab2a247a4`。

用 `git merge-tree` 三方只读计算，文本冲突只在 `.github/workflows/ci-macos.yml` 的两处：一处与 main 的 #1717 `sr_expansion` HFS 步相邻，一处与 #1716 的 artifact 步相邻。集成时两边的步骤与上传都要保留，artifact 名字不重名。`rust/src/economic_session/mod.rs` 两边都改了，但自动合并无冲突。

main 同期还改了 FIN/2 路径的直接依赖：`store.rs`（`Config::load` 新增 `sr_expansion::validate_config`，`init` 写 SR meta，`audit` 新增 SR 重放与回执分支）和 `service.rs`。所以 df02 或修复 head 上的 macOS 读数不能直接转到集成后的 head。集成 head 必须重跑 macOS FIN/2 步与 G-1，并补 PR 常规 CI、DevSkim，再做增量独审。main 合入仍需用户另批。

## 已查证据

- 完整读：Poteto mode SKILL.md 含 Principles，Investigation playbook，how、no-comments 与 `references/comment-sicko.md`、Prove It Works、technical-writing、unslop，`~/.agents/pstack-models.md`，仓库 AGENTS.md。
- 完整读：`1672-SPEC-GATE26BK.md`、`1672-CAPACITY-REVIEW26BK.md`、`CONTINUATION27E.md`、`preferences.md`、原 spec、原 `CAPACITY-PROPOSAL.md`、实施方 `1672-32M-TODO27.md` 与 `1672-32M-decisions27.tsv`。实施报告 `1672-32M-IMPLEMENTATION27.md` 尚不存在。
- 冻结对象逐段读：df02 的三文件全文，以及直接依赖 `content_store.rs:25-136`、`:141-190`、`:292-349`、`:1137-1150`，`store.rs:140-185`、`:505-545`、`:662-671`、`:765-809`、`:815-838`，`service.rs:1395-1435`，`tb03c_allocation_tests.rs:44-129`，`mod.rs:36-37`，`HAND-ORACLE.json`，`Cargo.lock` 的 libc 条目，以及本机 cargo 缓存中 libc 0.2.189 的 apple 定义。
- GitHub 只读：run 40380252050 的步骤状态、`--log-failed` 日志与 artifact（下载到 `/tmp/rev1672-art.h4Vx`，仓库与工位外），旧 run 40178218217 的步骤状态，PR1702 的 head、文件与合并状态。旧 run 的 FIN/2 步 09:00:12Z 到 09:02:08Z 失败，随后 G-1 步被取消，不是超时。
- 没有改动实现、票、store、Git refs 或卷，没有 dispatch，没有运行 Rust 或 hdiutil。仓库外的写入只有本报告和上述 artifact 下载目录。
- 源码判定之后、run 终态之前，已向主控发早报，内容为阻断项与修法。

## 方法说明

Prove It Works 改变了一个判断。作者记录的远端回归在跑，但我没有把它当作编译证据，而是按 `cfg` 门判定 nc-remote 根本不编译该模块，再去锁定版本的 libc 源码里读类型，这才找到阻断点。how 的调用链核读决定了 MEDIUM 1：从 `Session::send_envelope` 追到 `service::handle` 再到 `configure`，确认页帽是连接级设置。no-comments 的 Comment Sicko 口径由我在只读模式下亲自执行，没有派子代理，因为本审禁止改代码，独审结论也不外包。technical-writing 与 unslop 用于本报告的结构和措辞。

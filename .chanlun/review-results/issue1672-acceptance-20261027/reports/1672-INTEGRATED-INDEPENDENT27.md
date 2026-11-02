# #1672 集成候选增量异家族独审 27

## 结论：代码 PASS，运行门未齐

`9e77756b5038007d081962875d948511284bcef3` 与最终 head `0932d3dbcdb18d138ce1e67ed05f971d87e75e3b` 的代码增量判 **PASS**。df02 的编译 BLOCK 已解除，messages 全行对拍没有弱化合同，集成 main 只做了 `ci-macos.yml` 的并集解冲突，生产文件零差异。

9e777 到 0932d3d 是纯 rustfmt。去掉全部空白后，两版只差一对包住闭包体的花括号（`|row| Ok(..)` 改成 `|row| { Ok(..) }`），这是同一表达式。workflow、driver 与其余文件逐字不变。所以 macOS run 40389540213 的 FIN/2 结果可以复用到 0932d3d，前提是 0932d3d 的 PR 常规 CI 与 DevSkim 通过。

FIN/2 业务门已在 run 40389540213 上实跑通过：两个 ignored 测试 `2 passed; 0 failed; 0 ignored`，最终 B `[2,5,5]`、K `[204,506,506]`，21 次重送前后存储读数零变化，四域恢复，四卷回收 `cleanup_failed=0`、`root_exists=no`。

整 run 尚未成功。`issue1716-real-s` job 已终态 failure，原因是 main 自带的 #1716 debug 测试越过 3000 ms 期限，不归因于 #1672 增量，详见运行门第 3 项。`issue1717-real-s` 已 success。G-1、macos-session 内的 #1716 步、0932d3d 的 PR CI 与 DevSkim 在本报告写成时仍在途，不计 PASS。

审查对象：
- 返修增量 `df02abb258..6c1db9ce6a`（提交 `1bbd512c6a`、`6c1db9ce6a`）。
- 集成提交 `9e77756b50`，父为 `6c1db9ce6a` 与 main `499841b592`。
- 格式提交 `0932d3dbcd`，父为 `9e77756b50`。

工位 `/Volumes/AgentStorage/issue1672-fin2-hfs-20261025/worktree` 的 HEAD 等于 0932d3d，PR1702 headRefOid 已是 0932d3d。作者为 GPT 实施 owner，本审为 Claude 家族。时间取自 `date -u`，开审 2026-10-26T19:47:59Z。

## 概念层质询：通过

本增量不触及缠论教义定义。对照合同为原 FIN/2 spec `/Volumes/AgentStorage/issue1673-fin2-spec-revision-20261025/spec-candidate.md:174`（"21 次原消息重送均返回原收据且无新 commit/第二 application"）与用户批准文本 `1672-SPEC-GATE26BK.md`，未发现矛盾。

测试里复写的 cleanup 预算公式只记录、不准入，属 #799 限定词 2 的诊断口径。本轮它的容量来源已改为生产函数 `content_store::carrier_capacity`（`tb03c_fin2_tests.rs@0932d3d:286`），与生产 `configure` 共用同一容量读法，比上轮更贴近单一判定。

## 上轮 BLOCK 的解除

上轮阻断是 `tb03c_fin2_tests.rs@df02:270`、`:273` 的 u32 乘 u64。现为：

- `:283` `let free = u64::from(vfs.f_bavail) * vfs.f_frsize;`
- `:286` `let capacity = super::content_store::carrier_capacity(path).unwrap();`

这就是上轮推荐的修法 2。`carrier_capacity` 在 `content_store.rs@0932d3d:56` 返回 `Result<u64, CodecError>`，签名与 `:287` 的除法一致。编译已由真实 macOS 证明：1bbd 的 run 40382609382 和 9e777 的 run 40389540213 都编译并执行了该模块。

上轮其余发现的处理：
- MEDIUM 1 已解决。字段改名为 `connection_max_page_count`（`:299`）。9e777 原件里四域都同时出现 1016 与 SQLite 默认值 4294967294，正好对应"重开连接尚未经 `handle` 配置"的情况；生效页帽按 `carrier_page_limit` 读。
- MEDIUM 2 已解决。driver `:94` 用 `tee` 把完整 cargo 输出写进证据目录，`pipefail` 保留退出码。9e777 原件 `cargo-test.log` 含两条 `ok` 和 `2 passed`。
- LOW 2 已解决。driver `:42` 改成 `rm -rf "$ROOT" || cleanup_failed=1`。
- LOW 1 未改。两次真实运行四卷都正常 detach，不需要改。
- LOW 3 未改，见下文 LOW 1。

## messages 全行冻结：合同不弱化

`store.rs@0932d3d:52` 的表定义是 `messages(k TEXT PRIMARY KEY,payload_hash TEXT NOT NULL,receipt_key TEXT NOT NULL,request_json TEXT NOT NULL)`，全仓没有 `ALTER TABLE messages`，唯一写入点是 `service.rs@0932d3d:668` 的四列 INSERT。新 helper `stored_messages`（`tb03c_fin2_tests.rs@0932d3d:241`）按 `ORDER BY k` 读这四列，所以它比的就是整行整表。

旧写法 `store::rows(messages)` 走 `store.rs@0932d3d:611-620` 的 `SELECT json FROM {table}`，messages 没有 json 列，必然报错。1bbd 原件里 panic 落在当时的 `:630`，即 `before_messages` 的收集行，报错原文为 `no such column: json in SELECT json FROM messages`。这说明原断言从来没有执行过，16 MiB 时代的 ccc115 也没走到这里。现在它第一次真正执行。

断言强度：重送后 `:702` 要求每域 messages 逐行逐列等于重送前 `:642-643` 的冻结值。这比合同要求的"不增长"更强，行数增长和任一列改写都会失败。逐条 `request_json` 字节对拍（`:684-691`）、`session.next` 不变（`:693`）、seq、图像、receipts、applications 不变（`:694-710`）全部保留，df02 到 0932d3d 之间这段除 messages 两处外零改动。

## 观测语义、资源与规格不变

df02 到 6c1db9c 的 Rust 增量只有上述三处加 messages helper。merge 提交的 remerge-diff 只涉及 `ci-macos.yml`。因此以下各项与上轮已审状态相同，本轮复用上轮核对：32 MiB 四卷（driver `:64`）、`reader_resources` 全部字段、`response_timeout_ms` 与 `max_elapsed_ms` 30000、8 倍页帽公式、cleanup 余量、全部输入、6/3/12 业务序列、四域恢复、21 次重送，以及失败分支外不构造下一次 Apply。

`observe_storage` 仍然只读：PRAGMA、`SELECT` 最新图像长度、`metadata`、`statvfs` 加只读的 `carrier_capacity`，不调用 `service::handle`，不提交，不动夹具时钟。

## 集成：并集解冲突，main 依赖语义中性

`499841b592` 和 PR1738 的 `d310c46dd2` 都是 9e777 的祖先。`git show --remerge-diff 9e77756b50` 显示唯一冲突在 `ci-macos.yml`，解法是两侧原文都保留，只删冲突标记，外加一个空行。相对 main，最终 head 只改五个文件：`ci-macos.yml` +16、`mod.rs` +3、`tb03c_allocation_tests.rs`（PR 早先已审的可见性与 helper 拆分）、`tb03c_fin2_tests.rs`、driver。生产源码零差异。

最终 `ci-macos.yml` 的 macos-session 步序是 financial_session、FIN/2（`:88`）、FIN/2 归档（`:94`）、#1717 SR（`:105`）、G-1（`:124`）、#1716 cap（`:132`）、#1716 上传（`:161`）。`issue1716-real-s` 与 `issue1717-real-s` 两个 job 原样保留。四个 artifact 名字互不相同。归档放在 G-1 之前，G-1 再长也挡不住 FIN/2 原件上传；本次 run 中归档步 19:58:50Z 已 success。

main 对 FIN/2 直接依赖的改动逐项核过：
- `Config::load` 新增 `sr_expansion::validate_config`（`store.rs@0932d3d:120`）。它返回 `registry(manifest)?.is_some()`（`sr_expansion.rs@0932d3d:329-331`），FIN/2 manifest 不选 SR 时返回 false，不报错。
- `init` 只在 `bound_revision` 为 Some 时写 meta（`store.rs:805`）。`audit` 比较双方都是 None（`store.rs:1011`）。SR 的重放与回执分支只按 SR 的 kind 进入。
- `service.rs` 的改动是新增 SR、`ActivateCapPolicy` 分派和 `serve` 函数抽取，FIN/2 经过的 `handle` 路径分支没有变化。

实测与此一致：9e777 上 FIN/2 的 Config、init、recover、audit 全部通过。

## 9e777→0932d3d 纯格式裁定：可复用

`git diff 9e77756b50 0932d3dbcd` 只改 `tb03c_fin2_tests.rs` 两处，+4/-5 行：`stored_messages` 的闭包换行加块，`:702` 的 `assert_eq!` 合并成一行。我把两版去空白后逐字比对，再把闭包块还原回去，结果完全相同。owner 存档的 `1672-32M-evidence27/final-format.diff` 与此一致。

裁定：run 40389540213 的 macOS 读数（FIN/2 两个测试、存储 JSONL、原件、回收）适用于 0932d3d。条件有三：
1. 0932d3d 的 PR CI 40390534105 与 DevSkim 40390534300 须通过。其中 rust-check（含 fmt）在 19:56Z 已 success。
2. 验收记录须写明 artifact `provenance.txt` 的 `source_sha=9e77756…`，并以 `final-format.diff` 作为到 0932d3d 的映射。
3. 40389540213 其余 job 与步骤须按运行门达到终态。

## 工程层发现

| 级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | |
| LOW | 3 | 记录，不阻断 |

**LOW 1：卷回收失败时 dmg 仍只留在 `$ROOT`。** 上轮 LOW 3 未改。driver 在 `cleanup_failed=1` 分支保留 `$ROOT`，但该目录不在上传路径里。两次真实运行都没有走到这条分支。

**LOW 2：过期注释在写集外。** `content_store.rs@0932d3d:147` 仍写 "16 MiB HFS trial"。这是生产文件，不在本票写集，留给后续票。

**LOW 3：owner 摘要字段名会误读。** `1672-32M-evidence27/1bbd-run/observed-summary.json` 用 `max_page_count` 表示"观测到的最大 page_count"（E 是 209，不是 PRAGMA 值）。这是证据摘要，不是代码。验收摘要应改称 `max_observed_page_count`，避免和上轮 MEDIUM 1 的混淆重演。

注释按 no-comments 的 Comment Sicko 口径过了本轮增量：Rust 与 driver 增量没有新增注释，`ci-macos.yml` 只搬动了步骤位置，注释增删都是 0，没有 MUST KILL。

## 运行门

run 40389540213（workflow_dispatch，headSha 9e777，19:45:14Z 建）：

1. **FIN/2 步 7：success（19:58:49Z）。** artifact `issue1672-fin2-9e77756b5038007d081962875d948511284bcef3-1` 已下载到 `/tmp/rev1672i-fin2art.vC8i`，在仓库与工位之外，逐项核读如下。
   - `cargo-test.log`：负控 `fin2_capacity_shared_carrier_and_non_hfs_are_rejected ... ok`，正例 `ok`，`test result: ok. 2 passed; 0 failed; 0 ignored; ... 3531 filtered out`。
   - `RESULT.json`：captures 6、resolves 3、applies 12、resends 21、epochs `[2,2,2,2]`、next 22、sequences `[10,4,4,4]`。三本 B 的 `actual_quantity` 为 2、5、5，`cost_basis.K` 为 204、506、506，与 `HAND-ORACLE.json` 第 3 报逐值相同。
   - `recoveries.jsonl`：四条 `Recovered`，cut 分别为 E seq 10、三个 B seq 4。
   - `storage.jsonl` 共 75 条：21 条 original、21 对 before/after_resend，以及四域各自的 initialized、before_recover、after_recover。四卷容量均为 33,513,472 B，页帽 1016。最大 page_count 依次为 E 209、N1 584、N2 605、N3 622。最小 cleanup 余量 21,647,744 B，最小 next-write 余量 13,144,832 B，全部为正。21 对重送前后的 seq、page_count、current_image_bytes、freelist、allocated 零变化。
   - 32 MiB 下没有出现容量不足。
   - `cleanup.txt`：`command_exit=0`、`cleanup_failed=0`、`root_exists=no`。`provenance.txt` 列出四个互异设备 disk31s1 到 disk34s1，均为 hfs、local、nobrowse 挂载根。`carriers/{e,b0,b1,b2}/owner.db` 都在 artifact 里。
2. **归档步 8 success、#1717 SR 步 9 success。** 截至 20:00Z，G-1 步 10 在跑，#1716 cap 步 11 与上传步待跑。G-1 旧失败用例 `queued_unix_job_keeps_ingress_response_deadline_in_actor` 已由祖先 d310c46 修过，须看本步终态。
3. **`issue1716-real-s` job 121806620771：failure（19:53:57Z）。** `handler-reserve=101`，`formal-sockets=0`。失败点 `finance_real_s_tests.rs@0932d3d:217`，Reserve 返回 `Incomplete：经济操作最早绝对期限已过; staged_cleanup rows=1310 ...`。该测试冻结 `response_timeout_ms` 为 3000（`:177`），debug 构建下这次 Reserve 越期。
   - 归因：这是推断，不是证明，但把握在八成以上，此次失败与 #1672 无关。理由一，相对 main 本 PR 不改任何生产文件。理由二，该 job 跑在独立 runner 上，不经过 FIN/2 步骤。理由三，同代码在 main 的 run 40378315437 上这个 job 曾 success。
   - 这是 #1716 线的期限敏感问题，责任在 #1716 owner 与主控。禁止为过验修改期限或生产代码。是否对同一 head 重跑该失败 job、或交 #1716 线定性，由主控决定。原件在 `/tmp/rev1672i-1716art.krB5/work/handler-reserve.log`。
4. **`issue1717-real-s` job：success（19:56:37Z）。**
5. **0932d3d 的 PR CI 40390534105：** 截至 20:02Z，rust-check、test、fixture-drift、s-observer、sandcastle-ts、#1488 Plain 与 Owned、s-session-extras (content) 已 success，其余在途；macos-session 依设计在 pull_request 事件下跳过。**DevSkim 40390534300：在途。** 9e777 的 PR CI 40389552228 已 cancelled，被新 head 取代，不计。
6. main 合入未获本轮批准。

## 已查证据

- 完整读：Poteto mode SKILL.md 含 Principles，`principle-prove-it-works`、`principle-test-behavior-not-implementation`、`no-comments`；上轮 `1672-32M-INDEPENDENT27.md`、`1672-32M-IMPLEMENTATION27.md`、`CONTINUATION27E.md`、`preferences.md`、`1672-SPEC-GATE26BK.md`、`1672-32M-decisions27.tsv`、`1672-32M-MERGE-TREE27.txt`，以及原 spec 中关于重送的段落。
- 冻结对象只读：上列各提交区间的 diff 与 remerge-diff；`tb03c_fin2_tests.rs@0932d3d:241-312`、`:636-740`；`store.rs@0932d3d:44-63`、`:120`、`:605-620`、`:805`、`:1011`；`content_store.rs@0932d3d:56-100`、`:141-190`；`sr_expansion.rs@0932d3d:329-336`；`service.rs` 的 main 增量；`ci-macos.yml@0932d3d` 全部步骤；driver 全文。
- GitHub 只读：run 40389540213 步骤状态、`issue1716-real-s` job 日志、两份 artifact；runs 40382609382、40378315437 的步骤时序；PR1702 head 与 check 状态。
- 没有改代码、store、票、Git refs，没有 dispatch 或重跑，没有在 macOS 编译 Rust。仓库外写入只有本报告和 `/tmp` 下两个 artifact 目录。

## 方法说明

Prove It Works 改变了两处判断。第一，"纯格式"没有凭 diff 外观认定：我去空白后逐字比对，再把闭包块还原，确认 token 级等价，复用裁定才有依据。第二，FIN/2 是否通过没有采信 job 绿色，而是下载 artifact 逐项读 `RESULT.json`、`HAND-ORACLE.json`、`storage.jsonl`、`cleanup.txt` 与 `provenance.txt`。Test Behavior, Not Implementation 用在 messages 断言上：把行数增长和列改写两种"破坏行为"代进去都会失败，而且它比较的是持久表的可观测内容，不绑实现细节，所以判为合同内加强，不是弱化。no-comments 的口径由我在只读模式下亲自执行，没有派子代理，本审禁止改代码。

APPROVE（无 HIGH/MEDIUM；1 条 LOW 建议顺手清理，1 条 INFO）

# #1406 合并提交独立评审：911d795fde

评审对象：`codex/1406-owner-20261020` @ `911d795fde78e58f328ef86fff5fccad399564a1`，第一亲 main `df6e680930`，第二亲 validate/1406-c6a `cc0bdfa832`，merge-base `25bc128d88`。评审人非作者，只读，未改码、未提交、未 push。

## 一、合并面全量核对（无意外改动）

- 双方共改文件恰为 9 个：`comm -12` 两侧 `git diff --name-only 25bc128d88 <side>` 得 1350-AFFECTED-CODE.md、ci.yml、ADR 0032、capture.rs、q_content.rs、q_resources.rs、q_snapshot.rs、q_watch.rs、finance_tests.rs。
- 只有一侧改的文件逐个 `git diff --quiet <该侧> 911d795fde -- <f>`：**全部为空**（main-only 与 stack-only 均无一个文件偏离改动侧）。
- 合并结果里不存在两侧都没碰过的新改动文件（`comm -23 merge集 (main∪stack)` 为空）。
- 共改文件与两亲的差异行数（`^[+-][^+-]`）：capture.rs / q_resources.rs / q_watch.rs 对两亲均为 0（两侧内容逐字相同）；q_content/q_snapshot 对 main 为 0；ADR 0032 对 main 为 0。

⟹ auto-merge 没有在冲突文件之外制造文本差异；语义层用编译与测试兜底（第六节）。

## 二、ci.yml：干净并集

`git diff cc0bdfa832 911d795fde -- .github/workflows/ci.yml` 与 `git diff 25bc128d88 df6e680930 -- .github/workflows/ci.yml` 的 hunk 内容完全相同（timeout-minutes 150、`--no-fail-fast`、TB-02-B #1405 与 TB-02-W #1420 步骤），只是行号偏移（栈侧在其前加了 ~500 行）。对 main 的 480 行差即栈侧新增内容。无丢失、无重复步骤。

## 三、ADR 0032 与 1350 清单

ADR 0032 合并结果 = main 版。main 版修订节已含全部已裁内容：缺陷原因与反例 q=(1,3,3)、F 3→4 由 (1,1,1)→(0,2,2)；Balinski–Young 配额法逐单位生成规则 `a_i+1 ≤ ⌈f·q_i/Q⌉`、取 `q_i/(a_i+1)` 最大；并列先大请求量再 `tie_break_order`、金额残量同法；四性质加配额界；逆差量规则不变；政策 id `frozen-manifest-proportional-cumulative/2`、/1 未落持久库、不得写出 /1；Lean 增配额界；1350 同步。main 的「受影响代码清单」节还比栈版多列了「配额界」（栈版漏了这一项）。

栈版有而 main 版没有的只有论证材料：穷举数（n=2–4、q_i≤8 的 4,672 清单中 1,738 反向；根复算 486 中 156 反向）、「Alabama 悖论」称谓、「均有文献证明；穷举零违例」。这些是原因陈述的证据，不是裁定条文，**不算已裁内容丢失**（INFO-1，见下）。

1350-AFFECTED-CODE.md：表格行取 main 版（`git diff cc0bdfa832 911d795fde` 只差两行措辞：book.rs:321 行 Unmatched 闭集、Lean 行；main 版语义等价——未声明依赖的缺开局进 Unmatched、声明了依赖走等待；Lean 义务含配额界）。末尾「## #1406 已裁同步（2026-10-14，OQ-1 / OQ-3）」一节来自栈侧并保留（`git diff df6e680930 911d795fde` 只增此节 4 行），与 main 表格无矛盾。

## 四、丢弃 d6b842aef5（q_content / q_snapshot 整份取 main）

- 栈上这两个文件自 merge-base 起只有 d6b842aef5 一个提交；main 上只有 eb578149fb 一个提交（把测试 `job()` 的 `max_elapsed_ms` 从 20000 改为 600000，注 TestOnly）。合并后两文件 job() 为 600000（q_content.rs:476、q_snapshot.rs:503）。
- 覆盖等价性：d6b842aef5 的 `functional_job()` 也把期限设为 `FUNCTIONAL_TEST_ONLY_DEADLINE_MS`=600000（q_resources.rs:162），并在其注释自承「捕获仍会把 job 的绝对 Instant 交给 SQLite 的真实墙钟，因此同时沿用功能测试长期限」——即消除 runner 超期的承重件本来就是 600000 长期限，受控 tick 只是额外固定计时。main 版在 10 分钟内不会因 runner 速度超期，覆盖等价。两项跨页测试在合并提交上实跑通过（第六节）。
- 负控 `functional_job_clock_preserves_limits_and_rejects_capture_at_deadline`：其中一半是校验自身辅助函数（期限换值、deadline 区间），随辅助函数一起丢弃无损；另一半「到期时 `capture_configured` 对缺失源先判 `job_deadline`、请求计数不增」已由 capture.rs:2255 `configured_capture_retries_count_and_absolute_job_deadline_precedes_io` 覆盖（同一 `/deliberately-absent-q3-source.sqlite`、注入时钟跳到期限、断言 `job_deadline` 且 `requests` 不变，另加 `CaptureDeadline` 断言），两侧该文件相同且实跑通过。⟹ 未丢掉 main 没有的有效负控。
- 丢弃后 `FUNCTIONAL_TEST_ONLY_DEADLINE_MS` / `functional_resources` 仍在 q_resources.rs 内被使用，未造成悬空引用（编译通过）。

## 五、finance_tests.rs 的 build_slots_config 合成

逐段对比 `git diff df6e680930 911d795fde` 与 `git diff cc0bdfa832 911d795fde`：

- 对 main：只新增 C4a 内容——`c4a_doc` 字段、`build_config`、C4a 事件改写（side long、stable_point、evidence 片段）、`fill_manifest` 与 `c4a_doc` 构造、`book_revision`/`fill_manifests`/`fill_allocation_policy`/`ResolveEvidence` 授权、四个 c4a 时钟（85/88|1100/1100|90/1200）、以及两条 c4a 测试与辅助函数。与栈原版逐值一致（本段在 stack→merge 差异中没有任何行变动）。main 的 `two_slots`、`build_slots`、`slot_of`、多 slot `slots` 数组、`Reserve-slot-b` 时钟 93/94/94（仅 `slot_spec.len()>1`）、`request()` 按 slot_id 取 applicants、`witness_for`、`step()` 的 Reserve 分派，以及 3 条 #1394 测试，在 main→merge 差异中零变动。
- 对栈：只新增 main 内容；`build`/`build_config` 改为委托 `build_slots_config(…, &[("slot", applicants)], …)`，单 slot 结果与栈原版相同（slot_id "slot"、applicants 原列表、request_hash 同算法）。
- 唯一行为替换：栈原 `witness()` 用 `self.applicants` 生成 Reserve 证明，合并后改用 main 的 `witness_for(op,"slot")`，取 `data.slots[slot_id=="slot"].applicants`。c4a 测试都是单 slot，二者列表相同，证明集合等价。
- 时钟键不冲突：c4a 的 `Reserve-slot`（只在 c4a 夹具中覆盖）与 main 的 `Reserve-slot-b`（只在双 slot 夹具）互不触及。
- 无断言被改：两侧各自测试函数体在对方→merge 的差异里均未出现修改行。

**LOW-1**：`Fixture.applicants`（finance_tests.rs:12，初始化于 :281）在合并后已无读者——main 用 `witness_for` 取代了它，栈的读点随栈版 `witness()` 一起被替换。编译实测出现新告警 `field 'applicants' is never read --> src/economic_session/finance_tests.rs:12:5`（nc-remote log `/tmp/nc-remote/20261020-231217-911d795fde.log:4062`）。CI 未开 `-D warnings`，不阻断；建议删去字段及 :281 的初始化，使夹具与 main 的单一事实源一致。

## 六、编译与测试（nc-remote，NC_TARGET=target-1406-review，sha 911d795fde）

- `cargo check --features s_session --lib --bins --tests` → `EXIT=0`（log `/tmp/nc-remote/20261020-231217-911d795fde.log`）；告警中与本合并相关的仅 LOW-1。
- `cargo test --features s_session --lib economic_session` → `155 passed; 0 failed; 8 ignored`（log `/tmp/nc-remote/20261020-231310-911d795fde.log`）。其中两侧新增的 `c4a_fin1_reserve_and_resolve_both_orders_survive_restart_without_releasing_responsibility`、`c4a_fin1_uncovered_fact_cannot_reserve_or_resolve_and_keeps_all_state`、`commit_before_e_reserve_is_rejected_and_keeps_prepared`、`commit_while_slot_awaits_policy_is_rejected_without_winner`、`waiting_slot_blocks_other_slot_reserve_without_queue_jump` 全部 ok；8 个 ignored 均为需外部载体/夹具的既有忽略项。
- `cargo test --features s_session --bin s_structure_session -- v3::q_content v3::q_snapshot v3::capture::tests::configured_capture` → `10 passed; 0 failed`（log `/tmp/nc-remote/20261020-231614-911d795fde.log`），含两项跨页测试 `projection_continuation_after_source_commit_carries_independent_proof_and_old_anchor`、`real_pages_rebuild_old_cut_after_commit_and_restart_preserving_issued_digest` 与负控 `configured_capture_retries_count_and_absolute_job_deadline_precedes_io`。

## 发现汇总

- HIGH：无。
- MEDIUM：无。
- LOW-1：finance_tests.rs:12 `applicants` 字段成为死字段（新增编译告警），建议删除。
- INFO-1：ADR 0032 取 main 版后，栈版修订节里的穷举证据数与「Alabama 悖论」称谓不在正本中；属论证材料，非已裁条文。若希望正本保留可复核证据，可在 1350 已裁同步节或票内补一句出处，不阻断合入。

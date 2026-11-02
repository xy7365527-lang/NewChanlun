# 收据 workers/recover-independent-review-0043/REVIEW.md

- 原件：`workers/recover-independent-review-0043/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`9028ca91da9ed17f8c98e4d158f571793855cfa62c3ef80b1fa61686d7a4c8e7`
- 字节数：20316；行数：134

## 关键行（行号: 原文）

```
1: # #1488 复审 0043：Recover 绑定前进（0041）+ write_budget 夹具调查与正控（0039）
5: - 被审差异：`git diff 280437d999 38dd2ce493 -- rust/ .github/`，共 5 个文件，+216/-5。包含 0039 段（`280437d999..f23344909e`：1 个诊断/revert 对、2 个测试提交、2 个负控/revert 对，净写集为 driver.rs 加一条测试和锁清单加 2 行）和 0041 段（实施提交 `a6fead42c6`，加 NA、NB 两个负控/revert 对）。`git diff a6fead42c6 38dd2ce493` 为空。
6: - 依据：0036 评审项 1–7（对本 HEAD 重做；CI 用 39359439955，宽对拍用 20261020-102638）、0040 REVIEW/RESULT、0041 REPORT/NOTES、0039 REPORT/NOTES、主控裁定（#1488：0040 P2-1 取 (a)；0039 write_budget 取 (i)）。
7: - 本轮运行：没有跑 cargo，也没有重跑负控。重新下载了 CI artifact 并核对 digest；解析了 /tmp/nc-remote 中的负控、诊断、全锁和宽对拍日志；其余结论来自静态阅读。
9: **总判定：PASS_WITH_FINDINGS**（P1 0 条；P2 1 条（新，0039 正控断言）；P3 4 条（新））。0040 的 P2-1 判 **RESOLVED**。
15: ## 0. 重点 (1)：0040 P2-1 是否消除
25: **结论：0040 P2-1 判 RESOLVED。** 0040 反例原序列（Begin21 → 同 owner 新请求 ready → Recover）现在由 `recover_lock_promotes_after_new_request_readonly_rebind`（`driver.rs:1675`）锁为成功，并核持久 writer_epoch=2；他账段、伪造摘要锁（`recover_lock_rejects_foreign_ledger_budget_handoff`）、绕过重绑点段都仍然拒绝，符合裁定 (a)。受限之处见 P3-1、P3-2。
27: ## 1. 合同各条款（对本 HEAD 重做）
31: | 1.1 | 同 owner 成功 Recover 保留资格 | **PASS**（0040 为 PARTIAL） | 中间夹一次新请求只读 admission 的序列已能迁移（新正控锁，以及 0037 锁 2a 段）。receipt 和已提交重复 ingest 走同一分支，但没有单独的锁（P3-2）。 |
32: | 1.2 | 显式携带冻结预算 | PASS | 交接摘要门 `construction_state.rs:566-571` 与记录门 `:572-578` 都保留，后者拆成“缺记录 / 不符”两种文案（不符的文案逐字不变，门序不变）。`recover()` 仍为 `bind_existing(None)`，不重绑。 |
33: | 1.3 | 原料连续证明 | PASS | `Self::carry` 未改（`:582`）。 |
34: | 1.4 | 只允许原有效 owner | PASS | 未改。公开 `driver::recover` 每次新开连接，仍不会进入迁移。 |
35: | 1.5–1.8 | epoch 严格上升 / effects 闭合 / 真实 COMMIT 后晋升 / 旧 epoch、token、租约失效 | PASS | `commit_as`、`confirm_recover`、`check_commit`、`startup` 均不在 hunk 内（audit_owner.rs 的 hunk 只有 `@@1459/1464/1467`，都是测试 helper，以及 `@@1823` 推进点）。 |
36: | 1.9 | 外写、读域、rollback、提交失败、交接失败撤销 | PASS | 新的“缺记录”出口同样返回 Err，由外层 invalidate；新锁断言 `assert_recover_revoked`（`driver.rs:1703`）。 |
38: ## 2. 禁止项与触发条件
42: - **PASS。**
44: ## 3. 原锁与断言
47: - 全 diff 的 `-` 行只有：2 行 doc 注释、1 行测试 helper `handoff(u64::MAX, 0)`（P3-1 正额度改写）、1 行被拆分的记录门。**没有删除或修改任何 assert。** 0037 锁 2b 段的原拒绝期望与 `assert_recover_revoked` 原样保留；在它之前插入的 2a 段属于裁定 (a) 明确授权的“第二段改为合法换域成功 / 绕过则拒”。
49: - **PASS。**
51: ## 4. 预算段冻结不变量
54: - `request_audit.rs`、`state.rs`、`effects.rs`、Cargo.toml/lock 均无 diff。CI feature 组 78 PASS / 1 FAIL，唯一失败是 write_budget（已知，0042 改写）。P1-α、P1-1/2、P3-d、P3g 等预算段锁全部 PASS。
55: - **PASS。**
57: ## 5. 新锁与负控
61: | NA `a3def5a726` | 只删推进点 3 行 | `809c670a3a` ✓ | `20261020-102027-a3def5a726.log` | 2/2 FAILED：`driver.rs:1687`、`:1645`（见 §0） |
68: - **PASS**（受限部分见 P3-1）。
70: ## 6. 业务语义、SQL/错误顺序、资源帽、Cargo/features、函数体
75: - **PASS。**
77: ## 7. 测试证据
79: - CI run 39359439955：API head_sha=`38dd2ce49337474c6e8eeb088a30e8852561bd22`，workflow `issue-1488-uo-iter.yml`，分支 `codex/1488-uo-ci-20261019`，event=push，conclusion=failure。两个 job 的 head_sha 相同：`uo (s_session)` success，`uo (s_session,s_capture_native_testonly)` failure。
80: - artifact 重新下载：12236622871 sha256=`b0f8627a…2baab`，12236802001 sha256=`711a4879…bc94`，都等于 API digest；解压后与 `workers/uo-ci-runs/39359439955/` 逐文件 `diff -rq` 一致。
81: - feature 组 locks.tsv **78 PASS / 1 FAIL(101)=write_budget**（left 3602192）；79 个锁名与锁清单去注释后完全一致；18 条 recover_lock 和 0039 正控全部 PASS。s_session 组 1 PASS / 78 NOT_LISTED。
82: - 快通道全锁 `20261020-102422-38dd2ce493.log`：78 ok / 1 FAILED（write_budget，left 3602193）。
84: - **PASS。**
86: ## 8. 对 0040 各 finding 的处置
90: | P2-1 精确冻结域比对让中间夹只读新请求的 Recover 被拒 | **RESOLVED** | §0。按裁定 (a) 实现；正控 + NA/NB 负控真实。 |
97: ## 9. 重点 (2)：0039 推导、余量扫描、raw 残留与正控
103: | 写成功下限的机理说明 | **不成立（数值反例）** | REPORT §2.1 写道“写成功须 parent_limit 装得下 handoff_peak 8,181,876，其中新增部分为 5,308,141”。扫描本身否定了这两种读法：按“装下整个 peak”，7 MiB（parent_limit=7,340,032 < 8,181,876）应失败，实际成功；按“装下新增部分”，6 MiB（6,291,456 > 5,308,141）应成功，实际失败。6–7 MiB 的门槛来自别的门（stage 材料 cut / 材料溢出帽），不是 handoff_peak。这个错误不影响“==0 
105: | raw 残留结论 | 成立 | `complete_retained_bytes = charge.total + metadata + construction.retained_bytes()`（`audit_owner.rs:1279-1293`）。规则释放候选时，只有 cap < raw 才会 `invalidate_construction`（`:865-875`）。neg2 日志：候选被释放后原锁 left=320877，正好等于 raw。 |
106: | 正控断言与负控是否真测到 | 部分成立 | `!report.is_null()`（`driver.rs:2111`）加三个 effects 相位是实质断言：neg2 红在目标断言 `:2047`（当时行号），日志真实。neg1 红在写本身，不是目标断言（已披露）。`peak <= parent_limit`（`:2116-2117`）不是生产不变量（P2-1 新）；`charge <= cap - raw`（`:2118-2120`）是在测试内用公式重算，复述保留规则（P3-3）。 |
110: ## Findings
112: ### P2-1（新）0039 正控的 `handoff_peak ≤ total − entry` 不是生产不变量，而裁定 (i) 正要把它写进改写后的锁
114: - 位置：`driver.rs:2113-2117`（`parent_limit = total − entry_reserved_bytes`；`assert!(peak <= parent_limit)`）；`handoff_peak = entry_reserved_bytes + request_peak`（`audit_owner.rs:2860-2863`、`:2405`、`:2423-2425`）；REPORT §2.1、§3；主控裁定 (i) 原文“原锁…改写为已补正控（effects 路径写成功、保留量 ≤ cap−raw、handoff_peak ≤ parent_limi
120: - 为何不判 P1：不影响生产代码、不错账；“==0 不可达”和原锁改写的方向不受影响。
123: ### P3
128: - **P3-4（新）REPORT §2.1 把“effects 写成功则必保留”表述为一般结论。** 不等式链中“> peak ≥ charge”一步没有论证，实际依赖本夹具 charge + raw_new（3,602,063）远小于 peak（8,181,876）。对“本锁 ==0 不可达”的结论没有影响（有扫描支撑），但不应引用为保留规则的一般性质。
130: ## 未覆盖与限制
132: - 本机没有运行 cargo，没有重跑 CI，也没有重跑负控。P2-1 的反例取自作者的诊断日志读数；P3-1 的“变异仍绿”是静态推断，没有实际运行。
134: - 0035 journal 段不在本轮范围。#1488 仍 blocked_by #1394，本评审不解除任何门。
```

# 收据 workers/recover-independent-review-0040/REVIEW.md

- 原件：`workers/recover-independent-review-0040/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`c511d92de3420d92d469a25e6bc7a7c4eba9d291a248c6873d667007bf528ba7`
- 字节数：16815；行数：137

## 关键行（行号: 原文）

```
1: # #1488 Recover 绑定返修复审（0040）
5: - 被审差异：`git diff cc9fe7e1d6 280437d999 -- rust/ .github/`，共 4 个文件，+151/-3，7 个提交：实施提交 `27ff299759`，加 3 对负控/revert。`git diff 27ff299759 280437d999` 为空。
6: - 依据：control/1488-recover-review-0036.md 的评审项 1–7（对本 HEAD 重做；CI 换成 39352182056，宽对拍换成 20261020-090709）；0036 REVIEW/RESULT；0037 REPORT/NOTES；主控裁定 issuecomment-6278995984 第一条。
7: - 本轮运行：没有跑 cargo，也没有重跑负控。重新下载了 CI artifact 并核对 digest，其余结论来自日志和静态阅读。
9: **总判定：PASS_WITH_FINDINGS**（P1 0 条，P2 1 条（新），P3 4 条（新）；0036 的 P2-1 判为 RESOLVED）
15: ## 0. 重点：0036 P2-1（父账绑定自指）是否消除
25: **结论：0036 P2-1 判 RESOLVED。** 不过，精确冻结域比对带来了新的可用性代价，见 P2-1（新）。
27: ## 1. 合同各条款（对本 HEAD 重做）
31: | 1.1 | 同 owner 成功 Recover 保留资格 | **PARTIAL（见 P2-1 新）** | 01 锁和 14 条原 recover_lock 在 CI 与宽对拍中均为 PASS。但只要最后一次构造之后、Recover 之前有一次**新请求**的只读 admission（ready/receipt/重复 ingest），候选就会被重绑到新冻结域，Recover 随即被拒并撤销资格。0036 下这条序列可以迁移。 |
32: | 1.2 | 显式携带冻结预算 | PASS | 交接摘要门原样保留（`construction_state.rs:543-548`）；新增记录值门（`:549-551`）提供独立来源。仍过原总容量门；`recover()` 仍用 `bind_existing(None)`，不重绑。 |
33: | 1.3 | 原料连续证明 | PASS | 门之后仍走 `Self::carry`（`:556`），未改。迁移后 `pending.parent = handoff.parent`（`:557`）：已核实它与记录值同账同域，晋升后的资格记录本事务父账。 |
34: | 1.4 | 只允许原有效 owner | PASS | 未改（`:536-541`）。公开 `driver::recover`（`driver.rs:446-459`）每次 `AuditConnection::open`，生产公开路径仍不会进入迁移。 |
35: | 1.5 | 持久 epoch 严格上升 | PASS | `audit_owner.rs:964-965` 未改。 |
36: | 1.6 | effects 精确闭合 | PASS | `confirm_recover` 与 `check_commit` 未改。 |
37: | 1.7 | 真实 COMMIT 后晋升 | PASS | `commit_as`（`:1081`）函数体不在任何 hunk 内。 |
38: | 1.8 | 旧 epoch/token/租约失效 | PASS | 未改；相关锁在 CI 中 PASS。 |
39: | 1.9 | 外写、读域、rollback、提交失败、交接失败撤销 | PASS | 新增的两道门都返回 Err，由 `before_fixed_write`（`:2932-2940`）外层 invalidate；两条新锁都断言 `assert_recover_revoked`。 |
41: ## 2. 禁止项与触发条件
45: - **PASS。**
47: ## 3. 原锁与断言
52: - **PASS。**
54: ## 4. 预算段冻结不变量
56: - CI feature job 的 locks.tsv 为 75 PASS / 1 FAIL。唯一的 FAIL 是 `write_budget…`（0035 段已知未闭合项，基线 39349246070 为 73P/1F，同一条）。P1-α、P1-1、P1-2、P3-d、P3g 等预算段锁全部 PASS。
58: - **PASS。**
60: ## 5. 新锁与负控
64: | N1 `176061bf5a` | 删除记录值门 3 行 | `62b7070e0d`，`rev^==neg`，`diff neg^ rev` 为空 | `20261020-085851-176061bf5a.log` | FAILED `driver.rs:1632`，left=“原行许可：原件未由本父账许可出生，不得追认” |
65: | N1b `dd84ddbd90` | 记录比对改为 `same_ledger_only_for_neg` | `1d0cf71895`，同上 | `20261020-090112-dd84ddbd90.log` | FAILED `:1644`，left=“完整保留费超过总容量帽” |
66: | N2 `0a59f45a6f` | 删除 holds 门 3 行 | `280437d999`，同上 | `20261020-090257-0a59f45a6f.log` | FAILED `:1677:14`，“unwrap_err() on an Ok value … epoch_transitioned:true”：他账许可被晋升 |
68: - 3 个 revert 都是 HEAD 的祖先，目标锁在 HEAD 上为 PASS（CI 与宽对拍）。
71: - **PASS**（受限部分见 P3-1 新）。
73: ## 6. 业务语义、SQL/错误顺序、资源帽、Cargo/features、函数体
78: - **PASS。**
80: ## 7. 测试证据
82: - CI run 39352182056：API 返回 head_sha=`280437d999f58522d9cebef9215182f51993b898`，workflow `issue-1488-uo-iter.yml`，分支 `codex/1488-uo-ci-20261019`，conclusion=failure。两个 job 的 head_sha 相同：`uo (s_session)` success，`uo (s_session,s_capture_native_testonly)` failure。
83: - artifact 重新下载：12232848950 sha256=`3e8c0f0c…`，12234650190 sha256=`ce053f76…`，都等于 API digest。解压后与 `workers/uo-ci-runs/39352182056/` 逐文件 `diff -rq` 一致。
84: - feature job 的 locks.tsv 为 **75 PASS / 1 FAIL(101)=write_budget**。76 条锁名与 `.github/1488-uo-locks.txt` 去注释后完全一致，16 条 recover_lock 全部 PASS。s_session job 为 1 PASS / 75 NOT_LISTED。小出入：CI 中 write_budget 的 left=3602192，REPORT 引用的是远端快通道的 3602194，环境差 2B，不影响结论。
86: - **PASS。**
88: ## 8. 对 0036 各 finding 的处置
92: | P2-1 父账绑定自指；holds 无负控 | **RESOLVED** | 见 §0。按建议 (a) 实现：提交时记 Weak<Ledger> 和冻结域；fee=None 也比对；在比对前注入他账的负控 N1 为红；holds 独立负控 N2 为红，且是真实绕过。副作用见 P2-1（新）。 |
99: ### 主控裁定 issuecomment-6278995984 第一条
105: | 仍过原总容量门、不重绑、不换账 | PASS |
110: ## Findings
112: ### P2-1（新）精确冻结域比对让中间夹一次只读新请求的同 owner Recover 被拒并撤销资格，新锁把这一行为锁成了期望
120: - 为何不判 P1：没有错账或越权后果，生产公开路径不可达；作者已在 REPORT §6 把“精确相等 vs 放宽”交主控裁定，只是低估了触发频度。
121: - 建议（交主控裁定，三选一）：
126: ### P3
128: - **P3-1（新）N1b/域段锁受 allowance=0 影响**：`advance_complete_domain_for_test` 用 `handoff(u64::MAX, 0)`，没有新门时会被“完整保留费超过总容量帽”拦下（N1b 日志）。该段只锁住了“哪道门拒绝”，不能证明正常额度下域不符会被接纳。若采纳 P2-1 的 (a) 或 (b)，应改用正额度的真实换域来构造正控和负控。
131: - **P3-4（新）REPORT 数值小出入**：CI 中 write_budget left=3602192，REPORT §4 写的是远端 3602194（快通道和宽对拍）。不影响结论，仅供归档时更正口径。
133: ## 未覆盖与限制
135: - 本机没有运行 cargo test，没有重跑 CI，也没有重跑负控。P2-1（新）的反例是静态推导，未构造测试运行（只读约束：评审 checkout 不得提交）；旁证是新锁第二段使用的是同一个 `handoff` 原语。
136: - 0037 没有重做 0031/0034 的旧负控。本轮核对了这些守卫的代码在 0037 中未被改动，因此推断原结论仍适用。
137: - 没有覆盖 0035 journal 段（由独审 0038 另审），也没有覆盖裁定的第二、三条。#1488 仍 blocked_by #1394，本评审不解除任何门。
```

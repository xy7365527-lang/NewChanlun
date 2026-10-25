# 收据 workers/review-0045/REVIEW.md

- 原件：`workers/review-0045/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`6def69d53c3160497714569024ee2cef31b8f6ee6c0b3bae3e0b7390b56e0823`
- 字节数：18285；行数：176

## 关键行（行号: 原文）

```
1: # #1488 复审 0045：write_budget 改写（0042）+ 返修 0044 + 本段整体终审
7: - 依据：控制文 `control/1488-review-0045.md`；上一轮 `workers/recover-independent-review-0043/{REVIEW.md,RESULT.json}`；`workers/wb-rewrite-0042/REPORT.md`；`workers/fix-0044/{REPORT.md,NOTES.md}`；#1488 主控裁定（下文按 comment id 引用）。
9: ## 结论
11: **verdict：PASS_WITH_FINDINGS。P1 0，P2 0，新增 P3 4 条。**
13: 0043 的 P2-1 已解决：峰值断言改用生产实际比较的量，在 7MiB 和 8MiB 下都通过，旧公式在 7MiB 下按预期变红。历轮评审（0032/0036/0038/0040/0043）的 P1/P2 全部结清。
15: 本段删改的原断言只有四处，都有裁定：r14、fixed_radix、ordered_node_receipts 载体、write_budget。其他原锁未改。资源帽、`limit/8`、codec/SQL、首错与 COMMIT 顺序、Cargo/features、`startup` 和 `audit_with_budget` 的函数体都没动。两组 features 下非测试 `cargo check` 的 warning 集与 `62bad42a1b` 逐行相同。CI 39364508567 的两个 artifact 已重新下载，digest 一致，testonly 组 80/80 PASS。
17: ## 1. 本轮（0042 + 0044）
19: ### 1.1 0043 P2-1：已解决
30: ### 1.2 0043 P3-1~P3-4
39: ### 1.3 生产侧净改动（本轮）
50: ## 2. 整体终审（62bad42a1b..fff166540e）
52: ### 2.1 四处裁改断言清单
54: | # | 断言 | 提交 | 裁定 | 是否在本 diff 内 |
59: | 4 | write_budget：删除 `complete_retained_bytes()==0` 原锁 `audit_owner_write_budget_releases_candidate_after_successful_normal_append`，与 0039 正控合并，并改名为 `..._retains_candidate_within_retention_cap_and_parent_limit` | `f49db3d7e7`（0042，合并、改名、改锁清单）；`cc5e324736`（0044，峰值与保留断言改读生产值） | 6281614540 (i)。0044 的口
63: 其余 `-` 行都是生产代码：Recover 门、Journal key 合并、originals 增量许可，分别有裁定一、二、三及 0040 (a)（6281048348）覆盖。另外：
66: - 0037 锁第二段由 0041 改写，这是段内新增的锁，依据裁定 (a)，不属于原断言。
75: ### 2.2 帽、语义、顺序、构建
89: ### 2.3 测试证据
91: - **CI run 39364508567：** headSha `fff166540e355fe09c03ee441894b9e82265a59c`，push，conclusion=success；两个 job 都是 success。
92:   - artifact 12238563757（testonly 组）`sha256:75180ac3f426682af2ad19258616ab1fd13599d2fa11dec4b26f246916de3cc4`
93:   - artifact 12238592845（s_session 组）`sha256:fb22b5ce94b485f9220b8570ddba15453bfe7f7a39848431fb65f08ccb1aa185`
94:   - 两个 zip 已重新下载并计算 sha256，与 API digest 相符；解包后与 `workers/uo-ci-runs/39364508567/` 用 `diff -r` 比对一致。
95:   - testonly 组 locks.tsv 为 **80 PASS**，s_session 组为 1 PASS / 79 NOT_LISTED。build.log 两组都是 0 error，warning 296/255，与 0043 所记基线相同。
120: ## 3. 历轮 P1/P2 结清
124: | 0032 | P2-1 合同点名回归未单独覆盖 | RESOLVED（0034 补锁，0036 核过） |
125: | 0036 | P2-1 父账绑定自指，holds 门无负控 | RESOLVED（0037，0040 核过） |
126: | 0038 | P2-1 write_budget 未绿 | RESOLVED（裁定 (i) 改写，0042/0044 后 CI 绿） |
127: | 0040 | P2-1 精确冻结域比对误拒只读新请求后的 Recover | RESOLVED（0041 按 (a)，0043 核过） |
128: | 0043 | P2-1 handoff_peak ≤ total − entry 非生产不变量 | RESOLVED（本审 §1.1） |
130: 所有评审均无 P1。
132: ## 4. 本审新增 P3
140: - **P3-B：第四处裁改断言的最终形态与裁定 (i) 原文不同。** 原文是 “handoff_peak ≤ parent_limit、保留量 ≤ cap−raw”，现在是 `handoff_peak − entry ≤ candidate.limit` 加上生产 trace 版 `charge ≤ cap − raw`，按 0043 P2-1 和派发 6282365750 执行。生产字段本身也叫 `parent_limit: candidate.limit`（`audit_owner.rs:1648`），命名一致。建议在票上登记第四处时写明最终断言形态及其依据链。
141: - **P3-C：`.github/scripts/run-1488-driver-ci.py:30` 仍列着旧锁名 `audit_owner_write_budget_releases_candidate_after_successful_normal_append`。** 该 workflow 只在 `validate/1488-driver-six` 分支触发，且绑定了已审源哈希。锁清单“原六 driver”节现只剩五条。需要主控裁定这个文件是改名还是退役（0042 已披露）。
144: ## 5. P3 汇总登记表（整票验收前处置）
148: | R-01 | 0032 P3-1 | 预算隐式携带 | RESOLVED | 已转为裁定一，由 0034/0037/0041 闭合 |
165: | R-18 | 【05】/0042 | “8MiB 足够”未证明；新锁 eprintln 未采读数（需 `--nocapture`） | OPEN（开放风险） | 按裁定 (i) 随整票验收资源读数报告 |
169: | R-22 | 本审 P3-C | driver-six CI 脚本仍列旧名 | OPEN | 主控裁定改名还是退役 |
172: ## 6. 未覆盖
176: - #1488 仍然 blocked_by #1394，不属于本审范围。
```

你是 NewChanlun 仓库 PR #1663 的非作者第二轮独立审查者（只读，不改任何文件，不跑 cargo/pytest，不调用 gh）。工作目录是该 PR 分支 worktree。第一轮独审（`.chanlun/review-results/issue1604-20261021/review-claude-fable-5-1.md`）结论 PASS 但留了三条 MEDIUM（M1 S 写者 TB-02-C 预算戳与实际所读版本不一致、仍有 ROUND_BUDGET 常数；M2 Python v0 相切 `<`→`<=` 越出票面字面范围；M3 分类器未带段层记录戳、未消费尾窗截断标记）与 LOW L1（Python v1 TAIL_WINDOW 常数）、L5（提交编号声明）、L6（参考文档）。作者（另一模型 gpt 系，非你）针对它们新加了提交，本轮审查范围 = `git diff 35604dadf7..HEAD`（逐提交见 `git log 35604dadf7..HEAD`）。

票 #1604 共同验收要点：运行路径只读当前生效的版本化记录，每份输出写所用记录 id 与版本；生产/回放/准入路径无这些量的代码常数；缺记录/缺字段/非法值 ⟹ 显式报错停止，不得回退旧常数或备用值（#799）；负控测试各一例，改回常数的变异体须红；等值对拍输出逐位不变（不等处须逐条说明）。

请逐条核对：
1. M1：`rust/src/bin/s_session_v2/tb02c.rs`、`rust/src/theta_v0/parser/second_fold.rs` 等——输出的三处预算 policy 是否都来自本次所读记录（`ParseConfig::scan_budget`）；`ROUND_BUDGET`/`SECOND_SEQ_SCAN_BUDGET_POLICY` 是否全仓清干净；游标簿与段事实一致性检查、K1 线形跨版本拒读是否正确且无回退；第二序列 `tail_cache_window` 改读 `second_seq_window` 是否语义合理；`SecondFoldStore` 去掉 Default 后 `segment.rs`/`parser/mod.rs` 的归还路径是否引入行为变化；Python 合同 `s_session/s_tb02_contract.py` 与 `s_session/browser/q3-typed.js`、`tb01c-client.js`（后者为生成捆绑）的 v3→v1 / v4→v2 分档是否对称、是否算「宽严两档」违规（AGENTS.md #799 通则），历史兼容代价是否如实写明。
2. M2：读 `.chanlun/definitions/xianduan.md` §6.3（约 :256-266）、:386、§15 X-38（约 :524），判断 Python v0 改 closed 是否正本要求；新锁 `tests/test_segment_policy_1604.py::test_v0_signed_record_counts_tangency_as_overlap` 是否真的锁住 closed 行为（不打桩）；PR 描述草稿 `/tmp/fix1604/pr-body.md` 的影响面描述是否准确（若无法读该路径就略过）。
3. M3：`rust/src/theta_v0/classifier/pipeline.rs` 的 `TowerPolicyStamp` 新字段、`SegmentCoverage`/`segment_coverage` 语义（只标记不拒读）是否合理、是否静默；`rust/src/theta_v0/parser/tb1604_tests.rs` 新增负控/正控是否能抓住「恒 Complete」变异体；与 PR #1662（给 `TowerTermination` 加 `ParseInvalid`）的冲突面描述是否属实。
4. L1：`src/newchan/a_segment_v1.py` 尾窗读记录、`src/newchan/segment_policy.py` 新解析器口径是否与 Rust `parse_second_seq_scan_budget` 一致、1/2 值拒绝是否合理；`tests/test_segment_tail_window_scan.py` 改写是否仍测原意。
5. L5/L6：新提交是否都有编号声明且不写 fix/close/resolve；`docs/reference-theta-v0.md` 新增段与代码字段名是否一致；`.chanlun/review-results/issue1604/IMPLEMENTATION.md` §7 是否如实。
6. 新提交是否引入新问题（编译错误迹象、遗漏的调用点、测试与实现镜像、夸大结论）。

输出 Markdown：首行必须是 `结论：PASS` 或 `结论：BLOCK`；然后对三条 MEDIUM 各给「已处置/未处置」与理由；列新发现的 HIGH/MEDIUM/LOW（附文件:行号）；最后写明看过什么、未核实什么。不要写任何 IP 地址字面量。

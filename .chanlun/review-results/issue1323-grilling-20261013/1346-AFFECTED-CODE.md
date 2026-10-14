# #1346 受影响代码清单（Comparable／Sel_Θ／C(J,t) 适用域桥的实施缺口）

> 票：[#1346](https://github.com/xy7365527-lang/NewChanlun/issues/1346)（task 查证，非裁定）；正本：`qujiantao.md` §6.5 与 §9 第 46／49 条。
> 基线：**main `1f5cdf768da2d0b8dbde9ade8df930c2aaddd0a9`**。名分：工作草稿（宪法 §5），活期绑定本票；改码归「归属」列。

| 侧 | 文件:行 | 处置 | 归属 |
|---|---|---|---|
| rust | `rust/src/theta_v0/classifier/cand_predicate.rs:1-10/384-386/395-470` | 拆三条件元素谓词（Dir∧Comparable∧Extreme）为独立函数；Weak 作独立门 G；模块头与 docstring 由四条件改三条件 | TB-02-Q #1439 |
| rust | `rust/src/theta_v0/classifier/pipeline.rs:1684/1734-1750` | B2 背驰判据调用点改为『元素谓词 ∧ 力度门』两步同输入同输出（行为不变，对拍锁） | TB-02-Q #1439 |
| rust | `rust/src/theta_v0/backtest/econ_positive.rs:817-824/974` | acc_* 报告『四条件』自陈同改 | TB-02-Q #1439 |
| rust | `rust/src/theta_v0/classifier/nest.rs:978-985` | DFS 候选序改按 sel_order 降序（end 最新先），或先过滤后 select_best；对拍锁对 Lean selOrder 极值 | TB-02-Q #1439 |
| rust | `rust/src/theta_v0/classifier/nest.rs:1250-1258` | extend_upward 同向对齐 | TB-02-Q #1439 |
| rust | `rust/src/theta_v0/strategy/nest.rs:99-101` | chosen() 条件化于 J；或随 interp.rs:580-586 单元素链一并归 TB-10 退役 | TB-02-Q #1439 / TB-10 |
| rust | `rust/src/theta_v0/classifier/nest.rs:146` | 物化 C(J,t) 集合对象（members, cardinality）、selected_from 归因、readonly_all_candidate_result 只读轴 | TB-02-Q #1439 |
| rust | `rust/src/bin/s_session_v2/:—（新叶）` | S 发布 CandidateObservation（PREDICATES.md §6 形状）与 selected_from 边、selector_version | TB-02-Q #1439 |
| rust | `rust/src/theta_v0/backtest/admission.rs:500/1153` | strict_chain_verdict 不改；production_selected_result 携 predicate_id | TB-04 主动定位准入叶（#1362 下） |
| rust | `rust/src/theta_v0/classifier/level_view/pan_provider.rs:248-266` | 不改；生产三条件承重物 | 登记 |
| rust | `rust/src/theta_v0/classifier/nest_index.rs:272` | 索引快照携 selector_version，AsKnown 回放同版本 | TB-07 #1366 下叶 |
| lean | `formal/Origin/NestingCertificate.lean:239-240` | SelTheta 由 head? 改为 eligibleCandidates 上按 selOrder 取极值（或以『输入已按 selOrder 排序』为定理前件） | Lean 跟进（TB-02-Q 配套） |
| lean | `formal/Strict/Nest.lean:117-133` | selOrder / IsSelected 不改；作 I-2/I-5 的对拍基准 | — |
| lean | `formal/Origin/NestingCertificate.lean:151-153/232-259` | DivCand / eligibleCandidates / Cand / cand_false_implies_no_selection 不改 | — |
| python | `s_session/s_tb02_contract.py:890` | selected_from 已声明；加 CandidateObservation 校验 | TB-02-Q #1439 |
| python | `s_session/browser/tb01c-client.js:446` | 同上，前端只显示不判 | TB-02-Q #1439 |
| docs | `.chanlun/definitions/qujiantao.md` | §6.5 补同步段；§9 清单 46/49 落点转指 TB-02-Q；Changelog |  |
| docs | `.chanlun/review-results/issue1323-grilling-20261013/1346-*.md` | 四份副本 |  |

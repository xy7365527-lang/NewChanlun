# #1393 必要验证具名合同草案（未准入、返修后重绑）

状态：PREPARED_NOT_AUTHORIZED。当前被审产品source `c25f35f1dfe088affa07f18479f6736d069fc9a6` / tree `630f5cab59260448a46cc93edae23b5fc11a7bf1`；独审记录纯文档尾 `ab3f3fb0e1638f1ef3543b8b1948cd1856dc8675` / tree `3c383be7f45fe55d4a270ac3ef32a0b253de793b`。该产品有3项协议阻断，run-04正在返修，**不允许按本草案运行旧字节**。最终提交/tree、完整产品摘要、workflow/runner摘要、输入/规则、独审结论必须在根准入时重绑；#1419/#1488组合为另一份source，不继承本树通过。

## 可复用的现有入口与约束（只读核实，未执行）

- 本source `.github/workflows/ci.yml` 已支持 `workflow_dispatch`；push只对main、PR只对main，普通专用分支push不会自动运行这份CI。禁止为触发CI偷偷push/main或建PR；由根明确一次运输/dispatch后登记run ID，运输不明查远端/run，不盲目重发。
- `rust-check` job是ubuntu-latest，`timeout-minutes:60`、`CARGO_INCREMENTAL=0`、working-directory=rust；已有正式S测试 `--locked --features s_session --bin s_structure_session --jobs 1`，在always条件下运行；已有lib/backtest测试与default all-targets保持，不能为了1393绿删除或缩范围。
- `.github/scripts/run-cargo-with-telemetry.sh` 现有起始磁盘门为2147483648B、固定10秒采样、边界free/df/du，结束回收sampler并保真实命令退出；这不是运行全程硬磁盘帽、RSS帽或TERM/KILL监督，不能据此宣称有这些保障。
- 本树`theta_v0`默认lib可达；`backtest_bin`会拉nautilus可选套件，`s_session`启用sha2/rusqlite/libc。精确列名与执行的features必须一致，不以默认目标替代S bin。若根沿已有backtest_bin域验lib，则列名与运行都用该域，不混用两域计数。
- 现有fixture-drift job为ubuntu-latest，固定formal/lean-toolchain=v4.31.0，恢复elan后执行`lake build`与`python3 scripts/check_fixture_drift.py`。无显式job timeout，脚本自身也无timeout；默认build roots不含新增RawInclusionInitialization。因此仅运行既有fixture job不能证明新增定理已检验。
- #1488单次Native运行身份、两features、16/8MiB及其2370秒等窗口属于另一具名运行，**不授权1393借用或外推**。本机cargo/rustc/产品/Long/Lean构建禁令保持；本草案没有执行push、workflow_dispatch、产品或fixture脚本。

## 必需Rust清单与判据

先在最终源以相同target/features执行`-- --list`封全量stdout，确认以下7个新增测试和2个已有核回归实际存在且无同名歧义，不能用零匹配exit0过关。以列出的完整名称逐项`--exact --nocapture`运行，全部exit0、无ignored/skipped，断言原字节保留：

| target | source中实际短名（完整模块路径以最终二进制list为准） | 成功与拒绝义务 |
|---|---|---|
| lib | tb02u_independent_oracle_all_fields_and_first_pair | U01–U13中适用静态输入的全字段、真实首对、无伪方向 |
| lib | tb02u_batch_incremental_and_every_boundary_resume | batch/增量与每断点恢复同输出、单组DOWN恢复 |
| lib | tb02u_revision_retracts_and_reestablishes_without_mutating_prior_facts | 修订退回未定/再次建立、旧facts不改写 |
| lib | tb02u_exhaustive_small_raw_domain_keeps_established_geometry | 1555小序列已定几何保持、未定不偷产组 |
| lib | tb02u_parser_pending_activation_sparse_coords_and_resume | parser、新笔缓存解锁及稀疏坐标/序位 |
| s_structure_session / s_session | tb02u_old_cut_explicit_replaces_revision_and_recovery | 正式owner多对一replaces、旧cut/AsKnown、修订/恢复 |
| s_structure_session / s_session | tb02u_pending_append_keeps_first_known_and_policy_gate | 连续未定first_known保持，旧policy VersionMismatch拒绝 |
| lib | tb02a_trace_matches_existing_kernel_and_preserves_actual_roots | 已定方向域实际核/根逐位保持 |
| lib | tb02a_direction_is_from_actual_accumulator_not_previous_raw | 后续方向由actual acc/incoming确定 |

精确argv候选（最终域/名称核定后由具名runner执行，不是本轮授权）：lib使用`cargo test --locked --lib [根指定且全程一致的features] <完整测试名> --jobs 1 -- --exact --nocapture`；S使用`cargo test --locked --features s_session --bin s_structure_session <完整测试名> --jobs 1 -- --exact --nocapture`。经现有telemetry壳，不添加`--ignored`或弱化断言。逐项保build/list/test退出与stderr；编译失败则后续NOT_RUN，不把失败原因从旧run拼入本源。

## Python/JS、Lean与fixture漂移

- 返修后的`test_tb02u_contract.py`必须覆盖首独审3反例：合法重算内容hash后的未知/空/重复rule组合拒绝；initial_direction_evidence两端与确切raw修订的OHLC/坐标不符拒绝；JS typed+sources+identity联合校验活动/撤回未定raw及首对。合法W+已知base组合全部成功；不让hash错误替代规则或来源拒绝。
- 必要轻量回归复用当前5新合同/9既有合同的实际最终数量，并记录返修新增项；不将这些计数当Rust成功数。verify_model与verify_static原件可按产品同字节复用，产品或oracle变更则重核相应变化面。
- 新增Lean必须在远端预装工具链环境直接执行`cd formal && lake env lean Origin/RawInclusionInitialization.lean`并记录stdout/stderr/exit0，核6个具名定理和2个具体例；当前文件未进默认roots，未来持续验证须明确实际接线，不冒称已有默认build覆盖。不得在本机为此触发lake build。
- CLAUDE规定fixture drift必做，即使仅新增Lean未改fixture也不豁免：`python3 scripts/check_fixture_drift.py`。exit0无漂移；1真实字段漂移，原diff保留并归原义修复；2内部错误；3缺lake；4lake build失败；5导出器失败。后三项不是漂移也不是通过。不能手工改机器生成fixture修绿。

## 监督、封包和准入缺项

Rust可复用已存在60分钟job边界和2GiB起始门/10秒telemetry/单jobs=1；不扩帽，不把它说成OS内存/store硬帽。新增精确测试必须在既有CI语义保留前提下独立记录名称与结果；现有S全模块步骤会执行新测试，但无单独9项清单收据，因此需要有构建/list/精确运行原件，或从全模块完整stdout机械证明相同名单全部执行。

Lean/fixture当前job无明写监督截止，也没有新增Lean单文件入口。根在一次准入前必须指定可复用的已签监督边界与执行位置，或将这两步纳入已经有明确60分钟边界的具名验证任务；本草案不凭空新增数值上限、不改workflow。本缺项是准确运行合同仍待绑定，不是请求用户重裁W语义。

封包至少保：批准文本、精确source/tree与dirty状态、产品/输入/oracle/规则及工具链版本hash、runner/workflow hash、GH run/job IDs、build/list/各测试名/stdout/stderr/exit、Lean与fixture原始退出、telemetry全序列、超时/信号/进程回收、失败时NOT_RUN清单、ZIP/FILES摘要和独立验收。过期/失败/取消均保发生史，不自动rerun/dispatch，不拼不同source证明。

本合同仅覆盖#1393必要Rust/Lean/fixture与公共校验器验证。正式AcceptInput→Advance→API/Watch/AsKnown→真实浏览器、产品新进程/持久中断恢复仍要按本票U12/U13具名输入与真实监督另行接通；小模型不替代这些条款。#1393、TB父、SPEC、#1323和#1279仍不得据此关闭。

# PR #1697 `8ad401dc96` 非作者独立审查

- 候选：`8ad401dc967e7c0731432356a034eaa9bfc2896e`（复核时 PR head 相同）
- 固定点：`origin/main @ 3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 范围：完整 `git diff origin/main...HEAD`、#1670 原 AC、ADR 0032、生产调用链、Rust/Python oracle、后续证据提交与 CI。
- 方法：Standards / Spec 双轴独立审查后，主审复核关键行与证据；未修改候选，未在本机运行 cargo/rustc。

## 概念层质询结果

**通过。** 实现与 ADR 0032 裁定一一致：`projection_kind=per_leg` 只接受来源逐腿归属，缺归属是 Unmatched，不得运行时回落比例或 netted。冻结 manifest 的 hash、标的和参与重由共享 `tb03c_allocation` 路径绑定；成功路由不引入第二套分配政策。未发现与现有教义或资金合同的矛盾。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 1 | note |

**代码/合同结论：PASS-WITH-NITS。整票验收尚未闭合。**

### LOW：成功路由后的注释仍沿用旧合同

`rust/src/economic_session/tb03c_per_leg.rs:158` 写“路由证明不带数量或金额”，但成功路由已在 `:137-139` 写入 `units`、`acquisition_cash`、`commission`。建议改成“缺归属路由不得将未知分配伪装为零”。这是注释失实，不影响运行行为，不构成 HIGH 阻断。

## Standards

未发现仓库标准硬违规或需上报的 Fowler smell。三份既有冻结夹具零差异；新增 DevSkim 只追加精确 `rule|path|line` 键，没有目录排除或新增凭据；实施报告和工位登记均入 git。`git diff --check` 通过。独立复跑全部 allocation Python oracle **17/17 PASS**。

## Spec

- `tb03c_per_leg.rs:87-153` 精确覆盖冻结参与重，逐腿校验数量、单价、现金、佣金与总闭合；缺归属保持 Unmatched，不回落比例/netted。
- `tb03c_c4b_tests.rs:318-397` 通过正式 `Capture→Resolve→ApplyAllocation` 跑三重路由，核对 routes、数量、K、费用；同内容重送返回原收据，异内容 `IdentityConflict`，`store::recover` 后图像逐字一致。
- 缺失、未知重、数量/金额/费用不闭合、零量带费等负控保持 E/B 图像不变；`[2,1,0]` 与冻结请求量 `[1,3,3]` 的比例结果不同，比例变异会同时破坏 routes 与账簿断言。
- Python 独立 oracle 不调用生产 reducer；Rust 测试使用真实 RAW/HAND 和生产实现，不是 Python 自证。原 Unsupported 锁已改为成功路径，其余 C4b 测试未改。后续提交只增加实施记录、工位登记与精确 DevSkim 键，无生产行为变化。

## 验证证据

- 当前 head 的 Python：`17 passed`（本审查独立复跑）。
- nc-remote `/tmp/nc-remote/20261024-172025-75e397d860.log`：`economic_session::tb03c_allocation_tests` **18 passed / 0 failed**。
- nc-remote `/tmp/nc-remote/20261024-160545-fed87e5e5e.log`：per_leg 三项 **3 passed / 0 failed**。
- CI `40084137026`：per_leg 三项均通过；其余 rust-check、default、shard 0–3、coverage、DevSkim 等为 SUCCESS。
- 同一 CI extras：**223 passed / 3 failed / 9 ignored**。失败仅为 `each_signed_record_window_gates_alone`、`opening_batch_negative_controls_are_typed_and_deterministic`、`v2_domain_binds_revision_waits_in_e_and_replays`。
- main 基线 run `40005878482` 的 extras 为相同三项失败；这些归 #1621 主线时钟回归，不是本候选 per_leg 缺陷。

## 剩余门

票面“CI 全绿”仍未满足。须待 #1621 主线修复合入后重跑候选适用 CI，并核 main 合入与关票门；本报告不授权合入或关闭 #1670。若之后再改代码（含选择处理 LOW 注释），须把尾部 diff 纳入复审。

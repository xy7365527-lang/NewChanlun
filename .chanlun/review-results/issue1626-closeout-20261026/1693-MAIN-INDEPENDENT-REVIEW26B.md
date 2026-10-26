现在生成完整审查报告。我已充分核查关键冲突解决、自动合并交集、行为与格式验证、以及两新增交叉回归的覆盖范围。

# PR#1693 第二轮 main 集成独立审查报告

## 审查结论：PASS

## 审查基本信息

- **最终候选头**：`2fee8af14fdcdb317024b0a201bb36ea85ee3063`
- **父一（旧候选）**：`9f13b8efac9231eadf5ff755ca3943b21d8200a2`
- **父二（main）**：`40f11b540f02666a854bcf48e5ee259ff88af8bb`
- **merge-base**：`9b66f8a7acc85402f15fc9918628a86439d7ddb3`（已实际验证）
- **PR 远端分支**：`sandcastle/issue-1626`
- **审查时间**：2026-10-26
- **审查者模型**：Claude Opus（非作者模型家族）
- **审查模式**：只读独立审查，未修改任何文件、Git 状态或执行编译

## 冲突解决审查

### 一、两处手工冲突

**1. `rust/src/theta_v0/policy_records.rs`（已验证保留两父意图）**

最终候选在 `2fee8af14f:rust/src/theta_v0/policy_records.rs` 同时保留：

- **旧候选侧（#1626）**：`TREND_SCOPE_ID`（行41）、`TOWER_BASE_ID`（行43）两份记录常量及其 `ACTIVE_*_JSON` include（行65、67）和解析函数（行469起、行511起）。
- **main 侧（#1601）**：`ARM_SUB_EXPIRY_ID`（行69）、`ACTIVE_ARM_SUB_EXPIRY_JSON`（行71）及其解析函数（行328起：`parse_arm_sub_expiry`）和夹具入口（行349起：`active_arm_sub_expiry`）。

冲突解法符合原件声明「保留 trend-scope、tower-base 两份记录常量，同时保留 main 的 arm-sub-expiry id 和 include。没有二选一丢失记录」。三份记录常量互不覆盖，形成并集。

**2. `rust/src/trading/positional.rs`（已验证保留两父行为意图）**

最终候选在 `2fee8af14f:rust/src/trading/positional.rs` 同时保留：

- **main 侧资金记录接线**：公开入口 `run_positional` 签名（行1073）接受 `rec: &ReplayRecords`（行1077）；结果 `PositionalResult` 携带 `records: String`（行116）；测试夹具入口 `run_positional_fx` 传入 `ReplayRecords::legacy_parity_fixture()`（行1091–1093）。
- **旧候选侧趋势记录接线**：内部 helper `run_positional_with_trend_scope_record` 接受 `rec: &ReplayRecords` 和 `trend_scope_record: Option<&str>`（行1101–1106），从 `rec` 提取 `q_total` 和 `op_ladder`（行1108–1109），传入 Fusion 路径（行1199起）；Fusion 结果的 `trend_scope_policy` 字段写回 `policy.record`（行1218）；结构 `PositionalResult` 声明 `trend_scope_policy: Option<PolicyRef>`（行112）。
- **记录继续传入原 helper**：公开入口 `run_positional` 调用 helper 时传入 `rec` 和 `None`（行1079），helper 从 `rec` 读资金与操作层（行1108–1109）并传给 Fusion（行1199–1217）；Fusion 使用经原 profile 校验的趋势值（行1192–1196：`parse_trend_scope_for_profile`），写回 `trend_scope_policy`（行1218）。

冲突解法符合原件声明「保留 main 新入口的 `&ReplayRecords`、结果 `records` 标签及测试夹具入口。将记录继续传入原 trend-scope helper，由其读取 `q_total` 与 `op_ladder`；Fusion 使用经原 profile 校验的趋势值，并写回 `trend_scope_policy`。没有重引生产资金或操作层默认常量」。代码路径实际从 `rec` 读取，未见硬编码默认值。

### 二、写集交集的自动合并（五处已核）

merge-base 实际为 `9b66f8a7acc85402f15fc9918628a86439d7ddb3`（已验证），两父写集交集为：

1. **`docs/adr/0035-cost-stop-policy-instance-and-base-extension-evidence.md`**：最终候选相对 main 只保留 #1626 的 `tower_base_ref` 段（行12–16）；main 新 Q6 方法（行28–38：`share_threshold` 占比阈值、block bootstrap 算法、θ₂ 签署要求）未被 #1626 覆盖，完整保留。原件声明「main 新 Q6 方法未覆盖」得到确认。
   
2. **`rust/src/lib.rs`**：最终候选 `2fee8af14f:rust/src/lib.rs` 同时保留 main 的 `funding_profile` 形参（行1968、2140、2209）、`rec = py_replay_records(funding_profile)`（行1976、2214）、输出 `records` 和 `q_total`（行1981–1982、2218–2219）；以及 #1626 的 `trend_scope_policy` 输出（行2270–2271：`res.trend_scope_policy.map(|record| record.label())`）。原件声明「同时保留 main 的 funding_profile/records/q_total 和 #1626 的 trend_scope_policy 输出」得到确认。

3. **`rust/src/theta_v0/policy_records.rs`**：已在手工冲突部分核实，同时保留三份记录常量（trend-scope/tower-base/#1626 + arm-sub-expiry/main）。

4. **`rust/src/trading/positional.rs`**：已在手工冲突部分核实，同时保留 main 的资金记录接线与 #1626 的趋势记录输出。

5. **`rust/src/trading/positional_fusion.rs`**：最终候选保留 main 的 `op_ladder/q_total` 形参（行363–364：`run_fusion` 签名）及其边界检查（行374–377）、资金池初始化（行640）、级别下界计算（行1333）；以及 #1626 的趋势记录输出测试（行2365–2376：`trend_scope_policy_is_stamped_from_active_record`，断言 `trend_scope_policy.label() == "trend-scope/1"`）。原件声明「保留 main 的 `op_ladder/q_total` 形参、行为与 #1626 的趋势记录输出测试」得到确认。

### 三、不变文件核验

原件声明「旧候选至最终候选的 formal/、recursive_tower.rs、config.rs、trend-scope.json、tower-base.json 零差异」。已验证：

- `git diff --stat 9f13b8ef..2fee8af1 -- formal/ rust/src/trading/recursive_tower.rs rust/src/theta_v0/config.rs` 返回空（零差异）。
- `git diff --name-only 9f13b8ef..2fee8af1 | grep -E "trend-scope\.json|tower-base\.json"` 返回空（两份 JSON 记录未修改）。

既有 Lean 与塔底语义审查可复用。

## 新增交叉回归审查

原件声明两新增回归，已验证其在最终候选中的位置与行为覆盖：

**1. `fusion_preserves_funding_and_trend_scope_records_together`（行1950–1970）**

- **位置**：`2fee8af14f:rust/src/trading/positional.rs:1950`
- **行为**：对三个 Fusion profile（`fusion_t`、`hold26_anc`、`fusion_ta`）经公开 `run_positional` 入口验证：
  - 资金：`rec.funding.q_total = 500`（行1952），结果 `final_nav` 等于 `rec.q_total()`（行1962）。
  - 完整三记录标签：结果 `records` 等于 `rec.label()`（行1963），即 funding_profile + l_start + arm_sub_expiry 三份记录 id 与版本。
  - 趋势记录：`trend_scope_policy` 非空且 `label() == "trend-scope/1"`（行1964–1967）。
- **覆盖**：测试实际运行于行为日志 `20261026-103749-2554f669bd.log:28`（`ok` 状态）。

**2. `fusion_rejects_record_op_ladder_without_fixture_fallback`（行1973–1984）**

- **位置**：`2fee8af14f:rust/src/trading/positional.rs:1973`
- **行为**：实际签署记录 `ReplayRecords::active("P1-hl-testnet")`（行1974）给出 `op_ladder=2` 仍按既有 Fusion 边界拒绝（行374–377：`op_ladder ∈ [FIRST_BSP_LADDER+1, MAX_LADDER)` 即 `[3, 10)`），断言错误消息包含 `"op_ladder=2"`（行1983）。
- **覆盖**：测试实际运行于行为日志 `20261026-103749-2554f669bd.log:29`（`ok` 状态）。

两测试命中通过，验证资金与趋势记录组合接线、拒绝非法操作层记录而不偷换旧夹具常量的行为。

## 验证证据审查

### 一、行为验证（首轮，格式前候选）

**日志**：`1693-main-integration26b-validation/20261026-103749-2554f669bd.log`

**SHA 绑定**：日志首行 `sha=2554f669bd9923fbb4bc0fee53d86defaef04dd6`（行为 merge commit）

**远端环境**：nc-remote，target 复用（`/home/hanju/nc-remote/target`），避免 macOS cargo 干扰。

**命令**：`bash -lc rustfmt --check; cargo test positional::tests --jobs 1 --nocapture; cargo test funding_record:: --jobs 1 --nocapture; cargo test policy_records --jobs 1 --nocapture`

**结果**：
- `rustfmt --check` 报一处测试换行（行2–11：`positional.rs:1978` 的 `let err = ...` 断行）。该格式问题已由 `2fee8af14f` 单独修正（见下）。
- `positional` 20 passed（行22–44），含两新增测试行28、29。
- `funding_record` 6 passed（行49–57）。
- `policy_records` 12 passed（行63–76），含 main 新增的 `arm_sub_expiry` 测试行65。
- EXIT=0（行78），SECONDS=100。

### 二、格式修正与最终验证

**格式修正差异**：`git diff 2554f669bd 2fee8af14f` 仅修改 `positional.rs:1978–1982`，将测试中的 `let err = run_positional(...).unwrap_err();` 改为双行格式，完全纯格式，无行为变化（已逐字核查 diff 原件）。

**最终候选日志**：`1693-main-integration26b-validation/20261026-104044-2fee8af14f.log`

**SHA 绑定**：日志首行 `sha=2fee8af14fdcdb317024b0a201bb36ea85ee3063`

**命令**：`bash -lc set -e; rustfmt --check positional.rs policy_records.rs; cargo test positional_fusion::tests --jobs 1 --nocapture; cargo test no_stray_bsp_binding_rewrites_outside_registry --jobs 1 --nocapture; cargo check --lib --jobs 1`

**结果**：
- `rustfmt --check` 通过（行2–5，`set -e` 后继续执行）。
- `positional_fusion` 78 passed（行9–89）。
- BSP 白名单真实测试 1 passed（行94–97）。
- `cargo check --lib` 成功（行104–105）。
- EXIT=0（行106），SECONDS=106。

**证据复用**：首轮 38 项行为测试（positional 20 + funding_record 6 + policy_records 12）使用格式前候选 `2554f669bd`；最终候选只改一处断行，因此复用该组行为证据，不重复执行。最终候选独立执行 Fusion 78 项行为测试、BSP 白名单测试、cargo check，覆盖自动合并交集中的 `positional_fusion.rs` 与整体构建健康。

### 三、空白符净化

**Rust 净差异**：`git diff --check 40f11b540f 2fee8af14f -- rust/src` 无输出（通过，无尾空格或制表符问题）。

**历史归档**：整个 merge 相对父一的 `git diff --check` 会指出 main 已归档的 `atomic-init-tests-nocapture-73bbf098d9.log:6` 尾空格，该文件为历史原件，审查不改写原件凑绿，不计入告警。

## 未覆盖 CI 与限制

1. **GitHub CI 未完成**：PR head 更新时（`2026-10-26T02:43:55Z` 前回读）check 列表为空，`mergeable/mergeStateStatus` 均 UNKNOWN，属于刚推送后的状态。原件明确「回读发生在 ... 当时新 head check 列表为空 ... 不能称 CI 通过」。最终候选后续 CI 须主控另行消费。

2. **行为测试覆盖**：nc-remote 验证覆盖 positional 20 项（含两新增）、funding_record 6 项、policy_records 12 项（含 arm_sub_expiry）、positional_fusion 78 项（含 trend_scope_policy 测试）、BSP 白名单 1 项，共 117 项实际测试通过。不覆盖非 Rust 代码路径（Python bindings 的端到端集成测试、前端 #1460、非 backtest_bin feature 路径）。

3. **Lean 与形式化验证**：原件声明 formal/、recursive_tower.rs、config.rs 零差异，既有 Lean 审查可复用。本次独审未重新运行 Lean 4，信赖「零差异 ⇒ 既有审查仍适用」。

4. **只读独审限制**：审查严格遵守「不修改任何文件、Git 状态、tracker 或配置，不运行 cargo/rustc，不访问凭据，不启动子代理」。所有代码读取通过 `git show <sha>:<path>` 冻结对象完成，未读工作树。审查不授权合 main、改账本或关票。

## 行为与证据总结

**保留两父意图**：两处手工冲突均保留双方行为意图，未发明新的资金或趋势政策，符合 resolving-merge-conflicts 原则。自动合并交集五处均保留两父贡献。

**新增差异的回归覆盖**：两新增测试实际命中 main 引入的 `&ReplayRecords` 公开入口与 #1626 的 `trend_scope_policy` 输出，验证资金与趋势记录组合行为、拒绝非法操作层记录的边界。

**真实远端证据**：两份日志完整绑定 SHA、nc-remote 环境、命令、输出，已逐字保存到 `1693-main-integration26b-validation/`。行为验证使用真实 nc-remote 输出，非本地 macOS 编译；最终候选的格式、Fusion 行为、BSP 白名单、cargo check 均通过。

**Prove It Works**：集成采用 Prove It Works，核实际 merge 冲突、自动合并交集、真实 nc-remote 行为日志与远端 head，而不依据旧 CI 绿灯判定新集成完成。

---

**审查结论：PASS**。冲突解决保留两父意图，新增回归覆盖资金与趋势记录组合行为，真实远端验证通过 117 项测试（positional 20 + funding_record 6 + policy_records 12 + positional_fusion 78 + BSP 1），格式与空白符净化完成。未覆盖 GitHub CI、非 Rust 路径、Lean 重跑；已有 main 授权不替代本新增差异独审。候选可交主控后续消费 CI 与合并决策。

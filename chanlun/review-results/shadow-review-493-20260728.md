# #493 `wverify_run.rs` 拆分影子评审

- 被审 commit：`6a67e6840ec418e328d9b71790b0361e0a8363a5`
- 固定点：`5c58ec6a417c68caf9ba58698c7cdcebc07e2cf6`
- 评审时间：2026-07-28（America/New_York）
- 评审边界：只读；未改仓内文件、未执行 git mutation；跑批产物仅写 `/tmp/rev493_*`

## 结论

**打回（Standards：LOW×1；Spec：PASS；另登记非阻断边界债 LOW×1）。**

无 HIGH、无 MED。行为、bit-exact、公开路径、文件行数与三项机器硬门均通过；打回原因仅是清单明确要求的“`pub` 最小化”未完全满足：8 个只在本文件消费的 helper 被不必要地从拆前私有可见性提升为 `pub(super)`。

## Findings

### Standards LOW-1（打回）：8 个 helper 可见性扩大且非接线所需

证据：

- `report.rs:5,16,71,83,94,105`：
  `force_state_code`、`force_state_label`、`force_state_decode`、
  `exit_type_code`、`exit_type_decode`、`exit_type_label` 均为 `pub(super)`；
  对四文件及整个 `rust/src` 的逐名 `rg` 表明，定义外的所有消费点都仍在
  `report.rs` 本文件内。
- `m8.rs:218,265`：
  `apply_m8_fee_datum_from_env`、`apply_m8_level_cap_from_env` 均为
  `pub(super)`；消费点只在 `m8.rs:368,371,472,473`。
- 拆前上述 8 项均为裸 `fn` 私有项。移入子模块后它们无需跨到父模块，故不属于允许的
  “必要 `pub(super)`”；移除这 8 处 `pub(super)` 即可恢复最小可见性，不涉及行为变化。
- 其余跨模块调用项使用 `pub(super)` 有真实接线需要；`q4_margin_model` /
  `m6_cost_model` 保持原 `pub(crate)` 契约，不在本 finding 内。

影响：不扩大 crate 外 API，也未造成当前运行时缺陷；但扩大了父模块可见面，与本票核查清单
“被移项可见性未意外扩大（pub 最小化）”直接冲突，故评级 LOW、结论打回。

### 边界质量 LOW-2（非阻断登记）：模块职责仍有混合

- `report.rs:116-146,152-177,193-249,282-375` 还承载 σ̂ 计算、δ-free
  dump/load、统计 verdict，不全是渲染。
- `m8.rs:394-447,585-592,647-705` 仍内联构造四层表、结算节、活动集与 FeeAudit
  Markdown；`report.rs` 主要接走 cell/header/row/settlement helper。
- 这不违反“纯移动”硬门：`m8_e2e_all_systems_oos` 主体若再拆内部段落，需要新增参数/返回结构，
  已超出本票允许的纯移动范围。故作为后续模块边界债登记，不据此追加打回理由。
- 测试未错放：拆前 `mod tests` 的 32 个定点测试全部移至 `tests.rs`；根文件保留的 10 个
  `#[test]` 均是长跑/公开命令入口，其中两个是保持既有测试路径的 wrapper。

## Standards 轴证据

### 090 / 纯移动纪律

独立脚本 `/tmp/rev493_pure_move.py` 从
`git show 5c58ec6a41:rust/src/theta_v0/backtest/wverify_run.rs` 读取拆前单体，并对现四文件逐符号取
Rust 函数/常量/结构体文本。仅白名单归一化：

1. 子模块新增的一层 `super::` 相对路径；
2. 函数声明前新增的 `pub(super)`（提取从 `fn`/`const` 开始）；
3. 旧 `q4_fullpi_policy` / `m8_e2e_all_systems_oos` 对拍新
   `run_q4_fullpi_policy` / `run_m8_e2e_all_systems_oos`；
4. 两个原名 wrapper 作为新增接线单独列出。

重算结果：

- 函数：拆前 81，匹配 81，差异 0；
- 常量：拆前 9，匹配 9，差异 0；
- 结构体：拆前 1，匹配 1，差异 0；
- 新增函数恰为两个 wrapper：
  `wverify_run.rs:591-593` 与 `wverify_run.rs:693-695`。
- `#[test]` 总数拆前/拆后均为 42（根 10 + `tests.rs` 32）。
- `git diff --check 5c58ec6a41...HEAD` exit 0。

重点逐函数 SHA-256（对允许差异归一化后的完整声明+函数体）：

- `m8_e2e_all_systems_oos`：
  `296862b5a90da2aee1f475516820d5411b34e2b1c0735f6b8ae7e959c7d5903c`
- `layer4_cells`：
  `739b318953801130b0796799c3e50a8997d08286747b76e46c1acc0fc76539c2`
- `layer4_scalar_caliber_notice`（`CALIBRATED_UNAVAILABLE` 族实际落点）：
  `aecb4ebbfd5e597a1cbb70398691aa9dc97271603deb13e933c88659c62e7974`
- `format_m8_fee_audit_row`：
  `f329b07a11b9c55fec2471134b99ff3104d03b29134e90e34c9519189079ceb8`
- `deltafree_verdict`：
  `0fb60fa0ff3089e5dd3134ade3ac92fa59e7d66e5fb96ae328c11d8e1992b54a`
- `deltafree_verdict_key_is_delta_free_four_tuple`：
  `bfc9d0710f5df8283dc2c7cefa35c6b1c9cfd8dde270d25afa6f299a3b116625`
- `SCALAR_UNDEFINED_UNAVAILABLE`：
  `f4feb3e60cd766a33aae699d5d9cb6bbd5a6b2d54fdd6fd8f692a81333044b6c`
- `_DELTAFREE_KEY_SHAPE_FROZEN`：
  `43b64bfed823f2d4025e2366c734b7a42ab2cd2cc15654c77b61d9c579b8ec3e`

因此函数体内字符串、常量引用、断言、FeeAudit 渲染、δ-free 桶逻辑及定点测试断言均逐字保留。

### wrapper 透传

- `q4_fullpi_policy()`：零参数、单位返回，仅调用 `run_q4_fullpi_policy();`。
- `m8_e2e_all_systems_oos()`：零参数、单位返回，仅调用
  `run_m8_e2e_all_systems_oos();`。
- 两者保留原 cargo test 路径；本次 wf8 实际通过原路径命中 wrapper。直接删除 wrapper 会把测试路径
  改成 `wverify_run::m8::*`，故 wrapper 有兼容性必要，不作为 Middle Man smell。

### 通配引入

`wverify_run.rs:34` 的 `use report::*;` 当前无命名冲突：`cargo check --lib` exit 0，
且所有冲突都会在编译期 fail-loud。它扩大了父模块的候选名字集合，存在未来维护风险，但仓库未见禁止
私有子模块 glob 的成文规则；不另立 finding。

## Spec 轴逐项核销

### 三模块与 `<800`

独立 `wc -l`：

| 文件 | 行数 |
|---|---:|
| `wverify_run.rs` | 776 |
| `wverify_run/m8.rs` | 710 |
| `wverify_run/report.rs` | 660 |
| `wverify_run/tests.rs` | 680 |

四文件均严格 `<800`。拆前单体独立实测 2809 行；commit 描述“2809 → 776 + 三模块”准确。

### 接线、公开路径、循环依赖

- `wverify_run.rs:33-37` 声明 `report` / `m8`，并以
  `pub(crate) use m8::{m6_cost_model, q4_margin_model};` 保持旧路径。
- 原消费点仍按 `wverify_run::q4_margin_model/m6_cost_model` 命中：
  `runner.rs:5354-5355,5460,5497,5557-5558`。
- `cargo check --lib` exit 0；无循环依赖或解析歧义。
- 除 LOW-1 所列 8 项外，未见 crate 可见性扩大。

### bit-exact 三重硬门（全部独立复跑）

1. `cd rust && cargo test --lib`
   - 结果：**1992 passed / 1 failed / 135 ignored**；
   - 唯一失败：
     `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`；
   - 与票面预期精确一致，failed 集未扩大。

2. `python3 scripts/check_armR_trades_digest.py`
   - exit 0；
   - p3fold：141 trades，digest `0xac5952cfcd8b8746`；
   - wf7：184 trades，digest `0x981a0560b8db0b40`；
   - wf8：159 trades，digest `0x18291f8ba8f1d40f`。

3. BTC 臂R wf8 隔离跑批
   - 严格使用票面 env：
     `M8_WIN_FILTER=wf8 VOICE_EXEC=1 THETA_NEST_CERT_GATE=1
      M8_REPORT_PATH=/tmp/rev493_armR_wf8.md
      OPSEM_DUMP_DIR=/tmp/rev493_opsem_wf8`
   - 测试结果：1 passed，0 failed；264,960 bars；34.24s（不含编译）。
   - 报告 SHA-256：
     - `/tmp/rev493_armR_wf8.md`
       `851b8b8a654181dc6bb44a56cf0a703dd93de203bb902e093e66009ba4e7d85c`
     - `/tmp/accept493_armR_wf8.md`
       `851b8b8a654181dc6bb44a56cf0a703dd93de203bb902e093e66009ba4e7d85c`
   - dump：
     - `trades.jsonl`
       `abf8245ffd880c0f4488cd02f8696e950cd8349de15c705b52a2add3d679eaf8`
     - `tower_events.jsonl`
       `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f`
   - `/tmp/rev493_opsem_wf8` vs `/tmp/m8_win_gate/wf8`：
     `diff -qr` exit 0；两文件分别 `cmp` 同义为 SAME。

## 在案异常：`b2953a7f` / 842 单轨迹

commit resolution 声称：执行方自跑拆前/拆后受并行未提交 `nest_lifecycle.rs` WIP 污染；
该瞬态基线不构成拆分缺陷。独立复核得到：

- `/tmp/493_pre_armR_wf8.md` 与 `/tmp/493_post_armR_wf8.md` SHA-256 均为
  `b2953a7f4eb899ac4cd8864ead5a532962f085a29e803ae68ef9dec26222693a`，
  `diff -u` 零差异；这里的 `b2953a7f` 是产物哈希前缀，不是当前仓库可解析 commit。
- pre/post dump 也逐字相同：
  - trades：
    `006c31f54cd72d8ec9c9461122068faf58377cad2d176f839f6a6e0c5ff601b7`
  - tower events：
    `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f`
- 异常报告确有 `wf8 n_orders=842`；接受基线/本次独立重跑为 `n_orders=246`。
- 被审 diff 只触及 `wverify_run.rs` 及三个新子模块；逐函数 81/81 相等，
  且异常态下 pre/post 自身相等、当前态下本次产物又与接受基线相等。这两条独立状态内的
  before/after 链共同排除“拆分引入行为变化”。

结论：**未发现把 842 异常归为拆分缺陷的反证**。但瞬态未提交
`nest_lifecycle.rs` 编辑内容未保存为可读 diff，故“污染精确由该文件哪一行造成”的细粒度归因无法在
当前工位独立重演；本报告只确认“非拆分缺陷”，不把无法重演的瞬态文件级归因膨胀为已独立证明。

## 未覆盖 / 环境声明

- 未使用外网或 `gh`。
- codebase-memory-mcp 的 list/status/index 调用均被客户端取消，未取得图索引；按 AGENTS.md
  降级为只读 `git show`、`rg`、结构化逐符号脚本。
- 未重建已经消失的并行 WIP 未提交 diff；其精确文件级因果归因如上限缩。
- 未跑未要求的全量 release suite；已跑票面要求的 debug 全量、digest gate、release wf8。
- 开工与收尾均未触碰共享工位既有修改/未跟踪文件；评审报告仅写本 `/tmp` 文件。


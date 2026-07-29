# #533 修复报告：p123/m8 字节护栏仓内化——常驻回归门

## 结论

#429/#430 三审对 p123（stdout/`P421_LIFECYCLE_DUMP`）与 m8（`trades.jsonl`/`tower_events.jsonl`）
做过的逐字节 pre/post 对拍，已收成两个 `#[ignore]` 回归门，跑法与既有 `theta_v0_fixture_drift`/
`m8_e2e_all_systems_oos` 同惯例：

| 门 | 位置 | 覆盖 | 断言档 |
|---|---|---|---|
| p123 | `rust/tests/issue533_p123_byte_guardrail.rs`（新集成测试） | 2000/20000/100000 三个 `P116_MAX_BARS` 前缀窗口 | 2000 全文；20000/100000 SHA-256 |
| m8 | `rust/src/theta_v0/backtest/wverify_run.rs::m8_byte_guardrail`（新单元测试，同模块 `#[cfg(test)]`） | p3fold/wf7/wf8 三窗（对齐票面点名） | 全文（三窗产物均 <1MB，未触发哈希门槛） |

均已实跑验证：正常态两门全绿；分别人为篡改一处 golden 字节后复跑，两门均正确变红并给出可读
diff/哈希不等信息，随后已还原 golden、复跑确认恢复绿——证明断言真实生效，非重言式空测。

## 两档方案落地（票面 Acceptance 第 3 条）

- **小样本全文对拍**：p123 用 `P116_MAX_BARS=2000`（dump ≈450KB）；m8 三窗天然 <1MB（p3fold
  trades 647KB/tower_events 330KB，wf7 679KB/370KB，wf8 699KB/355KB）——均整份 check 进
  `rust/tests/fixtures/`，失败给出 `assert_eq!` 逐字段可读 diff。
- **大窗哈希断言**：p123 20000/100000 前缀 dump 分别 ≈5MB/≈25MB，check 全文不现实，只
  check SHA-256（`issue533_p123_{20000,100000}.sha256`，各含 `stdout`/`dump` 两行）——
  这一档口径在票内登记：哈希不等只能报告"变了"，不能报告"哪个字段变了"，需要时须回退到
  三审当年的人工 `cmp`/字段级比对流程定位。

## golden 值来源

在**当前 HEAD**（含本会话 #532 拆分改动，以及并行线已落地的 #571/#527/#591/#592 等修复）上
实跑生成，非沿用三审旧提交（`391936a486`）的哈希——三审基准与当前 HEAD 之间已有合规的行为
变化（如 #592 gap_len 诊断字段），旧哈希本就不该继续做断言基准。本票的 golden 是"从今往后
不再漂移"的新起点，不是"复现三审当天的数值"。

## 边界条件

- **CI 覆盖边界（票内登记，不算缺口）**：`analysis/data_cache/btc_1m_full.json` 被
  `.gitignore:82` 排除，CI checkout 不含该文件，因此本门物理上**不能**像 `fixture-drift`
  job 那样接入 `.github/workflows/ci.yml` 的 push/PR 触发——这与仓库内既有的
  `m8_e2e_all_systems_oos`/`wverify_full` 等真实行情 `#[ignore]` 测试面临完全相同的既有边界，
  不是本票新引入的缺口。本门满足票面"门挂接 cargo test **或** CI workflow"的前一选项：
  持有数据的机器（本地/自建 runner）用 `-- --ignored` 显式触发。
- **golden 变更纪律**（票面 Acceptance 第 2 条）：两个测试文件的头部文档均已写明——golden
  只允许因已审阅、故意的行为变化更新，且须在同一 PR 说明原因（引用 issue/report）；禁止为
  让测试变绿静默重跑重生成。
- m8 门把三窗放在**同一个** `#[ignore]` 测试函数里顺序跑（非三个并行 `#[test]`），因为
  `M8_WIN_FILTER` 走进程级 env、无线程局部注入机制（`OPSEM_DUMP_DIR_OVERRIDE` 已有此机制，
  `M8_WIN_FILTER` 没有）——三个独立 `#[test]` 在默认并行 `--ignored` 下会互相踩环境变量。
  这是刻意的设计选择，不是遗漏。
- 大窗哈希门（20000/100000）不覆盖字段级定位——已在票内登记为已知局限（见"两档方案落地"）。
- 本票不覆盖 p123/m8 之外的其它字节护栏面（若三审报告日后点出新的护栏缺口，需另行开票）。

## 影响声明

- 新增文件：
  - `rust/tests/issue533_p123_byte_guardrail.rs`（集成测试，2 个 `#[ignore]` 测试）
  - `rust/tests/fixtures/issue533_p123_2000_{stdout,dump}.golden.txt`
  - `rust/tests/fixtures/issue533_p123_{20000,100000}.sha256`
  - `rust/tests/fixtures/issue533_m8_{p3fold,wf7,wf8}_{trades,tower_events}.golden.jsonl`（6 个）
- 改动文件：`rust/src/theta_v0/backtest/wverify_run.rs`（+60/-0，纯新增：1 个 `#[ignore]` 测试
  函数 + 1 个断言辅助函数；未改动任何既有函数）。
- 未新增 Cargo 依赖：`sha2 = "0.10"` 已在 `rust/Cargo.toml` 的 `[dev-dependencies]` 中（供
  `venue_fee::datum_io` 既有测试使用），本票复用同一 dev-dependency。
- 验证：
  - `cargo test --release --test issue533_p123_byte_guardrail -- --ignored --nocapture`：
    `2 passed`。
  - `cargo test --release --lib theta_v0::backtest::wverify_run::m8_byte_guardrail --
    --ignored --nocapture`：`1 passed`（三窗顺序跑完，45s）。
  - `cargo build --all-targets`：绿。
  - `cargo test --lib -- --test-threads=1`（全量单线程，排除并行 flake）：`2034 passed / 1
    failed / 137 ignored`——`137` 较 #532 报告的 `136` 多 1，即本票新增的 `m8_byte_guardrail`；
    唯一失败仍是在册 `extract_signals_bit_exact_digest_guard`（#491），与本票无关。
  - `rustfmt --check`：新文件 `issue533_p123_byte_guardrail.rs` 干净；`wverify_run.rs` 内
    本票新增代码块干净（文件内其余既有 fmt 漂移非本票改动，未触碰）。
  - 红/绿双向验证：分别对 p123 小样本 golden、p123 大窗哈希 golden、m8 golden 各做一次
    人为篡改→复跑确认变红→还原→复跑确认变绿，逐一记录于本 session（未落盘中间篡改产物）。

## 认识论等级（`.claude/rules/formalization-validity-domain.md`）

L2：BTC 单标的（p123 三窗 + m8 三窗）、固定前缀/固定日期窗口的真实数据字节级断言；不外推
其它标的或窗口（非 L3）。golden 捕获本身（把当前正确行为固化为回归基线）是 L0 操作，不是
经验命题。

## 并行线说明

本会话工作期间，工位 `/tmp/kimi-nest-mainline` 有其它并行线持续提交（`chi-line-falsification-ruling`
档处置文档化，涉及 21 个文件的纯 doc-comment 追加 + `pi_bsp_timing.rs` 的 #563 M7 修复，均已核对
与本票触碰面 `p123_fast_replay.rs`/`wverify_run.rs`/`nest_lifecycle.rs`/`nest.rs` 无交集，未见冲突）。
本票未提交任何 git 操作（分支策略要求前台单线程、禁 git 写），改动与新增文件均待编排者验收后
统一处理。

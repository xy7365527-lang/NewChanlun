# #1294 对拍锁网：全仓判据同输入同输出对拍锁建设·第一批（#1278 批三，⑧列收口）

> 票：GitHub Issue #1294（[#1278](https://github.com/xy7365527-lang/NewChanlun/issues/1278) 批三基建）。
> 性质：**测试锁建设 + 清单成文**，零生产行为改动（新增代码全部 `#[cfg(test)]` / 集成测试 / Lean fixture 导出）。
> 基线：`sandcastle/issue-1294`（== `origin/main` @ `34613cbef3`）。
> 日期：2026-08-29。

## 0. 一句话结论

按验收顺序建成三把对拍锁（赋格族结构判据 / BSP 族 Lean↔Rust 向量级 / #804 恒等性），全部落在 `cargo test` 全量内（CI `rust-check` job 的 `cargo test --all-targets` / `cargo test --lib --features backtest_bin`）；每把锁带至少一组「历史分歧案例」回归向量。第 4 项（#1290 L3 读数、#1289 二类见证）各自票内已带，本票只登记索引。

## 1. 锁网清单（判据 × 锁位置 × 形态）

| # | 判据 | 锁位置（测试名，file:line） | 形态 | 历史分歧案例回归向量 |
|---|---|---|---|---|
| 1a | 「背驰段被打破」结构判据（`div_segment_broken`，#1274 替换后）· 窗口清窗 site | `trading::nested_fugue::tests::window_clear_price_break_without_structure_keeps_window`（nested_fugue.rs:1486） | 确定性集成测试，同 tape 同输入逐 site 比对结构判定输出 | #1263 §6.1 误杀向量：价格破极值（c=111>110）∧ 结构未破 ⟹ 不清窗 |
| 1b | 同上 · 窗口清窗 site | `window_clear_structure_break_without_price_clears_window`（nested_fugue.rs:1507） | 同上 | #1263 §6.2 漏杀向量：结构破（confirmed Sell1）∧ 价格未破（c=109<110）⟹ 清窗 |
| 1c | 同上 · 定位失效 site | `located_invalid_structure_break_without_price_invalidates`（nested_fugue.rs:1535） | 同上（对照臂 = `liquidation_only_at_top_emergence_perfection`） | #1263 §6.2 located 漏杀向量：结构破（confirmed Sell1@4）∧ 价格未破 ⟹ located 失效 ⟹ C 不清仓 |
| 1d | 同上 · 声部解栈 site | `voice_unwind_structure_break_without_price_negates`（nested_fugue.rs:1568） | 同上（对照臂 = `negation_kills_child_with_shrink_rebase`） | #1263 §6.2 voice 漏杀向量：结构破（confirmed Sell1@3）∧ 价格未破（c=109<110）⟹ 解栈 |
| 2 | BSP 谓词族（IsType1 / IsType3Buy(firstRetrace) / SecondTypeStructure）Lean↔Rust | `rust/tests/bsp_predicate_lean_parity.rs::lean_is_type1_vectors_bit_exact`(:159) / `lean_is_type3_buy_first_retrace_vectors_bit_exact`(:173) / `lean_second_type_structure_existence_bit_exact`(:189) | 固定输入向量集，Lean `decide`/可计算判定镜像提取值 vs Rust 输出逐位比对（先例：Interval gap/overlap 段） | `type1_broke_not_divergent`（破中枢但力度反超）、`type3_buy_not_first_retrace`（非第一次回抽）、`type3_buy_reenter_zg`（回抽回到 ZG）、`last_leg_type1_no_successor`（唯一破中枢背驰腿无后继） |
| 3 | 中枢构造两套恒等：`center_from_segments ≡ center_from_window`（L0 生产输入域） | `theta_v0::classifier::center::tests::center_segments_window_identity_on_l0_domain`（center.rs:531） | 全枚举 L0 方向交替域 + 逐位恒等断言 | #812 Z-3 来源分歧臂：同向三段（域外）segments 拒绝 / window 接受（锁非 vacuous） |
| 4（登记索引） | #1290 L3 读数对拍 | 各自票内已带（#1290 票面「L3 读数靶向对拍」） | — | — |
| 4（登记索引） | #1289 Lean↔Rust 二类见证对拍 | 各自票内已带（#1289 票面「补对拍锁：本谓词的 Lean↔Rust 向量级对拍用例（同输入同见证点）」） | — | — |

## 2. 锁 1：赋格族结构判据对拍（#1273/#1274 替换后）

- **判据**：`div_segment_broken(dir, evrow, flip)`（nested_fugue.rs:497）——#1263 报告 §7 操作化规格：反向突破（confirmed Sell1/Sell3 或 Buy1/Buy3）∨ 次级别走势完成（dir flip 翻反向）。
- **锁位置/形态**：四把确定性集成测试（1a-1d），用 `warmup34`/`with_ev` 合成 tape 驱动 `run_nested_fugue` 生产路径，逐 site（窗口清窗 / 定位失效 / 声部解栈）断言「结构判定输出 = 生产否定行为」，价格不参判据。
- **历史分歧案例**：全部取自 #1263 §6 实测分歧面（价格代理与结构判据吻合 0.18%、误杀 1667/漏杀 1662）——误杀向量证明「价格破但结构未破不得否定」，漏杀向量证明「结构破但价格未破必须否定」；旧价格代理（`c > w.extreme` / `c > ext` / `c > line`）在这两组向量上方向相反，回归向量即锁的判据来源。
- **CI 面**：`cargo test --all-targets`（default targets）内非 `#[ignore]`，确定性无外部数据依赖。

## 3. 锁 2：BSP 族 Lean↔Rust 向量级锁（F-8）

- **判据**：`IsType1`（BspClassification.lean:94）↔ `is_type1_buy`（closed_loop/buy.rs:112）；`IsType3Buy`（:115，含 `firstRetrace`）↔ `is_type3_buy`（buy.rs:120）；`SecondTypeStructure`（RMoveCompose.lean:233，∃i1<i2 存在性）↔ `find_second_type_structure(...).is_some()`（rmove_compose.rs:175，首个后继 i2=i1+1）。
- **机器耦合**：fixture 段 `bsp_predicates` 由 `formal/Origin/ParityFixtureExport.lean` #eval 机器导出（`bspEndpointVectors` :178 / `secondTypeStructureHolds` 可计算判定镜像 :244 / `secondTypeStructureVectors` :340），落盘 `rust/tests/fixtures/theta_v0_parity.json`；`scripts/check_fixture_drift.py` 守「Lean 源 → fixture」一段，rust 侧 `include_str!` 单源读取消解（#319 先例）。
- **SecondTypeStructure 存在性等价口径**：`∃ i1 < i2` ⟺ `∃ i1 ≤ subs.length-2`（`SubLevelType1` 只约束 m1，i2 存在即后继存在，取 i2=i1+1）——Lean 可计算判定镜像与 Rust `.is_some()` 同真同假；**见证级（i1/i2/second_point/retrace_breaks）对拍归 #1289**，本票只登记索引。该存在性等价已由 `secondTypeStructureHolds_iff`（列表级）与 `secondTypeStructureHolds_iff_secondTypeStructure`（对接规范 `RMoveCompose.SecondTypeStructure`）机器证（review #1294 补；腿级 `subLevelType1Holds_iff` 原已有）。
- **CI 面**：`cargo test --all-targets`（集成测试）+ `fixture-drift` job（`check_fixture_drift.py`，本 diff 已重落 fixture，drift 绿）。

## 4. 锁 3：#804 恒等性锁（#812 Z-3 撤销的随票锁）

- **判据**：`center_from_segments`（center.rs:241，含 `dir_alternates`）≡ `center_from_window`（center.rs:283，无方向交替）在 L0 生产输入域（`dir_alternates` 真）上逐位恒等。
- **锁位置/形态**：`center_segments_window_identity_on_l0_domain`（center.rs:531）——先钉分歧臂（同向三段：segments 拒绝 / window 接受，证明两函数非无条件恒等、锁非 vacuous），再对两种方向交替三元组 × 4×4×4×4×4×4 区间端点网格全枚举断言 `Option<Center>` 逐位相等（覆盖核心非空 / 核心空 / 单点核心三类落点）。
- **CI 面**：`cargo test --all-targets`（lib 测试）。
- **元层同步**：AGENTS.md 在案实例段已由 #1278 批前提交（`2e7ccd9ae7`）订正为「#812 Z-3 撤销」，本锁补齐其缺失的测试锁载体（sliceA ⑧❌ 收口）。

## 5. 验收对照

- [x] 锁网清单成文（判据 × 锁位置 × 形态）：本文件 §1。
- [x] 1-3 三项锁在 CI 跑（cargo test 全量内）：§2/§3/§4 各锁均为非 `#[ignore]` 测试，落 `cargo test --all-targets` / `--lib --features backtest_bin` 面。
- [x] 每把锁带至少一组「历史分歧案例」回归向量：§1 表内各锁回归向量列。
- [x] 全量测试不降：新增 4（lib）+ 1（lib）+ 3（集成）= 8 把测试全绿，既有 parity/嵌套赋格测试全绿（见 §6 证据）。
- [x] delivery-discipline 全子句：见关票声明（编排层）。
- [x] 收尾 `python3 scripts/check_fixture_drift.py`：`--fixture parity` 绿（见 §6）。

## 6. 验证证据

```text
# 环境（同 #1263 报告 §10：PyO3 链接需 Python 3.11 库）
export PYDIR=/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu
export LD_LIBRARY_PATH=$PYDIR/lib:$LD_LIBRARY_PATH LIBRARY_PATH=$PYDIR/lib

cd rust
cargo check --all-targets                  → Finished（仅既有 62 警告，零新增 error）
cargo fmt --all -- --check                 → 0 差异（绿）
cargo test --lib nested_fugue              → 28 passed / 0 failed / 1 ignored（nested_fugue_extreme_probe 仍 #[ignore]）
cargo test --lib center_segments_window_identity_on_l0_domain → 1 passed
cargo test --test bsp_predicate_lean_parity → 3 passed
cargo test --test theta_v0_buy_parity --test theta_v0_lean_parity --test theta_v0_center_parity → 10+1+4 passed

cd ..
python3 scripts/check_fixture_drift.py --fixture parity → ✓ 无漂移（语义一致 0 字段差异；字节级一致）
```

## 7. 收尾

- 零生产码改动：锁 1 全部 `#[cfg(test)]` 内、锁 2 为集成测试 + Lean fixture 导出 + fixture 重落盘、锁 3 为 `#[cfg(test)]` 内。
- 数据文件零新增（不依赖 BTC/外部数据）。
- `check_fixture_drift.py` 全 fixture 缺省跑法需 Lean 工具链；本环境已装 elan + lean4:v4.31.0，`--fixture parity` 实测绿。

*本报告只登记锁网与证据，不代裁、不关票。*

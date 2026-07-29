# 影子评审：#319 端点机器耦合（票 #325，commit 37504114c8）

评审者：独立 claude 新上下文（只读）。结论：**通过，无 HIGH**；MED×1、LOW×2。

## Spec 轴（对照 #319 票体 / resolution / #316 MED-1）

| 核对点 | 判定 | 证据 |
|---|---|---|
| 端点真机器求值 | PASS | `ParityFixtureExport.lean:99-102` `Json.num a.low/a.high/b.low/b.high` 读 `FeatureElem` 字段；`:204-208` `feOf` 为唯一权威源 |
| rust 两处手抄清零 | PASS | `gap_overlap_fixture.rs:125-126` `a:(l.a_low,l.a_high)`；主缝 `theta_v0_lean_parity.rs:479-497` 5 组 `Interval{lo,hi}` 字面量已删（grep `Interval { lo` 无命中）；用例名嵌字端点已去 |
| 消费方单源 | PASS | `feature_seq.rs:506`、`segment.rs:811` 均只经 `gap_overlap_cases()` 取 `case.a/case.b`，无第二份端点 |
| fixture 机器再生、旧真值不变 | PASS | drift gate exit=0「字节级：一致」；fixture diff 非 gap_overlap 段逐字段相同，5 组真值未变 |
| #316 MED-1 闭环 | PASS | 三处对照中的两处 rust 誊写消除，剩导出器一处=权威源，非转录 |
| 篡改红绿可复现 | PASS（推演核实，未改文件） | 主缝 `intervals()` 接反端点：tangent×2 + strict_overlap 三案 `overlaps` 由 true 翻 false 即红 |

复跑：`cargo test --release --lib` 1862 passed / 1 failed（唯一失败 `extract_signals_bit_exact_digest_guard` = #110 在案）；parity 四测试 10+19+4+11 = **44/44**；`check_fixture_drift.py` exit=0；gap_overlap 定向 3/3。

## Standards 轴

| 项 | 判定 | 说明 |
|---|---|---|
| LeanTruth→LeanCase 重构 | PASS | `from_lean()`（:120-130）把人工输入收窄到 `name`/`kind` 两项，5 处结构体字面量塌缩为一个构造点 |
| valid/kind 双守卫职责 | PASS | valid（低层不变量，`:186-197`）与 kind（#246 裁定口径人工编码，`:222`）职责分离；valid 收敛为 lib 一份，主缝 `:500-502` 显式登记不重复断言的理由 |
| 红传导链注释诚实 | **MED-1** | lib 侧 `:141-145` 诚实登记「端点变动不改形态/真值时 rust 不红也不该红」；主缝 `:469-472` 仍写「Lean 端点改 ⟹ fixture 变 ⟹ 与导出真值不符即红」——#319 后端点与真值同源再生，该因果不成立（是漏改时代的旧模型），两处口径冲突且主缝一侧属过度声明（090 声明膨胀） |
| 090 其余 | PASS | 有效域（仅 `cargo test --lib`、CI 不跑）、L0 等级、#276 三笔互异缺角均如实留存 |

## 问题分级

- **MED-1**：主缝 `theta_v0_lean_parity.rs:469-472` 红传导表述与 lib 侧登记冲突（过度声明）。修法：改为「端点唯一权威源在导出器；Lean↔fixture 由 drift gate 守，fixture↔rust 由单源读取消解」，与 `gap_overlap_fixture.rs:139-145` 对齐。不阻塞合入。
- **LOW-1**：`gap_overlap_fixture.rs:137` 引用主缝 `theta_v0_lean_parity.rs:148`，实际 `include_str!` 在 **:154**（本 commit 把旧 :132 更新时偏 6 行；resolution 称「行号引用漂移已修」只覆盖 Lean 侧 :204-208）。
- **LOW-2**：主缝 `:502` 「此处接反端点仍会被谓词失配捕获」只对 3/5 用例成立——`strict_disjoint`/`_rev` 接反后 `overlaps` 仍为 false、`gap` 仍为 true，单独存在时不红。守卫是集合级而非逐用例级，宜照实标注。

## 边界条件

若 #246 裁定翻转（相切⟹有缺口），`fixture_endpoints_match_kind` 的形态→真值映射须同改，届时其红的含义是「裁定已变」——commit 注释已登记。若 CI 接入 `cargo test`，本缝有效域声明须更新。

## 影响声明

本评审只读，未改任何被评审文件；产出仅本报告。

# #964 CI 对拍错位 · 勘定与修法选项

日期：2026-08-14　票：https://github.com/xy7365527-lang/NewChanlun/issues/964（map #787 关图残雾毕业 debt）
执行器：本体直办　状态：**勘定完成，修法待一处退役时序判断（见 §四）**

## 一、错位实况（逐条证据）

**九组对拍** = `tests/test_rust_{bi,bsp,divergence,macd,move,ph,recursive,segment,zhongshu}_equivalence.py`（合计 34 个 `def test_`），逐一对应旧顶层 `rust/src/{bi_engine,bi_zhongshu_bsp,divergence,macd,moves,ph,orchestrator,segment,zhongshu}.rs`，验证 **Rust-v1 ≡ Python-v1 逐位等价**。

**生产 Rust 判定链 = `theta_v0/`，与旧顶层零 crate 内依赖**（坐实）：

```
$ grep -rn "crate::bi_engine\|crate::zhongshu\|crate::segment\|crate::buysellpoint\|crate::divergence\|crate::moves\|crate::macd\|crate::orchestrator\|crate::ph\b\|crate::bi_zhongshu_bsp" rust/src/theta_v0/
（空——theta_v0 不引用任何旧顶层模块）
```

即：九组对拍的 **Rust 侧（旧顶层 v1）不是生产判定路径**，票面「对拍测的是与生产无关的等价面」成立。

## 二、生产判定路径的对拍已经在 CI（错位的一半已被 #810/#951 补上）

- **Rust↔Lean parity**（theta_v0 判定路径的真对拍）：`rust/tests/theta_v0_{lean,classifier,center,buy}_parity.rs` + `theta_v0/classifier/retrace_ledger/tests/replay_parity.rs`——由 #810 的 `cargo test --all-targets` 接入 CI，**真执行**。
- **PyO3 桥**（theta_v0 消费面）：`tests/test_theta_pyo3_bridge.py` + `ThetaStream/ThetaPiStream`（#951 执刀）——在 pytest `test` job 内跑。

⟹ 统一架构正本 §二句 2「每条判定三档载体验收，且 CI 真执行」在 theta_v0 侧已由上述两者落位。

## 三、九组旧对拍的准确名分（照实，不夸大不缩小）

九组对拍验证的 **Rust-v1 ≡ Python-v1** 等价面，其两侧的生产地位**不对称**：

| 侧 | 现役？ | 证据 |
|---|---|---|
| Python-v1（`newchan.bi_engine`/`a_move_v1`/`zhongshu_engine`…） | **是**（Python 生产链） | `src/newchan/core/recursion/*.py`、`src/newchan/topology/*.py`、`src/newchan/gateway.py:33` 仍在 import |
| Rust-v1（旧顶层 `rust/src/*.rs`） | **否**（Rust 生产链） | theta_v0 零引用（§一）；仅作 PyO3 出口 `PyBiEngine`/`run_recursive_rust` 等（`lib.rs:2681-2704`）供 Python 侧可选替换 |

⟹ 九组对拍不是「完全无意义」，而是「**半边错位**」：它守着一个 Python 侧现役、Rust 侧非生产的等价面。

## 四、修法选项与阻塞点

| 选项 | 动作 | 代价/前提 |
|---|---|---|
| A 退役九组 | 删或 skip | 需先裁「旧顶层 Rust-v1 是否随 theta_v0 迁移整体退役」——当前 Python 侧仍在用 Python-v1，且 Rust-v1 是它的 PyO3 替换候选，退役时序未定（属 theta_v0 Python 侧迁移，多票未决） |
| B 改标 | 九组文件头加「v1 legacy 等价，Rust 侧非生产（生产=theta_v0，对拍=Rust↔Lean parity）」 | 非破坏、立即澄清名分；但不改变「CI 仍跑 34 个测非生产 Rust 面的用例」这一事实 |
| C 重指 theta_v0 | 把九组改测 theta_v0 | **无目标可指**：theta_v0 无 Python 孪生实现（其对拍是 Rust↔Lean，不是 Rust↔Python），「重指」的实质结果就是 A（退役）+ 认领 Rust↔Lean parity 为对拍 |

## 五、090 照实 + 结论

- **#810 已把 `cargo test` 接进 CI（本票「关联」那条，非本票范围）**：`ci.yml` rust-check job 现含 `cargo test --all-targets`（默认 + backtest_bin），Rust↔Lean parity 真执行——theta_v0 对拍已在 CI。
- **本票（对拍错位本身）尚未收尾**：九组 `test_rust_*_equivalence.py` 仍是 pytest `test` job 在跑的对拍，其 Rust 侧非生产。修法三选一（§四）里，A/C 实质是同一件事（退役 + 认领 parity 为对拍），**前提是「旧顶层 Rust-v1 退役」这一时序判断**；B 是零前提的澄清动作。
- 该退役时序判断牵涉 theta_v0 Python 侧迁移（Python-v1 何时换到 PyO3 桥），属 HITL/后续票，本票不代裁。

**建议**：编排者回来时在 A（退役，配合 B 的改标收尾）与「暂缓退役、先做 B 改标」之间拍一板，即可关 #964；若选 A，具体删/skip 由下游实施链执行。

## Resolution（2026-07-30，本票关闭）

报告：`.chanlun/review-results/issue809-doctrine-triage-20260730.md`（261 行），commit `cc1ddd7f4f`，已入当前主线。**主控独立核实**：commit 坐在 [#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 的正本 commit `dac8ff9fa1` 之上，报告内容用 `git show` 独立读路径核对，非转述其自述。

### 三个数

| 类 | 条数 |
|---|---|
| **违规-已定性** | **6**（涉及落点 11 处） |
| **合规例外候选** | **1**（上限 3，**未触顶**） |
| **真分歧-待裁** | **34 组**，分 7 概念 |
| 不适用 | 19 |

**上限预警**：例外候选只有已知那条（`compose_level` 的 `is_l0`），但违规里的 V3（`recursive_t::leg_strength`）与 V6（`a_nested_divergence` 力度按级别换判据）**与它同型**——若中枢票、背驰票选择给它们开举证书而不是收敛，[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 的全仓 3 条上限在两张票之内用光。余量 2。

### 6 条违规（4 条落在「力度」上）

V1 `div_cand`↔`sublevel_diverges` + C1 降级（[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 在案）；V2 `Cand^δ` per-rung 恒真档；V3 `recursive_t/divergence.rs:72` `nest>0?嵌套深度:几何振幅`；V4 `rust/src/divergence.rs:188` 无 MACD 时 fallback 振幅×时长；V5 `a_trendtype_v0.py:143` 源码自陈「§8.2 弱化版本」兜底；V6 `a_nested_divergence.py:118` 力度 level1 走 MACD / level≥2 走振幅×组件数。

**四条同一个病**：正经判据取不到数据 ⟹ 换个便宜的顶上 ⟹ 照常出信号。V3–V6 为本票新判，由对应概念票复核后定性生效。

### 订正 [#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 两条

1. `classifier/mod.rs:547` 的一/三类 `is_l0` 分派**不是分歧**——两侧最终都落到 `signal.rs:1526` 同一个 `extract_signals_with_hist_anchored`，级别差异只以传入方向锚出现，**总缝规则通过**。
2. `segments_diverge_or` 是**生产件**（`signal.rs:1285` 盘整背驰真值路径），#789 记的「在编译树」低估了。

### 编排者当场裁定：Lean 分支算「真待裁」，待裁面 = 34 组

本票报告把这一刀列为「没权拍」的第 3 条提示（决定待裁面是 34 组还是约 28 组）。**编排者裁定：算真待裁，34 组。**

理由（主控提出，编排者采纳）：`AGENTS.md` 权威分层明写 **Lean 管边界、是正本的一部分**，不是旁观的检查工具；「不适用」那一格是给对照臂 / 只读观测器留的，那些东西不进正本，Lean 进。[#321](https://github.com/xy7365527-lang/NewChanlun/issues/321) 被定性为「裁错了层」正说明 Lean 在裁定链里有位置。

**实证支撑（本票关票当日新增）**：[#811](https://github.com/xy7365527-lang/NewChanlun/issues/811) —— 当前主线上 `theta_v0_classifier_parity` 有 2 条 Lean↔Rust 一类买点对拍断言**为红**。Lean 与 Rust 的分歧**真的会打架、会让测试红**，不是纸面差异。

### 090 照实：本报告的证据强度

**全部总缝判定是源码语义推断，不是对拍实测**——总缝规则的判别式要求同输入同输出对拍，而当时 CI 不跑 `cargo test`。本票关闭当日另查出**第二层**：CI 触发分支 `main-rewritten` 落后主线 545 个提交（[#810](https://github.com/xy7365527-lang/NewChanlun/issues/810) 评论），即便跑也是在旧树上跑。镜像已于同日快进至 `f6d000fed2`。

### 毕业：第二批 6 张概念票已开出并接边

| 票 | 组数 | blocked by |
|---|---|---|
| [中枢的教义正本 #812](https://github.com/xy7365527-lang/NewChanlun/issues/812) | 7 + E1 退场条件 | — |
| [笔与线段的教义正本 #813](https://github.com/xy7365527-lang/NewChanlun/issues/813) | 9（两概念合并不拆） | — |
| [背驰的教义正本 #814](https://github.com/xy7365527-lang/NewChanlun/issues/814) | 5 | — |
| [走势类型与级别的教义正本 #815](https://github.com/xy7365527-lang/NewChanlun/issues/815) | 4 | #812 |
| [买卖点的教义正本 #816](https://github.com/xy7365527-lang/NewChanlun/issues/816) | 3 + [#811](https://github.com/xy7365527-lang/NewChanlun/issues/811) 必答 | #814 |
| [区间套的教义正本 #817](https://github.com/xy7365527-lang/NewChanlun/issues/817) | 6 + 两件上游转来 | #812 #814 #816 |

本票建议的「0a CI 接 cargo test」已是 [#810](https://github.com/xy7365527-lang/NewChanlun/issues/810)；「0b 六条违规登记入票」**已由上表兑现**——每条违规都写进了对应概念票的票面，不另开登记票。

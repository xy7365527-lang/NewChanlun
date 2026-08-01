Part of #787

## Question

[教义收敛的方法与次序 #793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 裁定：**先裁总缝，再逐概念**。本票就是那条总缝。

要裁的规则（初稿，本票负责把它裁成可执行的口径）：

> **同一个判定，级别的差异只准以数据形式传进去，不准写成按级别分叉的两套代码。凡分叉一律举证，举不出就出局。**

裁完这一条，[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 查出的约 40 套口径里，所有「按级别分两份」的当场批量出局，剩余分歧面才是真正需要逐概念裁的。**七张概念票在本票裁完之前不建**——现在建等于在不知道剩多少分歧面时定边界。

### 已定前提（不重议）

来自 [#793](https://github.com/xy7365527-lang/NewChanlun/issues/793) 与图 [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)：

- **权威分层**：原文管语义 / Lean 管边界（端点、退化情形）/ 生产代码只有否决权（可用实测读数打回，不得以「现在就这么跑」为由支持）。已落 `AGENTS.md`「缠论教义正本」节。
- **关票判据**：文字正本落 `.chanlun/definitions/` + 附受影响代码清单（点名到文件行号）。「在注释里登记口径分歧」不再算合规收尾，注释必须带票号。
- **正本载体**：`.chanlun/definitions/` 一概念一份，三行头（正本声明 / 最后裁定票号 / 受影响代码清单指针）由各概念票落地时填。
- **靶子形态**：不是「40 套里选一套」（投票），是「收成一套能在每一级复用的」（设计）——[#787](https://github.com/xy7365527-lang/NewChanlun/issues/787) 立图前提。

### 要裁的四件事

**① 规则的准确措辞**——「级别差异只准进数据」这句话的边界在哪？
已在案的形态样板（[#801](https://github.com/xy7365527-lang/NewChanlun/issues/801)）：`recursive_t` 信号层做到了纯函数 + 判定作参数 + **级别差异进数据** + 测试 gate，对 level 的比较 grep 命中 **0**；`theta_v0` 已 90% 同形态（`compose_level` 把中枢构造函数当值传入，`divergence`/`decompose`/`bsp` 零 level 分叉）。
但**输入类型本身随级别变**是合法的（L0 中枢由笔构成、上层由线段/走势构成——这是缠论原文的结构，不是分叉）。规则必须能把「同一判定吃不同输入」和「同一判定写了两份」分开，否则会误伤。

**② 举证门的形式**——「凡分叉一律举证」，举证交给谁、写在哪、什么算举证成功？
候选：写进该概念的定义文件作例外条款 / 走单独裁定票 / 由 Lean 定理背书。**注意与关票判据的衔接**：举证失败的分叉进「受影响代码清单」，即成为下游实施链的改造项。

**③ 已知例外面**——本仓已有一条实测立住的例外：[#800](https://github.com/xy7365527-lang/NewChanlun/issues/800) 查实**操作层**（三阶段资金战役）「每级独立自我复制」试过并被实测打回（三标的 L3 全部 `sink=0`、四角度全 infeasible、最终由编排者裁决回退），根因是**账户只有一份、不是每级一份**。[#801](https://github.com/xy7365527-lang/NewChanlun/issues/801) 的结论明写**仅覆盖结构判定层**。
⟹ 总缝规则的**适用域**要当场划清：结构判定层强制，操作层按 [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787) 的边界判据「凡单例者显式声明为单例」处理。本票需给出**判定某个东西属哪一侧的判据**，而不是逐个点名。
警示（[#803](https://github.com/xy7365527-lang/NewChanlun/issues/803) 教训）：**用该判据分类前，先确认「被分类的那个东西」是不是你以为的那个**——三阶段被当成「仓位生命周期」分类过，而真正每级一份的是 `LegPair` + `leg_open_units`，归错对象比归错类更隐蔽。

**④ 机械锁的载体**——规则靠什么防止再次漂移？
[#801](https://github.com/xy7365527-lang/NewChanlun/issues/801) 的结论是「取思想不取机制」：机械保证的载体**可以是测试 gate 而非类型系统**（`recursive_t` 的 `operator.rs:198` 即有回归测试锁「所有级别用同一套代码」，且零 trait 零泛型零宏）。本票需裁：本仓采不采这条，锁放哪一层。
**注意本图不带实装**——本票只裁「锁应当是什么形态」，真加 gate 进下游实施链。

### 已在案的靶子（不是待查事实，是本票要消费的读数）

- `theta_v0` 的 `is_l0` 命中 **36 处 / 4 文件**（`classifier/mod.rs` 为主场，另有 `classifier/recursive_tower.rs`、`backtest/wverify_run.rs`、`backtest/econ_positive.rs`）。[#801](https://github.com/xy7365527-lang/NewChanlun/issues/801) 判它是「同塔 L0 与 L≥1 中枢定义不同」的**机制源头**。
- 同塔中枢口径分叉（[#794](https://github.com/xy7365527-lang/NewChanlun/issues/794)）：L0 走 `center_from_segments`（含方向交替，9700 次）、L1/L2 走 `center_from_window`（**不看方向**，8956 次），上层吃下层产物，**零裁定票**。
- 40 套口径的第二个生成机制（[#801](https://github.com/xy7365527-lang/NewChanlun/issues/801)）：实验箱重抄口径，`theta_v0/backtest/wverify_run.rs:2944` 的 `c327_weak_*` 一族。
- π 门 rung 链的 `Type2|Type3 => true`（[#802](https://github.com/xy7365527-lang/NewChanlun/issues/802)）：**实测 95.36% 信号 rungs 空**，恒真有 673-fix + [#97](https://github.com/xy7365527-lang/NewChanlun/issues/97) D1 两道裁决背书，与 P2 已结算的 `Cand^δ` 定义式三道共存 —— 这是「按级别免判」的极端形态，本票的规则要能对它表态。

### 边界

- **不裁任何具体概念的口径**（中枢/背驰/区间套各自的定义留给第二批概念票）。
- **不动代码**。本图不带实装；resolution 允许落文档与零行为变更注释。
- 本票裁完后重新清点剩余分歧面，再定第二批开几张、按什么次序。

## 纪律

`/grill-with-docs`，**一次一问**，agent 不得替人答。

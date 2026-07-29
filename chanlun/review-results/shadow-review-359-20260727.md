# 影子评审：#359 coverage 拆分合主线（三 commit：`59248b301b` / `da72db35a2` / `28773fbc99`）

- 票：#373（parent #59）；对象：coverage.rs（6909 行）→ coverage/ 目录拆分
- 评审者：独立新上下文（claude opus medium，非实装 lineage）；只读，零 git mutation
- 工作区：/tmp/nc-review-359（detached @ 28773fbc99）
- **谱系注记（090）**：评审 agent 交付时声称报告已落盘，实际未写任何文件（主 worktree 与其隔离 worktree 均无）。本文件由编排者**据评审 stdout 逐字重构**，内容即其交付原文，未加评判。

## 裁决

**Spec PASS（①–⑤ 全过）｜Standards PASS（0 CRITICAL / 0 HIGH / 0 MEDIUM / 5 LOW——全是簿记数字，代码本身无缺陷）**

## 独立机械复核（不采信自述）

- **① 零行为变化**：diff 19 条路径全在 coverage 下；原 43 个 `pub` 项集合差 missing=[] extra=[]；114 个测试名集合差空且无重名；7178 行中 6720 行与原文件字节级同行 = 93.6%；剩余 301 行非空新增逐条过完，只有 `super::` 深度补偿和字段可见性标注，**无一条逻辑改动**。复跑 1920 passed / 1 failed，fixture 漂移 exit=0。唯一失败 `extract_signals_bit_exact_digest_guard` 在 `classifier/signal.rs`，该文件不在 diff 内且对 `strategy::`/`coverage` 零引用——因果上不可能由本次三 commit 造成。
- **② 行数**：18 文件最大 721，全部 ≤800。
- **③ 门面兼容**：全仓 26 文件约 150 处 `coverage::` 全走门面，零处穿透私有子模块；`cargo check --all-targets` 零 error。
- **④ H-1**：删 22 项后零 error；保留的 8 项确有真实消费者（coverage 内 + backtest/fill.rs 等外部），「rustc 误报」登记照实，mod.rs 现存告警恰 3 处、正好这 8 个名字。
- **⑤ M-1 破环**：剥注释后的符号级依赖图 DFS **0 环**，`element` 出度 0 成唯一汇点。（不剥注释会假报 16 个环，全是 doc 里的交叉指名。）

## Standards 三面

- glob 登记照实（生产模块除 1 条 `use super::*` 外零 `use`）；
- 测试经 `#[path]` 挂在**被测生产模块**内（比塞门面更强）；
- cargo fix 越界回退完备——那 9 个文件相对基线字节级零差异。

## LOW×5（全部是 resolution 汇总数字与实际不符）

文件数 17→实际 **18**；`*_tests` 8→实际 **9**（commit message 自己写的 9 是对的）；mod.rs 119→**109**；新增行 7184→**7178**；可见性升级 45→实测 **41**；失败测试「#115 线」票号疑误引（#115 是 #110 评审票，正文无 signal/digest），建议更正为真实归属票，否则下轮「唯一失败在案」无法复核。

→ 处置：簿记数字已随 #373 resolution 照实订正；「#115 线在案」归属已由编排者在 #115 贴登记评论补强（digest 守卫属 #110 实装面，存留处置归该评审线）。

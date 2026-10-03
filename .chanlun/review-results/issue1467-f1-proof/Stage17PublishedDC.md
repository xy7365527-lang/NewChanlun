# Stage17：真实可见报价状态上的底层DC构造

日期：2026-10-03，绑定候选研究 #1467 和实验备料 #1468。接续 [开工卡](Stage17WorkCard.md)、[Stage16接收前缀视图](Stage16CausalViews.md)。名分为在制研究证据，`semantic=not_reviewed`。

## 结果与适用范围

在已取得的 Coinbase BTC-USD 2021-01-01 UTC 首分钟探索样本中，具名时钟 `R_W-DC-postcapture-v1` 产生1,677个L1价量变化观察。固定功能参数δ=0.01报价单位时，量加权路径确认911个底层DC单元，报价中点路径确认512个。两条路径均在每个观察前缀对照原首次命中/极值参考，completed、active与初始化位置一致，EOF保留活动尾部。

量加权路径有118次确认发生于报价价格不变、数量变化的观察，其中88次来自非match消息，分别为open 54次、done 34次。这说明数量变化进入了该观察函数和底层构造。它不证明这些结构能够预测、可交易或带来递归增量；这些次数也不是全部数量变化的次数。

四个正常检查点与一个缺口中检查点的新进程恢复、四个物理输入截断、精确投影、域中断与溢出控制通过。补查了活动单元角色首见与确认的完整来源字段。以上都是固定输入与具名扰动上的作者机器检查，未证明Rust全域精化、完整F₂资格或市场价值。

## 冻结的观察契约

[研究适配器](published_l1_dc.rs)只读取Stage16输出的已可用双边L1报价。普通ready行在四维价量向量变化时产生一个观察；量化后数值相同也保留这个观察。初始、snapshot_batch、catchup_batch开始新epoch，未知盘口结束旧epoch并保留未完成尾部。正常报价变化不结束epoch。

此时钟观察每个捕获行补应用完成后的状态，不重建批内逻辑路径。它是新具名候选输入时钟，不冒称原序号路径，也不把批内状态回填至历史可用时刻。数据的供应商capture时间给出本研究的信息可得口径，不是本机实际发布或下单时间。

对正整数bid价b、ask价a、bid量q_b、ask量q_a，精确投影为：

```text
weighted = floor((2 * (a*q_b + b*q_a) + q_b + q_a) / (2 * (q_b + q_a)))
midpoint = floor((a + b + 1) / 2)
```

价量尺度为1e8，输出采用半值向上舍入。δ固定为1,000,000观察ticks，即0.01报价单位；这是预先写卡的功能探针参数，没有在观察输出后选优，也不声明其为交易所最小报价单位。乘法、加法与舍入使用checked i128，超出计算域明确拒绝，不用浮点兜底。中点是同观察时钟、同δ下的辅助对照，不能替代逐笔成交基线A。

`directional_change.rs`逐字保留，SHA见[来源锁](stage17-core-origin.json)。复用它的构造器、首次命中参考与几何audit；本轮没有修改原候选定义、旧报告或声明锁。底层DC单元仍不是已获资格的缠论笔、线段或完整走势。

## 真实输出

| 项目 | 原样本结果 |
|---|---:|
| 捕获输入行 / 输出行 | 20,868 / 20,868 |
| ready / unknown行 | 20,529 / 339 |
| epoch / 已结束epoch | 1 / 0 |
| 观察 / 前缀参考对拍 | 1,677 / 1,677，每次同时核两条路径 |
| 量加权 / 中点完成单元 | 911 / 512 |
| 量加权仅数量变化确认 | 118，其中非match 88 |
| 中点仅数量变化确认 | 0 |

量加权确认来源为price_change的done 216、match 27、open 550，以及quantity_only的done 34、match 30、open 54。此分类记录触发当前发布状态变化的原消息类型，不把done全部解释为撤单，也不从非match推断过去没有成交。

每个完成单元保留起点、极值端点、方向首次可见时刻active_since及反转确认时刻confirmed_at。端点发生在过去，确认只能在当前捕获前缀发布。原样本末尾量加权活动方向首见于第20,855行，中点首见于20,848行；两者均为provisional，没有因EOF被结算。

## 检查与负控制

[主验证](stage17_verify.mjs)复用已经跑完的基线，未再次重跑基线。

| 检查 | 结果 |
|---|---|
| BigInt逐观察重算投影 | 正常与长缺口输入均与Rust一致 |
| 正常新进程恢复 | 第339、340、1000、10000行，后缀、状态、引擎和输出链相同 |
| 物理输入截断 | 四处输出分别等于完整运行相应前缀 |
| 长缺口 | 第1001行关闭旧域但不结算尾部；1001至10004无可用epoch；10005开始epoch 1，观察编号归零 |
| 跨缺口单元 | 0；新域所有单元起点不早于10005 |
| 缺口中恢复 | 第10000行恢复，后缀、状态和输出链一致 |
| 改δ恢复 | 改为1000001明确拒绝，无输出文件或通过report |
| 合成整数溢出 | `projection rounding overflow`，退出1、输出空、无通过report |

长缺口输入产生868个观察、量加权441个完成单元、中点263个，分属2个epoch。这些数字只描述该扰动的输出，不与完整样本作收益或稳健性比较。

检查点保存当前epoch的观察与来源，恢复时重放这些观察重建两台DC引擎。它验证的是**保存输入后重建**，不是DC内部状态序列化。恢复绑定输入文件SHA、可执行文件SHA、profile、δ、保存状态摘要与引擎指纹；不声称这些未加密摘要提供对抗性认证。文件全量哈希用于离线版本绑定，不参与结构数值计算。

[时间来源补查](stage17_provenance_verify.mjs)仅读取现有输出，独立计算首次阈值命中位置，并逐项核对角色首见、确认、活动/完成端点、相邻连接与域退出尾部。正常输出核41,058份活动状态、1,423个完成单元；长缺口输出核23,050份活动状态、704个完成单元及1次域退出。故意改确认观察号、角色首见行、活动端点时间或把退出尾部标为完成，四个负控制均被拒绝。它是作者编写的另一检查器，不是独立语义审查。

## 复现与证据

输入、输出和检查点留在仓外，未新增下载。当前证据目录约174.40MiB，小于开工卡200MiB上限。主验证最多并发2个子进程，每个60秒上限，全部正常结束；旧枚举、旧Lean和生产测试未重跑，`formal/`无改动。

现有证据目录：`/Users/silencehan/Documents/Codex/research-evidence/issue1467/published-dc-v1/`。完整命令和退出码在`verification-1/*/receipt.json`，汇总在`verification-1/summary.json`与`provenance-1.json`。[入仓证据索引](stage17-evidence.json)锁定源码、输入、二进制、报告、检查点及输出摘要，不提交原始数据。

从本工作树根目录复现时，用新的仓外输出目录，不覆盖已有证据。以下参数中的目录须事先不存在，`run`的父目录由调用者创建：

```bash
cargo build --locked --manifest-path .chanlun/review-results/issue1467-f1-proof/stage17-adapter/Cargo.toml --target-dir /tmp/nc1467-cargo-target
mkdir /Users/silencehan/Documents/Codex/research-evidence/issue1467/published-dc-reproduction
/tmp/nc1467-cargo-target/debug/order-event-published-dc run /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-v1/rows.jsonl /Users/silencehan/Documents/Codex/research-evidence/issue1467/published-dc-reproduction/checkpoints /Users/silencehan/Documents/Codex/research-evidence/issue1467/published-dc-reproduction/rows.jsonl /Users/silencehan/Documents/Codex/research-evidence/issue1467/published-dc-reproduction/report.json 1000000
node .chanlun/review-results/issue1467-f1-proof/stage17_verify.mjs /tmp/nc1467-cargo-target/debug/order-event-published-dc /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-v1/rows.jsonl /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-v1/gap-recovery-1/continuous/rows.jsonl /Users/silencehan/Documents/Codex/research-evidence/issue1467/published-dc-reproduction /Users/silencehan/Documents/Codex/research-evidence/issue1467/published-dc-reproduction/verification-1
node .chanlun/review-results/issue1467-f1-proof/stage17_provenance_verify.mjs /Users/silencehan/Documents/Codex/research-evidence/issue1467/published-dc-reproduction/rows.jsonl /Users/silencehan/Documents/Codex/research-evidence/issue1467/published-dc-reproduction/verification-1/long-gap/rows.jsonl /Users/silencehan/Documents/Codex/research-evidence/issue1467/published-dc-reproduction/provenance-1.json
```

不同构建的二进制哈希可以不同，应从自己的新基线生成检查点，不复用旧二进制绑定的检查点。

## 对研究目标的影响

本轮补上真实可见状态到具名底层DC的非空接线与限定前缀/恢复证据，保留了价格不动时数量变化进入构造的见证。它没有消除Stage14上递归反例，也没有验证R_D-Q-v0在报价变化与耗尽上的扩展。

下一数学问题仍是带成员、层级与生命周期的完整F₂语义及没有严格上级核心时的合法保留状态；局部窗通过不能封装为完成走势。P3的四臂仍缺具名确认任务、外部标签、确认集及合格的F₂，未进入训练、调参或确认实验。候选可见时钟、epoch边界和恢复方式为P4提供接口要求，但处理/发布延迟、吞吐与执行能力没有实测，不能据此选择硬件或生产换装。

父模型换模入口仍不可用，独立审查未运行。待审入口见[Stage17审查包](Stage17PendingReview.md)，作者检查和有限机器证据分别保留，不记为独立通过。研究票与活动goal保持开放。

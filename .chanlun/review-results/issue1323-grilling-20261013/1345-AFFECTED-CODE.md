# #1345 受影响代码清单（独立选父 NestInterval 的价格挂法）

> 票：[#1345](https://github.com/xy7365527-lang/NewChanlun/issues/1345)；教义正本：[`qujiantao.md`](../../definitions/qujiantao.md)（区间套；落点 §3.4.4、§3.4.1 翻盘口、§10.2 第 40 条、§9.1）。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**。行号于该基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码（本次提交仅有的一处代码改动是 #1344 的一行零行为注释，见 #1344 清单）。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/theta_v0/classifier/nest.rs:44/:72/:399/:486/:958/:974-997` | main `465174240b` | `NestInterval`（`:44`）保持索引轴、不加价格字段；为 typed 证书追加 R 价格证书附件（selected 身份、policy 版本、四端点算式、缺证原因）；不改候选选择（`:974-997` 同级全表选父）；`:44` 处零行为注释带票号 | TB-02-P #1438 |
| Rust | `rust/src/theta_v0/classifier/level_view/confirm.rs:123` | main `465174240b` | `NestCandidateEvent` 无价格字段：经同源原始序列查询桥取 R，不从索引强转 | #1438 |
| Rust | `rust/src/theta_v0/classifier/divergence.rs:926/:932` | main `465174240b` | `move_range_envelope` 与整支落入过滤不改名、不声称与 R 等价；R 另写原语 | #1438 |
| Rust | `rust/src/theta_v0/classifier/descend.rs:65/:91/:99` | main `465174240b` | 不改：真实后代链的取极值有效域不扩 | — |
| Rust | `rust/src/theta_v0/classifier/nest_index.rs:306` | main `465174240b` | 索引入口把价格证书与 selected 身份、版本、失败原因绑定 | #1438 |
| Lean | `formal/Strict/Nest.lean:93/:185` | main `465174240b` | 不改原 `Sub`（索引闭包含） | — |
| Lean | `formal/Origin/IntervalNestCertificate.lean:393/:419` | main `465174240b` | 新增具名映射 `P_R` 与条件定理：`Sub ∧ 同源完整 ⟹ price-subset` | #1438 或 Lean 跟进票 |
| Lean | `formal/Origin/NestingCertificate.lean:84/:219/:223` | main `465174240b` | 抽象 lo/hi 的坐标内容由 `P_R` 实例提供，不去掉负 Tick／scale／来源 | 同上 |
| Python | `src/newchan/a_nested_divergence.py:15/:81/:112` | main `465174240b` | #1286 冻结对照，外壳消费 Rust 结果 | — |
| Doc | `.chanlun/definitions/qujiantao.md` | main `465174240b` | 三行头、§3.4.1 翻盘口闭合、§3.4.4、§9 第 74 条、§9.1、§10.2 第 40 条（本次提交已落） | 本票落文档 |
| Doc | `CONTEXT.md` | main `465174240b` | 词条「价格投影 R」（本次提交已落） | 本票落文档 |

## 本轮复核订正（相对工位草稿）

- 工位草稿称新增「§3.4.2」；`qujiantao.md` 的 §3.4.2／§3.4.3 已被 #908 订正与元判据占用，正本顺延为 §3.4.4。
- `qujiantao.md` §9 第 74 条记的 `divergence.rs:888`／`:894` 已漂到 `:926`／`:932`，正本已就地订正。
- `nest.rs:44` 的带票号零行为注释随 #1438 落，本次提交不改代码。

基线说明：工位草稿以 main `f95a7f3e` 为基线（#1344 部分行以 TB02D `03239158` 为基线）；本清单不靠差异推断，全部行号直接在 main `465174240b` 上用 `git show` 逐行核过。

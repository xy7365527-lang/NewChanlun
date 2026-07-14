# #38 证书装配器 `confirm_src` 拆绑实装（2026-07-10）

## 裁决口径

本次兑现 #37 P0 局部改判：保留每级 C 段完成确认、确认时点递降门与结构区间闭包含，撤销“算法确认时点与跨级定位窗右端是同一字段/可互相回填”的装配器绑定。

本次只拆字段承载与赋值链，不另行发明 `J` 的新几何口径，也不声称实现 LiveCand 或提前实时确认。当前 settled 生产者上 `confirm_src` 与 `interval.end` 仍可能数值相等；该相等不再是证书结构的不变量。

## 实装摘要

- `rust/src/theta_v0/classifier/recursive_tower.rs`
  - `CandDeltaEvent.confirm_src` 明确为算法确认时点，`interval` 明确为结构定位区间；两者分别从 `pf.source_index` 与 `seg.end_index` 赋值，不互相派生。
- `rust/src/theta_v0/classifier/nest.rs`
  - `NestCertificate` 新增可选 `base_confirm_src`，严格基例装配写入 `Some(base.confirm_src)`。
  - `NestRung` 新增可选 `confirm_src`，严格逐级装配写入 `Some(CandDeltaEvent.confirm_src)`；`interval` 继续独立经 `cand_rung_interval` 读取。
  - 旧证书链没有逐级确认见证，适配时明确写 `None`，不以同一个 source 或 `interval.end` 伪造确认时点。
  - 确认递降仍比较 `confirm_src`，闭 `Sub` 仍只比较 `interval`，两门保持正交。
- `rust/src/bin/strict_nest_check.rs`
  - 证书样例同时打印 `confirm` 与 `J=[start,end]`，避免诊断输出继续只展示区间而丢失确认时点。
- 共享证书的既有构造点同步补齐新字段；未修改任何 parser 文件。

## 回归测试

新增 `theta_v0::classifier::nest::tests::assemble_preserves_confirm_src_independent_from_interval_end`：

- 基例：`confirm_src=90`，`interval.end=80`；
- 父 rung：`confirm_src=70`，`interval.end=100`；
- 断言装配后证书分别保留四个值，且完整链仍通过 `n_delta()`。

定向验证：

- `cargo test --release --lib theta_v0::classifier::nest::tests`：34 passed，0 failed；
- `cargo test --release --lib strict_nest_sidecar_summary_matches_p2_assembly`：1 passed，0 failed；
- `cargo test --release --bin strict_nest_check funnel_tests`：3 passed，0 failed。

## 全量硬门

命令：

```text
cargo test --release --lib
```

结果：**1536 passed，0 failed，127 ignored**。

## 影响边界

- 不改 `Cand^δ`、背驰 gauge、盘整背驰入链口径、确认递降或闭 `is_sub` 判据。
- 不改订单、候选、风控、账本或收益/alpha 路径。
- 不改 `rust/src/theta_v0/parser/` 及其他 parser 代码。
- 本次证明的是装配器可诚实承载 `confirm_src != interval.end`；新 `J` 右缘定义、LiveCand、证书产量与经济效果仍需独立任务裁定和验证。

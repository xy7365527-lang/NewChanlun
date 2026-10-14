# #1356 受影响代码清单

> 票：[#1356](https://github.com/xy7365527-lang/NewChanlun/issues/1356)；正本：`docs/adr/0036-*.md`。
> 基线：**main `3da0b2c872e6e7d69ffcfe29f989a08491d92dd1`**；FIN 线 `codex/1394-transport-revision-20261012` @ `0d8a6017`（标 FIN）。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/economic_session/book.rs:347/398-413` | main | 重视图增 cost_remaining（多头 K；空头 ΣM_b − 费后已实现利润）与稳定点序号；K 有符号不 clamp 保持 | #1406/#1398 |
| Rust | `rust/src/economic_session/book.rs:151/225` | main | fee_only 等分量（ADR 0031）作为样本输入；无新分量 | #1406 |
| Rust | `rust/src/economic_session/store.rs:636` | main | 样本构造由审计重放同源：按稳定点聚合、排除原因表、完整账对账 | #1398 |
| Rust | `rust/src/economic_session/service.rs` | main | 新增经营统计查询：Formal/NotComputable/InsufficientEvidence/AwaitingPolicy 四态、方法标名、买/卖频率分列 | #1398 |
| Rust | `rust/src/theta_v0/config.rs:188-193/242` | main | chi_z_alpha 正态 LCB 只作过渡（ADR 0022 裁定三）；验收读数一律 block bootstrap；不互引 | — |
| Rust | `rust/src/theta_v0/strategy/ledger.rs:716/742` | main | LedgerEvent 算子不变；旧 TW 账不作样本来源 | — |
| Lean | `formal/Origin/TotalWealth.lean:94-103` | main | 无持仓量分量（ADR 0015 丙-5）；本票样本在 R2 book 上定义，不依赖此处；登记 | — |
| Python | `s_session/economic_query.py:72` | main | 透传统计四态与排除原因表 | #1398 |
| Python | `s_session/browser/economic.html:67` | main | 报告卡片：方法标名、n/月块/窗、排除计数；不自算比率 | #1398 |
| 文档 | `docs/adr/0036（编号待根分配）` | main | 与 #1358 合写 |  |
| 文档 | `CONTEXT.md` | main | 词条：经营统计样本（重×稳定点）/ cost_remaining |  |

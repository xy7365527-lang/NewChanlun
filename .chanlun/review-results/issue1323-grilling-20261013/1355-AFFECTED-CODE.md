# #1355 受影响代码清单

> 票：[#1355](https://github.com/xy7365527-lang/NewChanlun/issues/1355)；正本：`docs/adr/0034-*.md`。
> 基线：**main `3da0b2c872e6e7d69ffcfe29f989a08491d92dd1`**；FIN 线 `codex/1394-transport-revision-20261012` @ `0d8a6017`（标 FIN）。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/theta_v0/config.rs:157` | main | depth_weights 资金帽语义已由 ADR 0017 作废；登记为旧入口参数，不作量档来源（TB-10 收编/退役） | TB-10 |
| Rust | `rust/src/theta_v0/strategy/voice.rs:12-13/193-197/206` | main | depth_weight() 为旧入口；新量档由 R2 policy 记录供给，不改此处行为 | TB-10 |
| Rust | `rust/src/theta_v0/strategy/oscillation_campaign.rs:589` | main | /3 常数已被 ADR 0017 取代；登记 | TB-10 |
| Rust | `rust/src/economic_session/book.rs:175/347` | main | fixed_q 校验保留；重视图增 quantity_policy_ref 与 tier 查询投影（只读） | #1444 |
| Rust | `rust/src/economic_session/store.rs:78` | main | manifest 的 fixed_q 保留为引用值；新增 quantity_policy 记录表与版本索引 | #1444 |
| Rust | `rust/src/economic_session/service.rs:105` | main | 路由校验 fixed_q 不变；新增 policy 查询 op | #1444 |
| Rust | `rust/src/economic_session/finance.rs:204-213/225` | FIN | policy 字段组加 quantity_policy_id（引用）；不复制 tiers | #1394 后续/#1444 |
| Rust | `rust/src/bin/p915_capacity_frequency.rs` | main | 探针输出补：n、最大月簇、逐窗、不确定性带（作为 Measured 来源的必填项） | #1444 或探针票 |
| Rust | `rust/src/theta_v0/strategy/risk.rs:554/598` | main | LeverageCaps 与量档无耦合；登记不互引 | — |
| Lean | `formal/Origin/` | main | 量层无定义式；本票不新增 Lean 义务（整数取整与版本生效为工程合同，锁在测试） | — |
| Python | `s_session/economic_query.py:72` | main | 透传 quantity_policy / AwaitingPolicy{tier,capacity} typed 结构 | #1444 |
| Python | `s_session/browser/economic.html:67` | main | 重卡片显示 tier_quantity 与未定档原因，不计算比例 | #1444 |
| 文档 | `docs/adr/0034-quantity-tier-policy-growth-equity-shift.md (编号待根分配)` | main | 新建，与 #1357 合写 |  |
| 文档 | `CONTEXT.md` | main | 词条：量档 policy / tier_quantity / 容量未知≠无限 |  |

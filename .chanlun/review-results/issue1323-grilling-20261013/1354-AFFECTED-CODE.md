# #1354 受影响代码清单

> 票：[#1354](https://github.com/xy7365527-lang/NewChanlun/issues/1354)；正本：`docs/adr/0035-*.md`。
> 基线：**main `3da0b2c872e6e7d69ffcfe29f989a08491d92dd1`**；FIN 线 `codex/1394-transport-revision-20261012` @ `0d8a6017`（标 FIN）。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/bin/p907_cost_gate_level.rs` | main | 探针补：落原始样本、按月 block bootstrap 单边 95% 下界、互不重叠窗参数、定点整数输出；作为 Measured 来源血统 | #1441 |
| Rust | `rust/src/theta_v0/venue_fee.rs:112` | main | Taker 常量不证明角色；费率只从 fee_schedule_ref 读 | #1441 |
| Rust | `analysis/data_cache/venue_fee_binance_spot_20260726.json` | main | 作为 fee_schedule_ref 的在册实例（spot，VIP0/BNB25），只对 spot/BTC 域有效 | — |
| Rust | `rust/src/theta_v0/config.rs:188-193/242/253` | main | chi_z_alpha 与 entry_delay_bars 是旧准入/代理参数，不是本 policy 输入；登记不互引 | — |
| Rust | `rust/src/theta_v0/backtest/econ_positive.rs:823-825` | main | 塔底判据位置；成本门 NotEvaluable 时只剩此门；加带票号注释 | #1441 |
| Rust | `rust/src/theta_v0/strategy/interp.rs:580-586` | main | 单元素链（qujiantao §4.8）：成本门接入前须先有长度>1 链（N-6）；登记依赖 | #1441/TB-02-X |
| Rust | `rust/src/economic_session/service.rs` | main | 新增 cost_stop_policy 记录与定位请求的四态输出（NotEvaluable/InsufficientEvidence/Fail/Enabled）与 L0Margin 查询 | #1441 |
| Lean | `formal/Origin/SubLevelDescent.lean:102-107` | main | 线段为递归底；不改 | — |
| Lean | `formal/Origin/NestingCertificate.lean:185-190` | main | 不改；成本门无定义式，登记 | — |
| Python | `s_session/economic_query.py:72` | main | 透传 cost_stop_policy / L0Margin / DescentBottomEvidence typed 结构 | #1441 |
| Python | `src/newchan/server.py:432` | main | 旧 HTTP 端点默认 false 的下钻开关：旧入口登记 | TB-10 |
| 文档 | `docs/adr/0035（编号待根分配）` | main | 新建 |  |
| 文档 | `.chanlun/definitions/qujiantao.md` | main | §4.5.3 第 1 条同步 #907 后裁；v1.3；三行头 |  |
| 文档 | `CONTEXT.md` | main | 词条：成本停止 policy |  |

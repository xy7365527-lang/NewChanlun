# 主控接线修复（2026-10-17）

名分：绑定 #1419 / #1488 的工作草稿。源码未编译，Rust 测试未运行；不构成远端或全图验收。

1. #1488：shared_parent 的全父账峰由 `resources.peak() > validation_limit` 改为 `resources.peak() > self.limit`。前者已扣 raw/source/set，会重复扣预算。原 candidate 保留费对缩小限额及非共享路径门均保留。独审在 `1488-raii-migration-plan.md` 已复读此修复。
2. #1419：CenterPair 目录仅对具版本的 `structure-classification/1` 请求按第二元具名 CC 归属原因。保留旧域/旧请求解析；解决 CC011/016 前缀丢失，以及 CC013/014 同主体不同判定混同。新增路由回归源码，工程独审已复读目录逻辑。
3. #1419：历史 V2 oracle 的 A/B 投影现先断言源域为 RULE_REVISION，再显式将投影 metadata 映射到历史 V3_RULE_REVISION；封存数据库、原始两账及业务载荷不改。保持13轴A/B对拍，不删对拍或改生产目录回固定表。更新注释明确投影边界，待独审；不能把投影后的字节称为未经映射的封存原件。

以上三文件均用单文件 rustfmt --edition 2021 --config skip_children=true 解析/格式化通过。最初一次 rustfmt 未给edition而拒绝C字符串；改为仓库Cargo.toml的2021版后通过。这不是Rust类型检查或测试证据。

## 当前源码指纹

- 1419目录：`.sandcastle/worktrees/resume-1419/rust/src/bin/s_session_v2/tb02a_facts.rs` SHA256 `f33aa9e513191f689b18539b06faf97b159484bbe9f8d88aefef5f8a92d4c5b4`
- 1419生成接线及测试：`.sandcastle/worktrees/resume-1419/rust/src/bin/s_session_v3/generation.rs` SHA256 `471825f38f5a41796346b286386d64f5ad25e06bfbd01ac51ea77248bdda4c0f`
- 1488父帽：`.sandcastle/worktrees/resume-1488/rust/src/bin/s_session_v3/audit_owner.rs` SHA256 `783555112e5863990d101ce379e8bdd16322e57c40e289e25e17635a68f64bc9`

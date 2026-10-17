<!-- goal1323-c6a-pagination-approved-20261017 -->
2026-10-17 用户明确批准 C6a 完整读取协议修订，原话：**“批准按提案实施”**。这是 #1406 承接的 SPEC #1340 追加实施决定，不改写原签署记录，也不代表验收完成。

批准的具体提案为 `c6a-wire-revision-proposal.md`，SHA256 `9edcded8451f1edb20cc45c4a3752cb3741556d37fded08c49e04de52bbdc6b6`，本机工作草稿目录 `.chanlun/review-results/goal1323-resume-20261017/`。容量矛盾证据见 https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6226271522 。

批准范围：

- owner→Q→验收客户端贯通固定 cut 的完整 canonical 字节分页；金融写语义、持久原件与账目 schema 保持。
- 全部 WE5 20 + WE7 21 业务步骤及每步观察、完整证明/原字节相等断言保持。新增只读传输轮次另账，原件全部可追。
- 每物理帧仍不超过 1 MiB；单逻辑文档及共享保留缓存受现有 4 MiB 总帽；活跃 watch 同账，最多 4 项，原 old_fixed/token 正控不得因淘汰策略被削弱。
- 每次逻辑读的全部分页、重试与核验共用原 30 秒绝对截止，不按页续窗；case/work、外层端点、RSS/磁盘/隔离/恢复及原件要求保持。
- 单页没有 CompleteVector 资格；全部片经独立核验、严格解码后才交原业务断言。跨 cut/epoch、缺页/错 hash/超帽均拒绝，不能用旧单帧臂替代新协议失败。
- 实施与必要验证获准；**不批准扩大资源帽或合入 main**。原运行准备、精确 source/tree、独立评审与完整验收门保留。

主控正在冻结具体接口并并行实施 Rust owner、Q/HTTP 和独立客户端/离线核验。当前未执行真实产品，未获得资源可行性或完整 41 步成功证据；所有 READY 仍关闭。#1406、#1340、#1323、#1279 均保持开放。

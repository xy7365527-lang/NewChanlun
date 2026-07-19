# 11 关验收台账（2026-07-18，CC 主控）
依据 goal：已收关以落盘产物直接验收、不重做。「待核」= 产物在但本主控尚未独立核验。

| 关 | 产物/证据 | 状态 |
|---|---|---|
| ① p116 存在性探针 | chanlun/review-results/p116-turnpoint-anchor-existence-20260717.md | done（落盘验收） |
| ② BSP 侧口径修复 | p117 四份设计 + p92-postfix-regression-20260717.md；终验=p124 S=8 重放对账 | 终验在途（C-7 s2 重放→归并→C-3 对账） |
| ③ 877 池 + U1–U3 | chanlun/escalate/bsp-terminal-endorsement-ruling-20260718.md + nest-leftover-rulings-u1-u3-20260718.md | 裁定落盘；数值终验并入 C-3 |
| ④ 小转大分支 | p118-xiaozhuanda-branch-design-20260718.md | 设计+实装在（econ_positive.rs 二通道准入门 GateCertificate::Xzd gate_pass，静态核验）；单测运行待 cargo 解禁 |
| ⑤ 嵌套子声部+双账本 | p120-nested-voice-dual-ledger-design-20260718.md | 设计+实装在（backtest/dual_ledger.rs，头注引 p120 §4，bit-exact 嵌入恒等 D8 对拍锁，静态核验）；单测运行待 cargo 解禁 |
| ⑥ recover 触发裁定 | chanlun/escalate/recover-trigger-nesting-ruling-20260718.md | done（落盘验收） |
| ⑦ A10 成本模型 | p121-a10-cost-model-design-20260718.md + a10-cost-model-ruling-20260718.md | 设计+裁定+实装在（strategy/ledger.rs η修正四桶 A10-C5(b)、config.rs κ 接口冻结附则A、机制真实装/费率待标定 waiver；守恒断言：dual_ledger.rs:88 fee 口径 + econ_positive.rs:2677 dx 守恒，静态核验）；单测运行待 cargo 解禁 |
| ⑧ M7 witness | — | 排队（C-4 实装 → C-5 跑批，cargo 解禁后） |
| ⑨ M8 Π_max-full 四层 | — | 排队（C-5） |
| ⑩ 090 修复 | 090-mirror-fixes-20260718.md | done（镜像层：条目1 证据链完整；条目2 含 git 对象库逐字恢复的 227 行补档草稿+解禁双情形边界说明；主仓正式补丁待解禁） |
| ⑪ 端到端两文档 | roadmap E2E 段 + doc-divergence-endtoend-prototype-20260718.md | done（0删/21增、编号25一致、锚8/8抽真/54全定位、M7/M8 逐字一致） |

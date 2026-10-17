# #1528-P2/P3：完整事件 tape 需求

## P2 事件采集

需要在 theta_v0 生产分类与 pi_Theta fill loop 建立只读、确定性事件 sink，并以 bar 序号关联：

1. classification：笔、段、中枢及确认状态；
2. pi_Theta signal：七链决策、门状态、最终 action；
3. order：订单方向、数量、类型、执行 bar；
4. position：逐 bar 持仓与变化，区分多空/分腿；
5. ledger：成交、费用、现金、权益、账本守恒证据。

事件编码必须使用固定字段顺序、整数 tick 或 IEEE bit 编码，并对 HashMap/声部集合排序；双跑输出逐字节比较。

## P3 对拍与依赖

P3 在 P2 tape 稳定后扩展 replay_compare：每类独立 canonical hash，报告首个分歧 bar 与字段，并保留有限摘要作为汇总交叉检查。依赖生产 fill loop 的事件出口、持仓唯一来源和账本唯一写入点；不能用现有 D4/p123 comparator 代替。

## 工作量估算

P2：约 2–4 个实施日，覆盖 sink、canonical encoder、单元/合成回放锁及生产路径接线。P3：约 1–2 个实施日，覆盖 CLI、双跑报告、差异定位和 ES/GC 样本验收。实际工期取决于现有 fill loop 是否允许无副作用观测。

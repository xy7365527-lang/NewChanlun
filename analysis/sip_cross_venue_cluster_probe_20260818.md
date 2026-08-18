# 跨所同笔多报聚类探针（#1064 S-a4，2026-08-18）

- 状态：**推论层（实施交付）**。数据实测，喂 #1064 裁决。
- 样本：AAPL 7 交易日 raw 5,734,269 行（#1051 同款窗口）。

## 探针一：同笔跨所共享 sip_timestamp？（假设 A）

判据 1+2（条件码黑名单 + 丢 TRF）后 2,722,118 行，**每行 sip_timestamp 全唯一（0 组共享）**。
⟹ **假设 A 证伪**：同笔跨所打印不共享 SIP 收报时间戳（各 venue 打印 = 各自 SIP 消息）。

## 探针二：Massive 有 SIP consolidated 打印吗？

`/v3/reference/exchanges`：SIP 类型 id = [5, 13, 314]，TRF = [4, 6, 16, 201, 202, 203]。
样本中 **SIP 行数 = 0**（top 是 TRF id=4 占 52%，判据二已丢）。
⟹ **Massive REST /v3/trades 是纯 per-venue+TRF 形态，无 consolidated 打印**，
23 倍基线（对标 Databento SIP 全合并）的官方语义**不可从 Massive 复现**。

## 探针三：模糊聚类可达多少？（2026-08-14 单日）

判据 1+2 后 227,020 行。1ms 窗内同价邻居对 1,252,766（551%——同价连续成交噪声极大）；
同价且同 size 的对 182,810。贪心归并粗估 → **44,210 行 ≈ raw 的 14.1 倍压缩**。

**代价照实**：同价同 size 的 1ms 邻居大量是**真实不同笔**（限价队列同价同量爆发），
误并率无法自证——Massive 无同源 ground truth。**唯一可对拍源 = Databento EQUS.MINI
（2023-03 起 consolidated，D1 裁定的对拍素材，17.55GB 已拉）**：用重叠段把「聚类判据产出」
vs「官方 consolidated」做 precision/recall 对拍，读数过关才可定稿第四判据。

## 裁决选项（给编排层）

- (a) 接受三判据 2.1x：正本 ≈250-380GB，「consolidated」重定义为三判据口径；
- (b) 上模糊聚类第四判据：先开对拍探针（EQUS.MINI 重叠段 precision/recall），过关才接线；
- (c) 个股线正本改 Databento consolidated（2018+/2023+ 短史）——违背 D1 2003 全史，不推荐。

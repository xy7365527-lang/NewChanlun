# SIP 消歧键标定记录（#1051，2026-08-18）

- 状态：**推论层（实施交付，定稿依据）**。数据实测，喂 #1041 草案 §4 定稿。
- 票：issue [#1051](https://github.com/xy7365527-lang/NewChanlun/issues/1051)（S-a3：消歧键标定执行）。

## 样本

- 标的：AAPL；7 个交易日（2023-09-05/06/07/08 + 2026-08-12/13/14）；
- 源：Massive REST `/v3/trades` 原样落盘（#1040 管线 `scripts/fetch_massive_tick.py`）；
- raw 总量：**5,734,269 行**；标定读数样本（三判据滤后）2,722,118 行。

## 双键读数（`scripts/consolidate_massive_tick.py --calibrate-key`）

| 键 | 行数 | 唯一键 | 重复率 | 组内价格一致率 |
|---|---|---|---|---|
| A `(sip_timestamp, sequence_number)` | 2,722,118 | 2,722,118 | **0.0%** | **1.0** |
| B `participant_timestamp` | 2,722,118 | 1,855,835 | 31.8% | 0.970 |

B 键 3% 的撞键组内价格不一致 = 跨所同 participant_timestamp 的不同成交被误并。

## 裁定

**消歧键 = A `(sip_timestamp, sequence_number)`。** 全唯一、价格一致率 1.0。
已回写 #1041 草案 §4 与决策表。

## 连带实测：正本/raw 行数比

三判据 + A 键全开后 **2.11**（#1030 基线 ≈23，对标 Databento SIP 全合并）。
差 11 倍 = 跨所同笔多报残留，第四判据（同笔跨所聚类）另票 #1053。
体积预期照实：正本 ≈250-380GB（不做第四判据）vs 原预期 30-50GB。

# SIP consolidated 去重/条件码过滤口径——草案（#1041 实施侧交付）

- 状态：**草案（推论层）**，非定稿。定稿（ADR 0023 §四-1 补节）须等 #1040 数据落地后跑
  `scripts/probe_sip_dedup.py` 出数，编排层裁。
- 票：issue [#1041](https://github.com/xy7365527-lang/NewChanlun/issues/1041)
- 来源：ADR 0023 §四-1（去重前移到落盘层，SIP 语义，引擎前置）
- 本文件是「裁完的结果」的候选稿，落地时进 ADR 0023 补节（纯工程口径，不涉缠论语义）。

## 0. 结论一句话

原样 Massive tick（13+1 字段）→ 三判据流水（每条单一、不兜底）→ consolidated 落盘：

```
条件/修正过滤（黑名单）→ 主所判定（丢 trf_id 非空）→ 同笔多报去重（单一消歧键）
```

三条判据各管一件事，互不兜底（#799 限定词一「不同判断」）；消歧键是**唯一参数**，
候选键由探针实测定一个，不是「A 失败换 B」的宽严两档。

## 1. 三件决策的现状

| # | 决策 | 草案口径 | 状态 |
|---|---|---|---|
| 1 | 主所判定 | 丢 `trf_id` 非空（TRF/暗池打印），保留主所打印 | 文档语义已核，**待数据对拍** |
| 2 | 条件码过滤 | 黑名单（19 码）+ correction 黑名单（4 值），其余全保留 | 文档语义已核，**待数据对拍** |
| 3 | 消歧键 | `(sip_timestamp, sequence_number)` vs `participant_timestamp` 二选一 | **未定稿**（实测数据定，见 §4） |

## 2. 主所判定（决策 1）

**权威源**：Massive `/v3/reference/exchanges` 的 `type ∈ {exchange, TRF, SIP}`（交易所 /
贸易报告设施 / SIP）；KB FAQ 逐字「If a trade comes through with an exchange ID of 4
(exchange:4) and also has an attached "trf_id" field, it came from a dark pool」。

**草案口径（单一判据）**：`trf_id` 非空 ⟹ TRF/暗池打印 ⟹ 丢。保留主所打印（`trf_id` 空）。

- 选 `trf_id` 而非「查 exchange 码表判 type」：字段已随数据落地、无需二次查表，且与
  `type=TRF` 同义（TRF 打印必带 trf_id）。两者等价，取字段侧是为了判据落在一处。
- 代价照实：场外（暗池/内化）成交若**只有** TRF 一份上报、无主所打印，丢 TRF 会损失该
  部分真实成交量。是否值得保，由探针的 `trf_matched_ratio`（TRF 打印里与主所同笔的比例）
  读数决定——见 §4。

## 3. 条件码过滤清单（决策 2）

**权威源**：Massive glossary `https://massive.com/glossary/us/stocks/conditions-indicators`
的 Trade Conditions 表（逐码含 Update High/Low / Update Last / Update Volume 三列）与
Trade Corrections 表（correction 字段编码）。

**取舍：黑名单**（不是白名单）。理由：「修正/撤单/非标准」的码集合小且语义明确，保留集
= 其余全部——含 Odd Lot(37)、Form T(12)、Cash Sale(7)、Next Day(20) 等**真实成交**（只是
规模/结算/时段不同），tick→Bar 不丢真实成交。白名单（如「Update Last = Yes」）会把它们
误杀。

### 3.1 条件码黑名单 `DROP_CONDITIONS`（19 码，逐条 glossary 出处）

| 码 | 名称 | 丢的理由（glossary 语义） |
|---|---|---|
| 2 | Average Price Trade | 均价，非单笔成交价 |
| 10 | Derivatively Priced | 衍生定价，非报价驱动 |
| 15 | Market Center Official Close | 官方收盘合成值，非成交 |
| 16 | Market Center Official Open | 官方开盘合成值，非成交 |
| 22 | Prior Reference Price | >90s 前参考价，执行时间≠报单时间 |
| 38 | Corrected Consolidated Close | 收盘修正，非成交 |
| 39 | Unknown | 未知 |
| 42 | NonEligible | 非合格，不入 consolidated 带 |
| 43 | NonEligible Extended | 延时段非合格 |
| 44 | Cancelled | 撤单 |
| 45 | Recovery | 恢复 |
| 46 | Correction | 修正 |
| 48 | As of Correction | 截止修正 |
| 49 | As of Cancel | 截止撤单 |
| 50 | OOB | 越界 |
| 51 | Summary | 汇总 |
| 54 | Errored | 错误 |
| 56 | Placeholder | 占位（Update 列 TBD） |
| 59 | Placeholder for 611 exempt | 占位（Update 列 TBD） |

> 备注：15（Official Close）与 16（Official Open）是「官方开/收盘**合成值**」（Update
> High/Low/Last 三列皆 No），列黑；17/18/19（Opening/Reopening/Closing **Trade**）是
> 真实成交（REG NMS 611b3 单笔定价成交），**保留**。这条边界待数据对拍复核。

### 3.2 correction 字段黑名单 `DROP_CORRECTIONS`（4 值）

glossary「Trade Corrections」表逐字（两位编码，REST `correction` 为 integer，同值）：

| 值 | 语义 | 处置 |
|---|---|---|
| 0/00 | 原始成交，未修正/撤单/错误 | 保留 |
| 1/01 | 原始成交（迟修正，含修正后数据） | 保留 |
| 7/07 | 原始成交（后被标为错误） | **丢** |
| 8/08 | 原始成交（后被撤单） | **丢** |
| 10 | 撤单记录（跟 08） | **丢** |
| 11 | 错误记录（跟 07） | **丢** |
| 12 | 修正记录（跟 01，含原始错误数据） | 保留（草案），见下 |

- `12`（修正记录，含「原始错误数据」）草案暂**保留**、由消歧键与 01 折叠；若数据对拍
  显示 12 与 01 成对且 12 价格错误，改列黑。**此项是草案里唯一「保留但存疑」的码**，090
  照实。

## 4. 消歧键（决策 3）——未定稿，实测数据定

**权威源**：Massive REST trades 字段语义——

- `sequence_number`：逐票（per ticker）唯一、逐日重置、未必连续；
- `sip_timestamp`：SIP 收报时间（ns）；
- `participant_timestamp`：交易所生成时间（ns），「同一笔多报」在各 venue 上共享。

**候选**：

- A `(sip_timestamp, sequence_number)`——SIP 收报锚 + 序列号，逐票唯一/日的强键；
- B `participant_timestamp`——交易所生成时间锚，同笔多报各 venue 同值。

**定稿判据（探针出数）**：`scripts/probe_sip_dedup.py` 对 5 交易日 × 3 标的报
`dup_by_key`（唯一组数 / 重复率 / 组内价格一致率）。选「重复率读数稳、且组内价格一致率
≈ 1（不误并不同价成交）」的键；若两键读数差异不显著，取 A（含 sequence_number，唯一性
更强，抗同 ns 多笔）。

**状态**：本沙盒无 Massive 数据（#1040 未落盘、无 `MASSIVE_API_KEY`），探针跑不出实证
读数，**本件不定稿**。数据落地后跑探针，读数贴回本票评论，编排层裁。

## 5. 实施清单

已交付（本提交 `#1041`）：

| 文件 | 内容 |
|---|---|
| `src/newchan/sip_consolidate.py` | 三判据流水纯函数（filter/drop_trf/dedup/consolidate）+ 黑名单常量 |
| `scripts/probe_sip_dedup.py` | 探针：per-venue 重复率 + 条件码/correction 占比 + 消歧键对拍 |
| `tests/test_sip_consolidate.py` | 修正规则测试锁（合成 fixture，consolidated vs 原样行数差 = 重复率） |

后续（待 #1040 数据落地）：

1. 跑 `uv run python scripts/probe_sip_dedup.py`，出 5×3 读数；
2. 用读数复核 §3 黑名单（各条件码占比是否与 glossary 语义一致）+ `trf_matched_ratio`
   定「丢全部 TRF」还是「TRF 独一场外量保留」；
3. 用 `dup_by_key` 读数定消歧键（§4），把 `consolidate()` 默认键写死；
4. 定稿落 ADR 0023 §四-1 补节（正本层），本草案文件留作历史（不追溯更新）。

## 6. 诚实残余（090）

- **无实证读数**：本沙盒无 Massive 数据且无 `MASSIVE_API_KEY`，#1040 未落盘——per-venue
  重复率、条件码占比、消歧键对拍三件**均未出数**，本文 §2/§3 是文档语义草案、§4 未定稿。
- **correction 编码对拍未做**：glossary 是两位编码（00/01/07/08/10/11/12），REST
  `correction` 是 integer，二者同值推断待真实数据核实。
- **条件码黑名单是手编清单**：虽逐条有 glossary 出处，但「16/17/18/19 一族只黑 15/16」
  等边界是语义判断；定稿前应据 `/v3/reference/conditions` 的 `update_rules`/`type` 机械
  复核（该端点需 key，本沙盒不可达）。
- **`exchange` 码表未落仓**：主所判定用 `trf_id` 字段绕开了查表，但 per-venue 统计与
  「主所 vs TRF」复核仍需 `/v3/reference/exchanges` 的 id→name/type/mic 表，待数据侧一并
  缓存。

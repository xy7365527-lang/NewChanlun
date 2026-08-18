# SIP consolidated 正本产出步 + raw staging 处置 + 消歧键标定（#1048 实施侧交付）

- 状态：**推论层（实施交付）**。代码与测试已落；消歧键定稿须实测数据（本沙盒无 raw staging、
  无 `MASSIVE_API_KEY`，见末尾「诚实残余」）。
- 票：issue [#1048](https://github.com/xy7365527-lang/NewChanlun/issues/1048)（S-a2：consolidated 正本产出步）
- 链：ADR 0023 · D4 #1033 · #1040（raw staging）· #1041（三判据草案）· 实施总单 #1049
- 上游草案：`analysis/sip_consolidated_dedup_rule_draft.md`（#1041，本票不追溯更新它）

## 0. 结论一句话

`#1040` 原样落盘（raw staging）→ `#1041` 三判据流水（`filter_trades` → `drop_trf_prints`
→ `dedup_by_key`）→ `#1048` 正本落盘（consolidated），raw 在正本产出且校验通过后转
transient 处置（保留期后人审删除）；消歧键二选一（`(sip_timestamp, sequence_number)` vs
`participant_timestamp`）已接线为「标定器 + 决策规则」，**最终定稿待实测数据**。

## 1. consolidated 正本产出步（#1048 第一件）

- 接线脚本：`scripts/consolidate_massive_tick.py`（本票新增）。
- 数据流：`analysis/data_cache/massive_tick/{TICKER}/dt={YYYY-MM-DD}/trades.parquet`
  （raw）→ `consolidate()` → `analysis/data_cache/massive_tick_consolidated/{TICKER}/dt={YYYY-MM-DD}/trades.parquet`
  （正本，Parquet ZSTD，13+1 字段全保留，列序不变）。
- 可复现契约：
  1. `consolidate()` 确定性（稳定排序 + `keep first` + 恢复原行序）；
  2. 正本分区已存在且行数>0 则跳过（幂等重跑）；
  3. 落盘原子写（临时文件 + rename）。
- 消歧键 `--key` **必选**（`sip_seq` / `participant`），不替操作者选键——定稿前不落静默默认。
- 验收口径（#1048 票面）：十标任选 1 标的若干日 raw → `filter_trades` → 正本分区可复现；
  **正本行数 ≈ raw/23 量级**（#1030 实测比）——即 per-venue 多报去重后每笔成交留一份，
  `duplicate_rate ≈ 1 − 1/23 ≈ 0.9565`。该比值是**验收读数 + 异常告警**，不是自动 retire
  的硬闸（硬闸只查「正本非空且行数 ≤ raw」，见 §2）。
- 测试锁：`tests/test_consolidate_massive_tick.py`（离线合成 fixture，22 项，锁行数折叠、
  schema 保留、幂等重跑、确定性、retire 闸、键列校验、无数据不编数、坏分区跳过）。

## 2. raw staging → transient 处置策略（#1048 第二件）

**名分**：raw staging 是 transient 中间层，不是正本。D4 拍 A 的本意 = 常驻的是 consolidated
（≈30-50GB），不是 per-venue 全量（500-800GB）。raw 只作产正本的过程件。

**处置时序**（三步）：

1. **产正本**：单标的单日 raw 分区跑 `consolidate()` 出正本分区。
2. **校验闸**（`verify_partition`）：正本分区存在、非空、行数 ≤ raw（不超原样）。未过闸的
   raw 分区**保留**（不能动）。
3. **转 transient**：`--retire-raw` 把过闸的 raw 分区**移动**到
   `analysis/data_cache/massive_tick_raw_transient/`（可逆移动，**不删除**）。

**保留期（grace）**：正本产出后 raw 保留 ≥ 7 个自然日（覆盖人工对拍/回查窗口），期满由人审
删除（transient 区 `rm -rf` 或等后续自动清理票）。保留期写进策略、不写进自动删除——避免
自动化误删数据。

**对拍素材**：EQUS.MINI 17.55GB 已定位为 Massive↔Databento 对拍素材（ADR 0023），不因 raw
清理受影响；如需另留 raw 样本，保留每标的最近 1 个交易日分区即可，其余走 transient。

**代价照实**：transient 区同样占盘，保留期后须清空；自动删除（定时清理保留期前已 retire 的
分区）本票不落，留后续。

## 3. 消歧键标定（#1048 第三件）——接线已落，定稿待实测

### 3.1 候选与语义

| 键 | 语义（#1041 草案 §4 已核） | 预期行为 |
|---|---|---|
| A `(sip_timestamp, sequence_number)` | SIP 收报锚 + 逐票序列号 | 每个 venue 上报各占一个 (sip, seq) ⟹ **不折叠**主所多报（dup_rate ≈ 0） |
| B `participant_timestamp` | 交易所生成时间锚 | 同笔多报各 venue 同值 ⟹ **折叠**主所多报（dup_rate ≈ 1 − 1/23） |

离线合成 fixture（`test_key_readings_participant_collapses_sip_seq_does_not`）坐实：同一笔
成交经 3 个主所打印（`participant_timestamp` 相同、`sip_timestamp`/`sequence_number` 各不同），
B 键 6→2、A 键 6→6。**这是决策规则的行为底座**，不是实测读数。

### 3.2 标定器与决策规则（已接线，机械版）

- 标定器：`scripts/consolidate_massive_tick.py --calibrate-key [--days N]`——对每标的最近 N
  日分区，在 filter + venue 之后的帧上跑双键读数（`dup_rate` + `price_agree_rate`），聚合后
  按 §3.3 出推荐，读数落 `analysis/data_cache/massive_tick_consolidated/_dedup_key_calibration.json`
  （含逐分区明细；该路径在 gitignore 数据区，不入仓）。
- 读数口径与 `#1041` 探针（`scripts/probe_sip_dedup.py`）一致，区别只在「读 filter+venue
  之后的帧」——那才是 `dedup_by_key` 的真实输入。

### 3.3 决策规则（`recommend_key`，#1041 §4 定稿判据的机械转写）

1. 两键都要有读数（缺样本 → 不定稿）；
2. `price_agree_rate < 0.999` 的键判「误并不同价成交」，出局；
3. 恰一个键合格 → 推荐它；
4. 两键都合格：重复率差异显著（> 0.05）→ 取重复率更高者（同笔多报折叠更充分）；差异不显著
   → 取 A（#1041 §4 平手条款：含 `sequence_number`，唯一性更强，抗同 ns 多笔）。

### 3.4 状态与未定项

- **未定稿**：本沙盒无 raw staging 数据（`analysis/data_cache/massive_tick/` 不存在）且无
  `MASSIVE_API_KEY`，标定器跑不出实测读数 ⟹ **消歧键不能据实数定稿**。`consolidate()` 默认键
  维持 `#1041` 落地的 `KEY_SIP_SEQ`（参数化，未写死新的默认）。
- **待数据落地**：跑 `--calibrate-key`，把 `_dedup_key_calibration.json` 的读数贴回本票，
  编排层按 §3.3 裁；定稿时把 `consolidate()` 默认键写死 + 回写 ADR 0023 §四-1 补节 +
  `sip_consolidate.py` 模块头「诚实声明」销账。

## 4. 诚实残余（090）

- **无实证读数**：正本行数 ≈ raw/23、per-venue 重复率、消歧键对拍三件均未在真实数据上出数；
  本文 §1 的 1/23 是 #1030 实测比的验收预期、§3.2/§3.3 是决策规则接线，不是实测结果。
- **retire 无自动删除**：策略只有「校验闸 + 可逆移动」，删除是保留期后人审动作。
- **标定 JSON 不入仓**：标定读数属数据衍生物（gitignore 数据区），人读记录以本文 §3 为凭。

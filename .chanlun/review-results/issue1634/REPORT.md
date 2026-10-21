# #1634 诊断：全史 BTC 从文件首根起坍缩为单组

## 结论

成因是**实现违背正本**，不是数据缺陷。main 的包含状态（紧凑 `IncrFactsState::append` 与完整事实 `process_inclusion_with_facts`，`rust/src/theta_v0/parser/inclusion.rs` main `df6e680930` :627–643 一带）在**首对 raw 互含**时置 `initial_direction_unsettled = true`，此后每根都走 `waiting` 分支 `continue`，再也不检查相邻 raw 非包含严格对 ⟹ 整条序列只剩首根一个组（`merged=1 fractals=0 strokes=0`）。

`.chanlun/definitions/baohan.md`「初始方向」（#1342，v1.5 起；现 v1.7）要求：首个相邻 raw 非包含严格对出现的 cut，以其方向自序号 0 左折叠并发布；只有**始终等不到**该对才保持未定。BTC 全史在下标 1→2 就有严格对（Up），之后严格对遍布全序列，按正本绝不可能是单组。#1342 受影响代码清单早已点名此处（「紧凑状态的永久 unsettled（`:843` 遇包含即停）→ 非包含对出现即解锁并自起点重折」），归 #1393 TB-02-U。#1393 的 W 实装（`raw-initialization-w/1`）已在 #1419 整合分支 `codex/1419-1393-cfw-integrate-20261019`（`356df5db72`）完成，**尚未进 main**。

## 数据形态（首 130 根，`logs/head_shape.txt`，脚本 `head_shape.py`）

2017-08-17 04:00 起，时间戳逐分钟单调、无重复、无缺值；首 104 根中 86 根 O=H=L=C（恒价），31 根零量。这是币安开盘初期的真实稀疏成交，不是坏数据：相等区间按正本属包含，不需清洗。

## 坍缩判据：首对是否互含，与平盘段无关

`ISSUE1588_WINDOW=W` 的起点 = 4,613,599 − W + 5。#1629 探针的 8 个窗口，坍缩当且仅当起点处首对 raw 互含（`logs/head_shape.txt` 末段）：

| W | 起点 | 首对互含 | #1629 实测 |
|---|---|---|---|
| 4,613,599 | 5 | 是 | merged=1 |
| 4,613,590 | 14 | 是 | merged=1 |
| 4,613,500 | 104 | 否 | 3,037,848 |
| 4,613,000 | 604 | 是 | merged=1 |
| 4,610,005 | 3,599 | 是 | merged=1 |
| 4,600,005 | 13,599 | 是 | merged=1 |
| 4,400,005 | 213,599 | 是 | merged=1 |
| 4,000,005 | 613,599 | 否 | 2,702,328 |

起点 213,599 远离开盘平盘段仍坍缩，排除「数据首段缺陷」解释。

## 独立期望

`oracle.py`：逐字按 baohan.md 写的 Python 折叠（首个相邻 raw 非包含严格对定方向、自 0 左折叠，`contains`/`strict_dir`/上下合并同正本），tick 1e-8 量化。起点 104 得 3,037,848 组，与 main 实测逐数相同（该段两实现一致，证明 oracle 忠实）；起点 0 得 3,037,868、起点 5 得 3,037,866（`logs/oracle.txt`）。

## 复现与回归锁

新增 `rust/src/theta_v0/parser/issue1634_tests.rs`：

- `issue1634_btc_file_head_flat_prefix_does_not_collapse_to_one_group`：真实数据下标 5..=140 的 high/low 夹具（首对互含），紧凑 `parse_layer` 的合并序列须等于 Python 期望的 24 组，并与批量 `process_inclusion`、完整事实入口、S 生产入口 `parse_layer_with_group_evidence` 逐字段一致。
- `issue1634_full_history_from_file_head_matches_independent_oracle`（`#[ignore]`，需 `NC_BTC_FULL_JSON`）：全史起点 0/5/104 的组数等于 Python 期望，且等于 `process_inclusion`。

| 基线 | 结果 | 日志 |
|---|---|---|
| main `df6e680930` + 本测试（本地临时提交 `70e228a660`，未推送） | 2 FAILED：夹具得 1 组；全史起点 0 得 merged=1 | `logs/main-red-70e228a660.txt` |
| #1419 整合 `356df5db72` + 本测试（`5eebf3b71e`） | 2 passed；起点 0/5/104 = 3,037,868 / 3,037,866 / 3,037,848 组，笔 388,172 / 388,170 / 388,166 | `logs/int-green-5eebf3b71e.txt` |

远端 windows-5090 WSL，`nc-remote` 私有 target `issue1634-main` / `issue1634-int`，`cargo test --release --lib issue1634 -- --include-ignored`。本机未跑 cargo。

## 处置

不在 main 另写一份解锁实现：#1393 W 是该正本的获准实装，再写一份即同一判断的第二实现（#799）。本 PR 以 #1419 整合分支为基，只加回归锁与证据；#1393/#1419 合入 main 时锁随之生效。数据侧无需处置；ADR 0041 起点签署字段不需要——任何起点按正本都应正常解析。

Destination ④ 的含义：在 #1393/#1419 进 main 之前，main 上任何首对互含的起点（全史文件头即是）都会坍缩；全史回放须基于含 W 的代码。

## 残余

- 本锁在 main 上为红，直到 #1419 合入；PR 基分支为整合分支，非 main。
- 完整事实入口在整合分支上仍是 #1628/#1629 之前的 Θ(n²) 形态，全史读数锁只跑紧凑入口与批量核；夹具锁覆盖全部四个入口。

# #585 否则域多窗普查：一类点 T3-in-c 分档与影响面

- 日期：2026-07-28
- 票据：#585（地图 #582；并行工程口径票 #586）
- 性质：纯调研；主仓未写
- 回放冻结快照：`c8e87c4a0d056c039bb215347adf6c21e75c408c`
- 独立 worktree：`/tmp/research-585/wt-run`
- 独立 target：`/tmp/research-585/target`
- 实际报告：`/tmp/research-585/firstclass-otherwise-census-20260728.md`
- 票据期望归档位：`.chanlun/review-results/firstclass-otherwise-census-20260728.md`；因“主仓一律不写”硬边界，未落主仓

**统计口径**：在冻结快照 `c8e87c4a0d` 上，以本机
`analysis/data_cache/btc_1m_full.json`
（SHA-256 `16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`）
分别重放 M8 `p3fold/wf7/wf8`；分母为生产增量分类器逐窗新确认的每个
`buy1/sell1` 一类点事件。D0 test-only 旁路在
`signal::judge_segment` 的 `judge_first_cached` 成功点读取同一 last-B、`λ_C` 与
`seg.end`，并调用现有 `trend_third_class_in_c`；`c=[λ_C,seg.end]` 内扫描相邻段对，
任一紧邻对满足“leave 与趋势同向且严格破 B 核心，retest 反向且严格不重回核心”
即记 `trend_first`，否则记 `otherwise`。Reset 由
`center_lifecycle.jsonl` 按 `(level,side,trigger_src)` 精确连接；成交分母为本跑
`trades.jsonl` 行数，一类全平限定
`exit_type=CloseRoot ∧ trigger_bsp_class_at_exit=1`。分档后的 trades 只给“当下不消费
CloseRoot 行”的静态局部代理；未实装状态分支，故 `execR/R/MaxDD/LCB` 反事实均按 090
记“未能判定”。

## 0. 结论先行

| 窗 | 一类点 | c 内含 T3（趋势一类） | c 内无 T3（否则域） | 否则域占比 |
|---|---:|---:|---:|---:|
| p3fold | 87 | 20 | 67 | 77.01% |
| wf7 | 33 | 4 | 29 | 87.88% |
| wf8 | 59 | 21 | 38 | 64.41% |

三窗中，否则域均为多数。每窗的一类点与 Reset 按
`(level,side,source)` 全部 1:1 对账：`87/87、33/33、59/59`，未匹配均为 0。

否则域 Reset 真正对应到一类 `CloseRoot` 成交的数量很小：

| 窗 | 否则域 Reset | 一类 CloseRoot（全部） | 其中归属否则域 | 占全部 trades |
|---|---:|---:|---:|---:|
| p3fold | 67 | 2 | 2 | 0.1866% |
| wf7 | 29 | 1 | 1 | 0.0820% |
| wf8 | 38 | 4 | 1 | 0.0912% |

这说明“否则域 Reset 很多”不等于“当前持仓被一类全平很多”：大多数 Reset 到场时没有形成
`trigger_bsp_class_at_exit=1` 的 `CloseRoot` typed trade。

## 1. 教义边界与本次临时工程口径

### 1.1 37:18 的边界

原文与考据链支持的限缩读法是：

1. 讨论对象先是 A、B 为同级别中枢的标准趋势；
2. c 至少包含对 B 的第三类买卖点；
3. 不含时可看作 B 的小级别波动，按盘整背驰处理；
4. 即使含 T3，仍须满足 37:20 的新高/新低等后续条件。

来源：

- `docs/chanlun/text/blog/037-第37课.md:14-20`
- `.chanlun/review-results/text-status-third-before-first-20260728.md:3-19`
- `docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:230-253`

当前 `judge_first_cached` 在
`rust/src/theta_v0/classifier/signal.rs:289-404`
检查趋势门、破最后中枢、A/C 配对、37:20 新极值和 MACD 背驰，但不检查 T3-in-c。因此本普查
是在“不改现行一类点集合”的前提下，事后分为两档。

### 1.2 #586 尚未形成正式生效口径

截至本报告结束：

- GitHub #586 仍为 `OPEN`、无评论；
- 并行调研稿
  `.chanlun/review-results/t3-in-c-caliber-20260728.md`
  已出现，但仍是未入仓研究产物；
- 该稿建议补充十五式“旧框右边固定第一对，首对失败不后扫”，并指出当前
  `trend_third_class_in_c` 是“全 c 窗后扫，取首个成功对”，两者不是同一谓词。

故本报告严格执行 #585 票面和用户本次指定的临时口径：使用现有扫描器。这里的
`trend_first/otherwise` 计数不能冒充未来 #586 固定首对口径的终值；若 #586 正式裁定固定首对，
须用五桶失败分类重新跑本普查，当前 `trend_first` 可能下修。

## 2. 可用 M8 窗口与数据源

### 2.1 `M8_WIN_FILTER` 有效窗

`m8_e2e_all_systems_oos` 当前只构造三项：

| 标签 | 闭区间 | bar 数 | 构造来源 | 说明 |
|---|---|---:|---|---|
| p3fold | 2023-01-01 ～ 2023-06-30 | 260,560 | `wverify_run.rs:1219-1222` | M8 硬编码单折 |
| wf7 | 2023-02-17 ～ 2023-08-16 | 260,560 | `wverify_run.rs:1223-1226`、`prereg_windows.rs:85` | BTC anchored i=7 |
| wf8 | 2023-08-17 ～ 2024-02-16 | 264,960 | 同上、`prereg_windows.rs:86` | BTC anchored i=8 |

`M8_WIN_FILTER` 仅按字符串 retain（`wverify_run.rs:1228-1232`）。其他字符串不是第四个可用窗，
而会得到空清单。另：p3fold 与 wf7 在 2023-02-17～2023-06-30 重叠，三窗统计不能机械合并成
独立样本。

### 2.2 原始数据

三窗共同读取：

- `analysis/data_cache/btc_1m_full.json`
- 大小：329,485,099 bytes
- mtime：2026-06-25 10:27:51
- SHA-256：`16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`
- 路由：`rust/src/theta_v0/backtest/data.rs:50-70,106-155,288-297`

该文件受 `.gitignore` 排除，故 hash 是复现封印的一部分，不能只登记文件名。

### 2.3 现存转储与本次新封印

| 窗 | 本次 current-snapshot 转储 | 其他现存来源 | 可用性 |
|---|---|---|---|
| p3fold | `/tmp/research-585/results/p3fold/` | `/tmp/wt527fix-post-m8-p3fold/`；`/tmp/wt578-post/chanlun/review-results/t5a_chain_dump_p3fold.jsonl` | 本次目录含 census、四条 OPSEM 流、M8 报告；旧目录无完整封印/last-B |
| wf7 | `/tmp/research-585/results/wf7/` | `/tmp/wt527fix-post-m8-wf7/`；`/tmp/wt578-post/chanlun/review-results/t5a_chain_dump_wf7.jsonl` | 同上 |
| wf8 | `/tmp/research-585/results/wf8/` | `/tmp/main542-on.2jRQ67/`；`/tmp/research-565/full-third.jsonl`；`/tmp/wt527fix-post-m8-wf8/`；T5a wf8 dump | #565 四流只覆盖旧快照/wf8；本次重跑供 current-dispatch 快照主证 |

其他残留：

- `rust/target/opsem_new/{trades,tower_events}.jsonl` 无窗口/commit/lifecycle 元数据，不可用于 #585；
- `/tmp/m8_e2e_all_systems_oos.md` 是共享覆盖路径，本次已逐窗复制到各结果目录。

## 3. D0 旁路与中性验证

### 3.1 旁路落点

临时探针仅存在于 `/tmp/research-585/wt-run`：

- `rust/src/theta_v0/classifier/firstclass_census_probe.rs`
- `rust/src/theta_v0/classifier/mod.rs`：test-only 登记当前 classifier level
- `rust/src/theta_v0/classifier/signal.rs`：在 `judge_first_cached` 已返回、`points.push`
  之前只读外化

门：

```text
FIRSTCLASS_CENSUS_DUMP=/tmp/research-585/results/<window>/firstclass.jsonl
```

未设门时立即返回；设门时仅输出，不把 T3 结果回馈 BSP、lifecycle、策略或执行层。每行包含
`level/side/confirm_src/A span/λ_C/seg.end/last-B/T3 pair/class`。

### 3.2 验证结果

- `trend_third_class_in_c_geometry`：1 passed，0 failed；
- 三个 M8 窗：各 1 passed，0 failed；
- census 行不变量检查：`confirm_src==seg.end`、`B.end<=λ_C<=seg.end`、命中 pair 落在 c 窗内、
  `Long↔Down/Short↔Up`，三窗合计 179 行违例 0；
- wf8 env on/off：
  - `trades.jsonl`：`cmp=0`
  - `tower_events.jsonl`：`cmp=0`
  - `center_lifecycle.jsonl`：`cmp=0`
  - `rebase_observability.jsonl`：`cmp=0`
  - M8 Markdown：`cmp=0`

wf8 四流 on/off SHA-256：

| 流 | SHA-256 |
|---|---|
| center_lifecycle | `083fa44978265412147524d7edce00f408e5d38861848b211f2ad8537e6b6dc6` |
| rebase_observability | `a4223a17b7ff8629919fb08bc94466acbcdbc8c120217a0762d6d8c8cbe10f83` |
| tower_events | `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` |
| trades | `5d90baeaaf2da5a76128844b96a7427cd882f0721cfbad745c2e401bf3f5eecc` |

### 3.3 快照漂移声明

回放 worktree 冻结在任务开始时的 `c8e87c4a0d`。运行结束时主仓 `main` 已由其他会话推进到
`06689c71a130a61ddf4c141f15134bb93faf207f`（#526，提交说明为纯测试）。本报告不把旧 worktree
结果冒充新 HEAD 封印；若要求 `06689c71a1` 的形式复现，仍应在新独立 worktree 重跑。

## 4. 逐窗普查

### 4.1 分窗总表

| 窗 | 趋势一类 | 否则域 | 合计 | 趋势占比 | 否则占比 |
|---|---:|---:|---:|---:|---:|
| p3fold | 20 | 67 | 87 | 22.99% | 77.01% |
| wf7 | 4 | 29 | 33 | 12.12% | 87.88% |
| wf8 | 21 | 38 | 59 | 35.59% | 64.41% |

### 4.2 分侧、分级、分窗

下表只列出现过一类点的组合；未列组合计数为 0。

| 窗 | 级别 | 侧 | 趋势一类 | 否则域 | 合计 |
|---|---:|---|---:|---:|---:|
| p3fold | L0 | Long | 4 | 7 | 11 |
| p3fold | L0 | Short | 3 | 8 | 11 |
| p3fold | L1 | Short | 1 | 22 | 23 |
| p3fold | L2 | Short | 12 | 5 | 17 |
| p3fold | L3 | Short | 0 | 25 | 25 |
| wf7 | L0 | Long | 2 | 5 | 7 |
| wf7 | L0 | Short | 2 | 7 | 9 |
| wf7 | L1 | Short | 0 | 17 | 17 |
| wf8 | L0 | Long | 1 | 4 | 5 |
| wf8 | L0 | Short | 1 | 5 | 6 |
| wf8 | L1 | Short | 9 | 18 | 27 |
| wf8 | L3 | Short | 10 | 11 | 21 |

值得注意：

- p3fold L3 Short 为 `0/25`：25 条全部落否则域；
- wf7 L1 Short 为 `0/17`：17 条全部落否则域；
- wf8 的 T3 命中主要来自 L1/L3 Short（9+10），但这仍是临时“全窗后扫”口径。

### 4.3 与 Reset 的逐条对账

| 窗 | census 一类点 | lifecycle Reset | 精确匹配 | 未匹配 | 否则域 Reset | 趋势 Reset |
|---|---:|---:|---:|---:|---:|---:|
| p3fold | 87 | 87 | 87 | 0 | 67 | 20 |
| wf7 | 33 | 33 | 33 | 0 | 29 | 4 |
| wf8 | 59 | 59 | 59 | 0 | 38 | 21 |

此处直接证明的是当前执行配置下“一类确认事件 → Reset 广播”的事件数，不是“Reset 杀死中枢”；
现行 #489 语义下 Reset 只广播，`died_*` 是到场时 arena 活中枢快照。

## 5. 一类全平与 trades 影响

### 5.1 归因方法

1. 在 `trades.jsonl` 取
   `exit_type=="CloseRoot" && trigger_bsp_class_at_exit==1`；
2. 用 `exit_bar` 连接同 bar lifecycle Reset；
3. 本次所有相关 bar 均恰有一条 Reset；
4. 再用 Reset 的 `(level,trigger_side,trigger_src)` 连接 census 行。

因此本次 7 笔一类 CloseRoot 的趋势/否则归因无多重匹配：

| 窗 | trade_id | exit_bar | 档 |
|---|---:|---:|---|
| p3fold | 245 | 66,443 | otherwise |
| p3fold | 471 | 109,721 | otherwise |
| wf7 | 183 | 42,041 | otherwise |
| wf8 | 574 | 146,768 | trend_first |
| wf8 | 1056 | 253,568 | otherwise |
| wf8 | 1075 | 258,075 | trend_first |
| wf8 | 1080 | 258,851 | trend_first |

### 5.2 成交计数与占比

| 窗 | trades 全部行 | CloseRoot 全部 | 一类 CloseRoot | 一类 CloseRoot/全部 | 否则域一类 CloseRoot | 否则域对应占比 |
|---|---:|---:|---:|---:|---:|---:|
| p3fold | 1,072 | 183 | 2 | 0.1866% | 2 | 0.1866% |
| wf7 | 1,220 | 295 | 1 | 0.0820% | 1 | 0.0820% |
| wf8 | 1,097 | 208 | 4 | 0.3646% | 1 | 0.0912% |

这里的占比分母是 typed `trades.jsonl` 行，不是 M8 `n_orders`。二者对象不同：一笔持仓生命周期可有
多个 fill/order，不能混用。

## 6. “否则域不发 Reset、全平不消费”的静态预估

### 6.1 可静态给出的局部差异

| 窗 | Reset 基线 → 局部过滤后 | ΔReset | trades 基线 → 本地删行代理 | Δtrade 行 |
|---|---:|---:|---:|---:|
| p3fold | 87 → 20 | -67 | 1,072 → 1,070 | -2 |
| wf7 | 33 → 4 | -29 | 1,220 → 1,219 | -1 |
| wf8 | 59 → 21 | -38 | 1,097 → 1,096 | -1 |

解释：

- Reset 列是在“只按本次分档过滤广播”的局部事件账面值；
- trades 列只是“不消费当下已归因的否则域 CloseRoot 行”的机械代理；
- 真分支下，这 4 个持仓不会在原 bar 全平，后续持仓、风控、反向开仓、费用和结算都会改道，因此
  实际窗口末 trades 可能多于或少于该代理值。

### 6.2 四层读数

| 窗 | execR 基线 → 反事实 | R 基线 → 反事实 | MaxDD 基线 → 反事实 | LCB 基线 → 反事实 |
|---|---|---|---|---|
| p3fold | -1,614,138 → **未能判定** | -1,583,081 → **未能判定** | 0.0976 → **未能判定** | -2,834,763 → **未能判定** |
| wf7 | -952,242 → **未能判定** | -915,473 → **未能判定** | 0.0732 → **未能判定** | -2,347,654 → **未能判定** |
| wf8 | -312,337 → **未能判定** | -276,641 → **未能判定** | 0.0776 → **未能判定** | -1,961,062 → **未能判定** |

不能用“删掉 4 行成交的 `pnl_raw_unlevered`”重算四层，理由是：

1. `pnl_raw_unlevered` 是费前方向价差，不含完整 fee/cost；
2. 不全平意味着仓位继续 MtM，不是该 PnL 简单消失；
3. execR/R 由全路径仓位、成交、费用和浮盈共同决定；
4. MaxDD 需要逐 bar equity；
5. LCB 需要完整 daily-return/bootstrap 输入。

因此静态估算能诚实给出的上限是事件/成交局部差异面，不能给数值化四层终值或方向。

## 7. 数据支持粒度与缺口

### 7.1 现有转储能直接支持什么

| 来源 | 已支持 |
|---|---|
| `firstclass.jsonl`（本次 D0） | 每个一类点的 level/side/source、A span、last-B 六边、λ_C、seg.end、扫描口径 T3 pair/两档 |
| `center_lifecycle.jsonl` | Reset 的 bar/level/side/source；born/broken/stale/superseded/chain_sync |
| `trades.jsonl` | typed trade entry/exit、exit_type、units、入口证书、出场触发 class |
| `tower_events.jsonl` | 中枢 new/extend/upgrade |
| M8 report/stdout | 每窗最终 execR/R/MaxDD/LCB 与成本分解 |
| #565 `full-third.jsonl` | wf8 全量 regular/routed T3；58 个 leak 目标的专用状态迁移 |

### 7.2 仍缺字段

1. **#586 五桶**：本次无 T3 只记 `otherwise`，未外化
   `missing_leave/missing_retest/same_direction/leave_not_outside/retest_reentered`；也未输出固定旧框首对。
2. **确认时点**：census 行有点的 `source_index`，但确认 bar 需从 lifecycle 外连；应直接写
   `observed_bar`。
3. **稳定连接键**：Reset 与 trade 没有共享 `firstclass_event_id/reset_event_id/close_decision_id`；
   本次依赖 `(level,side,source)` 和唯一同 bar Reset，当前可证但不是长期 schema 契约。
4. **完整 pair 证据**：命中只写 leave/retest end；缺方向、start、终点价和五桶失败证据。
5. **封印元数据**：census meta 未内嵌 git SHA、data SHA、完整 env/config fingerprint；本报告外部补记。
6. **四层反事实**：缺逐 bar position/equity/daily returns、完整订单/费用/TW 事件及 bootstrap 输入。
7. **对象身份**：lifecycle `died_*` 是 arena 快照，不等于一类判定 last-B；禁止以它替代。
8. **#586 口径状态**：issue 尚未关，最终固定首对/扫描口径未正式生效。

### 7.3 建议给 spec 的最小观测行

```text
schema, git_sha, data_sha256, window, env_config_fingerprint
firstclass_event_id, observed_bar, level, side, source_index, bsp_bits
last_B=(si,ei,zd,zg,dd,gg), lambda_c, seg_end, trend_dir, a_span
pair_policy=(fixed_first|scan_first_success)
leave=(si,ei,dir,end_px), retest=(si,ei,dir,end_px)
t3_grade=(pass|missing_leave|missing_retest|same_direction|
          leave_not_outside|retest_reentered)
reset_event_id, close_decision_id, order_ids[], trade_ids[]
```

四层反事实另需可重放的每 bar 仓位/权益/费用/TW 状态，不能把它们塞成一个“估算 PnL”字段。

## 8. 复现物

| 产物 | 路径 / SHA |
|---|---|
| 窗口脚本 | `/tmp/research-585/run_window.sh` |
| 汇总脚本 | `/tmp/research-585/analyze.mjs` |
| 背景证据底稿 | `/tmp/research-585/background-evidence.md` |
| 探针 worktree | `/tmp/research-585/wt-run` |
| p3fold census | `/tmp/research-585/results/p3fold/firstclass.jsonl` / `1447f204bacc3e3f016e8551415de128f5c1c174e0f9b31c9dc6e9b2e3f5d6b0` |
| wf7 census | `/tmp/research-585/results/wf7/firstclass.jsonl` / `b3ca44064274306fc3a65e83afa7091f9fcb12ca10758dc81a666495c1d880ef` |
| wf8 census | `/tmp/research-585/results/wf8/firstclass.jsonl` / `0e8bd3c1ffcce25f8be55fe63e00b73398f9fb3d98ca29cb5661dada9308f8b3` |
| wf8 probe-off 对拍 | `/tmp/research-585/results/wf8-off/` |

典型命令：

```bash
cd /tmp/research-585/wt-run/rust
/tmp/research-585/run_window.sh p3fold
/tmp/research-585/run_window.sh wf7
/tmp/research-585/run_window.sh wf8
node /tmp/research-585/analyze.mjs
```

## 9. 090 终结论

- **已判定**：三窗枚举、临时扫描口径两档计数、分侧分级、Reset 逐条归因、一类 CloseRoot
  成交及其占比、局部过滤事件数。
- **未能判定**：正式 #586 固定首对口径下的终值；分档实装后的真实 trades、仓位路径与
  `execR/R/MaxDD/LCB`。
- **未写主仓**：任务同时要求“主仓一律不写”和“报告落主仓路径”；本报告服从更强硬边界，仅留
  `/tmp/research-585/`。

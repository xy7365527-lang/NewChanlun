# wf7 链归因当前 HEAD 口径重跑 + 卡点定位图（#251 交付物）

- 日期：2026-07-25
- 执行：子代理（战术执行，非主控）
- HEAD：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`），当时 HEAD =
  `140a660944`（`docs(chanlun): #259 盘整背驰教义地位 + 消费路径双查`）
- 票：map #250 子票 #251

## 一、实际跑批命令与耗时

### 1.1 定位入口（照实记录，未凭猜）

- 测试函数：`theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos`（`rust/src/theta_v0/backtest/wverify_run.rs:1197`），`#[ignore]`。
- 窗口清单硬编码在函数体内（`wverify_run.rs:1211-1218`）：`p3fold` + `PREREG_WINDOWS`（BTC）里
  `wf_anchored.filter(test_start >= OOS_START).take(2)`——`OOS_START="2023-01-01"`
  （`prereg_windows.rs:350`），BTC `wf_anchored` 表（`prereg_windows.rs:77-89`）里首两个满足
  条件的窗口正是 `wf7`（test 2023-02-17..2023-08-16）与 `wf8`（test 2023-08-17..2024-02-16）——
  与票面「wf7/wf8」命名对应关系已核实非猜测。
- dump 接线：`T5A_CHAIN_DUMP_DIR=<dir>` env，m8 循环内逐窗调
  `super::admission::t5a_chain_dump::open_for_window(&tag)` / `close()`（`wverify_run.rs:1273/1276`）。
- 单窗过滤：`M8_WIN_FILTER=<tag>` env 可选（本次未用，跑全部 3 窗一次性产出）。

### 1.2 关键发现：dump 需要第二个 env，光设 `T5A_CHAIN_DUMP_DIR` 不够

**第一次尝试**（只设 `T5A_CHAIN_DUMP_DIR`）：

```
cd /tmp/kimi-nest-mainline/rust
T5A_CHAIN_DUMP_DIR=/tmp/chain-dump-20260725 \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

结果：测试通过，stderr 打印 `[t5a_dump] window=wf7 → ...`（open_for_window 确实被调用），但
落盘的 `t5a_chain_dump_wf7.jsonl` / `_wf8.jsonl` **均为 0 字节**。

**根因**（读 `fill.rs:805-843` 核实）：`t5a_chain_dump::record(...)` 调用点被包在
`match &nest_gate_hist { Some(hist) => {...gate.admit(...); record(...)}, None => step_gamma_trade }`
内部——只有 `nest_gate_hist` 非 None（即真链门已开）才会走到 `record`。而 `nest_gate_hist` 的
开关是 `nest_cert_gate_enabled()`（`admission.rs:52-59`）：**`THETA_NEST_CERT_GATE=1`**。
m8 测试本身不设这个 env（它跑的是默认口径），**dump 装置本身零开销中立，但默认口径下门关闭 ⟹
无记录**。7-24 旧 dump 显然是在设了 `THETA_NEST_CERT_GATE=1` 的前提下产出的（否则不会有 1724 行）。

**耗时**：第一次尝试（空转）44.8s（`time` 实测：`user 95.67s system 2.57s cpu 219% total 44.843s`）。

### 1.3 实际生效命令

```
cd /tmp/kimi-nest-mainline/rust
THETA_NEST_CERT_GATE=1 T5A_CHAIN_DUMP_DIR=/tmp/chain-dump-20260725 \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

- 结果：`test theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos ... ok`（1 passed）。
- **耗时**（`time` 实测）：`user 83.67s system 0.30s cpu 99% total 1:24.07`（84 秒，含 release 编译
  ~22s + 三窗回放 ~62s）。两次尝试合计 <3 分钟，远低于 15 分钟阈值，未中途放弃。
- 产物：
  - `/tmp/chain-dump-20260725/t5a_chain_dump_p3fold.jsonl`（1633 行，非本票范围，未分析）
  - `/tmp/chain-dump-20260725/t5a_chain_dump_wf7.jsonl`（**1724 行**，与旧 7-24 读数行数一致）
  - `/tmp/chain-dump-20260725/t5a_chain_dump_wf8.jsonl`（**1518 行**，与旧 7-24 读数行数一致）

## 二、三张分布表（当前 HEAD 口径，wf7 + wf8）

解析脚本：`/tmp/chain-dump-20260725/analyze.py`（纯读 JSONL 计数，无判定逻辑改动）。

### 表 A · verdict 三态

| 窗 | verdict | 当前(HEAD) | 占比 | 旧读数(07-24) | 占比 | Δ |
|---|---|---:|---:|---:|---:|---:|
| wf7 | pass | 32 | 1.9% | 33 | 1.9% | **−1** |
| wf7 | reject | 100 | 5.8% | 100 | 5.8% | 0 |
| wf7 | no_chain | 1592 | 92.3% | 1591 | 92.3% | **+1** |
| wf8 | pass | 33 | 2.2% | 34 | 2.2% | **−1** |
| wf8 | reject | 34 | 2.2% | 34 | 2.2% | 0 |
| wf8 | no_chain | 1451 | 95.6% | 1450 | 95.5% | **+1** |

（旧读数用同一脚本重新解析 `chanlun/review-results/endorsement-instrument-baseline-{wf7,wf8}-chain-dump-20260724.jsonl` 得到——**不是直接抄 #251 issue 文字里的旧表**，因为那张旧表本身是手写摘要、对 0 值列做了省略，逐格核对必须回到原始 jsonl 才可信，见第三节。）

### 表 B · verdict × first_gap(kind, level)

**wf7**（n=1724）：

| verdict | kind | level | 当前 | 旧(07-24原始重解析) | Δ |
|---|---|---:|---:|---:|---:|
| no_chain | missing | 0 | 884 | 883 | **+1** |
| no_chain | missing | 1 | 261 | 261 | 0 |
| no_chain | missing | 2 | 206 | 206 | 0 |
| no_chain | missing | 3 | 73 | 73 | 0 |
| no_chain | missing | 4 | 151 | 151 | 0 |
| no_chain | None | — | 17 | 17 | 0 |
| pass | None | — | 32 | 33 | **−1** |
| reject | missing | 1 | 26 | 26 | 0 |
| reject | missing | 2 | 17 | 17 | 0 |
| reject | missing | 3 | 17 | 17 | 0 |
| reject | missing | 4 | 15 | 15 | 0 |
| reject | broken | 0 | 17 | 17 | 0 |
| reject | broken | 1 | 8 | 8 | 0 |

first_gap 落 level 0（missing+broken 合计）：**901**（52.3%），旧 **900**（52.2%）。

**wf8**（n=1518）：

| verdict | kind | level | 当前 | 旧(重解析) | Δ |
|---|---|---:|---:|---:|---:|
| no_chain | missing | 0 | 891 | 890 | **+1** |
| no_chain | missing | 1 | 324 | 324 | 0 |
| no_chain | missing | 2 | 73 | 73 | 0 |
| no_chain | missing | 3 | 133 | 133 | 0 |
| no_chain | None | — | 30 | 30 | 0 |
| pass | None | — | 33 | 34 | **−1** |
| reject | broken | 0 | 15 | 15 | 0 |
| reject | broken | 1 | 3 | 3 | 0 |
| reject | missing | 1 | 8 | 8 | 0 |
| reject | missing | 2 | 6 | 6 | 0 |
| reject | missing | 3 | 2 | 2 | 0 |

first_gap 落 level 0 合计：**906**（59.7%），旧 **905**（59.6%）。

### 表 C · 逐级 status

**wf7**：

| 级别 | missing_cert | missing_existence | missing_causal | closed | broken |
|---|---:|---:|---:|---:|---:|
| L0 | 1217 | 377 | **28**（旧27） | **85**（旧86） | 0 |
| L1 | 389 | 340 | 20 | 44 | 0 |
| L2 | 265 | 194 | 20 | 8 | 0 |
| L3 | 100 | 73 | 83 | 0 | 0 |
| L4 | 166 | 0 | 0 | 0 | 0 |

**wf8**：

| 级别 | missing_cert | missing_existence | missing_causal | closed | broken |
|---|---:|---:|---:|---:|---:|
| L0 | 1142 | 274 | **27**（旧26） | **45**（旧46） | 0 |
| L1 | 366 | 173 | 6 | 19 | 0 |
| L2 | 79 | 101 | 34 | 3 | 0 |
| L3 | 135 | 0 | 0 | 0 | 0 |

（旧表这几列在 #251 issue 正文/`chain-attribution-caliber-check-20260725.md` 里用「—」省略了
L0-L2 的 `missing_causal`/`closed` 具体值——那是手写摘要的省略，不代表旧原始 dump 里这些值为 0。
本节旧列数字全部来自对 `endorsement-instrument-baseline-{wf7,wf8}-chain-dump-20260724.jsonl`
的重新解析，与当前 HEAD 用同一脚本、同一字段口径，逐格可信对照。）

## 三、逐格对照差异归因

**结论：wf7、wf8 两窗各恰好 1 个候选（共 2 个）verdict 从 `pass` 翻转为 `no_chain`，其余全部
逐格 bit-exact 相同。** 无格差异 >10%（实际最大差异 = 1/1724 ≈ 0.06%），不构成"显著差异"，
但仍做单候选级溯源（用 (bar, source_index, level, dir) 做键，直接 diff 新旧原始 jsonl 定位）：

| 窗 | key(bar, source_index, level, dir) | 旧 | 新 |
|---|---|---|---|
| wf7 | (34998, 34934, 0, Long) | `status=closed, certs=1, clean=1, verdict=pass` | `status=missing_causal, certs=1, clean=0, verdict=no_chain` |
| wf8 | (234711, 234657, 0, Long) | `status=closed, certs=1, clean=1, verdict=pass` | `status=missing_causal, certs=1, clean=0, verdict=no_chain` |

两个候选证书数（`certs`）不变（仍是 1），但 `clean`（因果洁净计数 `n_causal_clean`）从 1 变为 0——
即该候选唯一的证书现在被因果守卫剔除（`cert.judge_at().iter().any(|&t| t > anchor_index)` 现在
判真，见 `admission.rs:914-916`）。**归因**：#251 前置核实文档已指出 dump 早于 `admission.rs`
knot 合版（commit `d2312352f9`，新增 `chain_driven_level_projection`）16 小时；该改动会派生
`level_projection` 配置，进而可能改变逐级 `existence`/证书查询的锚定索引重建时点
（`chain_key_hint`/`sync_index` 惰性重建，见 `fill.rs:816-829` 注释），使这两个边界候选的
`judge_at()` 时间戳相对 `anchor_index` 的比较结果翻转。**这是可归因、单点、非扩散性的翻转**——
不影响 92%+ 的候选分布，也不改变本票核心问句（missing_cert 主导 vs missing_existence/causal
次要）的构成比读法。

## 四、构成比裁读（核心问句）

延续旧读数指向、当前口径确认：**卡点主要是"证书产得太少"（missing_cert），不是"桥认不出"
（missing_existence + missing_causal）**。

- wf7 L0：missing_cert 1217 ≫ missing_existence 377 + missing_causal 28 = 405（前者是后者 3 倍）。
- wf8 L0：missing_cert 1142 ≫ missing_existence 274 + missing_causal 27 = 301（前者是后者 3.8 倍）。
- 更高级别（L1-L4）同一模式持续：missing_cert 恒 > missing_existence，且 missing_causal 除 L3 外
  普遍很小（L3 的 missing_causal=83/wf7、无/wf8 是唯一反例，量级仍远小于 missing_cert 主档）。

## 五、反事实量化（本票新增要求）——未能判定，缺口如实列出

### 5.1 需要的量

1. wf7 窗内，盘整块产出的 `cand_delta=false / pan_div_diag=true` 诊断事件数，按级别拆。
2. 反事实候选池规模（若计入这些事件）。
3. missing_cert 缺口所在级别与"该级有盘整诊断事件存在"的重合度。

### 5.2 逐项判定

**全部三项未能判定。** 已核实的具体缺口：

- **字段缺口**：本票用的 `t5a_chain_dump`（`NestGateObs`，`admission.rs:278-299`）记录的是
  **链判定结果**（三态/谱系/缺断），消费的是已装配的 `NestCertificate`（typed 证书），**不携带
  `CandDeltaEvent.pan_div_diag` / `cand_delta` 原始位**——这两个字段活在
  `classifier::recursive_tower::CandDeltaEvent`（`recursive_tower.rs:1200` 起），链门读的是
  下游装配产物，谓词层原始诊断位在这一跳已经丢失，dump 无法二次读出。
- **唯一携带 `pan_div_diag` 计数的现存装置**是 `rust/src/bin/strict_nest_check.rs`
  的 `P1Checker::final_snapshot`（每级 `(bsp bits, 事件总数, cand_delta=true, pan_div_diag=true)`
  四元组，见 `strict_nest_check.rs:546/1712-1719`）。但它有两个结构性不匹配，均不可绕过：
  1. **需要本环境不存在的数据缓存**：`data_path = <STRICT_NEST_DATA_ROOT>/analysis/data_cache/btc_1m_full.json`，
     默认 `/tmp/codex-work-p7`——已确认该目录在本 worktree/本机 `/tmp` 下**不存在**
     （`ls` 返回 `No such file or directory`），且约束禁止改代码补路径/伪造数据。
  2. **即使数据可用，口径也对不上**：`final_snapshot` 是**全历史因果重放到末 bar 的累积计数**
     （`STRICT_NEST_MAX_BARS` 只能从 bar 0 截断，不支持任意起点），不是"wf7 测试窗内发生"的
     事件数——用它，只能算出"全历史累积到某 bar 为止"的标量，无法做窗口内 delta，也没有可以
     和 `t5a_chain_dump` 里逐候选 `(bar, source_index, level)` 做 join 的公共键（`strict_nest_check`
     不输出逐事件级的 bar/source_index，只输出末 bar 汇总标量）。
- 因此：**候选池反事实规模（第 2 项）与 missing_cert 缺口/盘整诊断事件的级别重合度（第 3 项）
  都缺少可 join 的数据源**，不是"跑不出来"，而是现有两套装置（chain dump vs strict_nest_check）
  分属不同粒度（逐候选 vs 全历史末态标量）且无公共键，字段设计上不支持这个问题。

**未作任何数字推测填空。** 若要回答，唯一路径是新增一个逐候选/逐 bar 记录
`pan_div_diag`+级别+`missing_cert` 归属的 dump 字段——这属于"改代码补字段"，超出本票授权
（票面明确：不改代码、不改判定）。

## 六、纪律确认

- 未修改 `rust/src` 下任何生产代码，未修改判定口径。
- 未 `git commit`（仅写文件）。
- 跑批数字均来自本次实测的 `/tmp/chain-dump-20260725/*.jsonl` 与 stderr 输出，旧读数均来自
  worktree 内既有的 `endorsement-instrument-baseline-{wf7,wf8}-chain-dump-20260724.jsonl` 原始
  重新解析（非抄录 issue 正文的手写摘要）。
- 第五节的"未能判定"是完整的负面结果，未用旧数据或推测数字冒充。

# p100 证书 × BSP 全量对账收口（task #100，#105 口径复跑）

日期：2026-07-17　数据：btc_1m_full.json（4,613,599 bar）
工具：p100_cert_bsp_recon（rust/src/bin/p100_cert_bsp_recon.rs，只读探针，未做任何修改）
输入 dump：/tmp/p92_ckpt_dump.txt（p92 typed 全量重放产物，#105 修复后口径）
复跑命令：

```text
cd rust && cargo run --release --features backtest_bin --bin p100_cert_bsp_recon -- /tmp/p92_ckpt_dump.txt
```

## 0. 探针与 #105 dump 格式核验

- dump 含 66 行 `CERT ` 明细：`caliber=A` 41 行、`caliber=B` 25 行，与 #105 口径（A=41 / B=25）逐字一致。
- `CERT` 行字段（caliber/side/bucket/kinds/as_of/judge_at）与探针解析器
  （rust/src/bin/p100_cert_bsp_recon.rs:49-94）完全匹配；`judge_at` 逗号钟表取 max 的逻辑
  （:72-78）覆盖多身份行（dump 中 18 行多钟表）。**解析无出入，探针零修改复跑。**
- P100_INPUT 行确认：`bars=4613599 certs_total=66 A=41 B=25`。

## 1. 四项验证点结果（#105 新口径）

### 1.1 1440 bar 内同侧 BSP 覆盖率

| 口径 | n | 同 bar (dt=0) | \|dt\|≤5 | \|dt\|≤30 | \|dt\|≤240 | \|dt\|≤1440 | median\|dt\| |
|---|---|---|---|---|---|---|---|
| A | 41 | 15 | 15 | 15 | 29 | **41（100%）** | 108 |
| B | 25 | 8 | 8 | 8 | 17 | **25（100%）** | 108 |

- 两口径 1440 bar 内同侧覆盖率 **均 100%**，与旧口径结论一致（旧报告
  chanlun/review-results/nest-cert-bsp-recon-20260717.md:11-12 同为全覆盖）。
- **同 bar 比例显著下降**：A 15/41=36.6%、B 8/25=32.0%（旧口径 A 28/46=60.9%、B 31/45=68.9%，
  median 均为 0）。#105 重排证书时钟/成员后，证书判定时钟与 BSP 确认 bar 的精确重合减少，
  但全部落在 1440 bar 邻域内——证书落点仍无系统性错位，只是「同 bar 重合」不再是多数形态。
- median|dt|=108 bar（≈1.8 小时），远小于 1440 窗口。

### 1.2 反向覆盖率（BSP ±W 内存在同向证书的比例）

- w=240：60/28,417 = 0.211%（旧 49/28,417 = 0.17%）。
- w=1440：345/28,417 = 1.214%（旧 275/28,417 = 0.97%）。
- 量级结论不变：证书密度比 BSP 低约两个数量级（66 张 vs 28,417 个），
  证书仍是「极小的高置信子集」。

### 1.3 最近命中 lvl 分布（near_lvl，归因字段，非硬映射）

| 口径 | L0 | L1 | L2 | L3 | L4 |
|---|---|---|---|---|---|
| A (n=41) | 35 | 3 | 2 | 1 | 0 |
| B (n=25) | 23 | 2 | 0 | 0 | 0 |

- 合计 58/66 = 87.9% 的证书最近同侧 BSP 命中 L0；exec≥2 的多顶证书同样主要绑到 L0
  （如 A exec=3 top=3 judge_max=1833453 near_lvl=3 是唯一 L3 例）。
- 支持 #101 草案条款 5：nest exec/top 与 classifier lvl 语义不同构，不做硬映射
  （chanlun/escalate/cert-bsp-binding-ruling-DRAFT-20260717.md:23）。

### 1.4 BSP 侧总量与分层（与旧运行逐字一致）

- events_total=28,417（buys=14,762 / sells=13,655），max_lvl=4。
- L0 buys=10,298 sells=9,478；L1 2,318/2,158；L2 1,199/1,208；L3 559/651；L4 388/160。
- 与 #101 草案物化校验行（cert-bsp-binding-ruling-DRAFT-20260717.md:43）逐字一致——
  BSP 侧是生产因果塔 + runner 同语义 seen-set diff（p100 探针 :148-168），不受 #105 证书侧变化影响。

### 1.5 方向与 bucket 分布（#105 新口径）

- A：Long=17 / Short=24；bucket pan=29 / mixed=9 / trend=3。
- B：Long=12 / Short=13；bucket pan=22 / trend=3（mixed=0）。
- **B 口径 Long 侧在本对账口径下为 12/25，并非 0 张**；CKPT 层 B Long=138 / Short=135 同样均衡。
  「B Long 0 张」若存在，必在 CERT/CKPT 之外的下游层（如信号转换或交易层）——归因属 #102 范围，
  本报告只固定 p100 可见事实。

## 2. 全部 P100_* 报表行原文

```text
P100_INPUT bars=4613599 certs_total=66 A=41 B=25
P100_BSP events_total=28417 buys=14762 sells=13655 max_lvl=4
P100_BSP_LVL lvl=0 buys=10298 sells=9478
P100_BSP_LVL lvl=1 buys=2318 sells=2158
P100_BSP_LVL lvl=2 buys=1199 sells=1208
P100_BSP_LVL lvl=3 buys=559 sells=651
P100_BSP_LVL lvl=4 buys=388 sells=160
P100_CERT caliber=A side=Long as_of=4613598 judge_max=612465 bucket=pan kinds=Consolidation dt_same=Some(42) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=665571 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=707523 bucket=pan kinds=Consolidation dt_same=Some(-147) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=922759 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=1590498 bucket=pan kinds=Consolidation dt_same=Some(-352) dt_any=Some(-352) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=1661820 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=1837547 bucket=pan kinds=Consolidation dt_same=Some(436) dt_any=Some(-39) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=2109551 bucket=pan kinds=Consolidation dt_same=Some(-230) dt_any=Some(-182) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=2125061 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(1)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=2563943 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(1)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=3747181 bucket=pan kinds=Consolidation dt_same=Some(-215) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=2174906 bucket=trend kinds=Trend dt_same=Some(1191) dt_any=Some(-40) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=81324 bucket=pan kinds=Consolidation dt_same=Some(-79) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=153753 bucket=pan kinds=Consolidation dt_same=Some(526) dt_any=Some(-110) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=1744395 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=2261563 bucket=pan kinds=Consolidation dt_same=Some(-273) dt_any=Some(149) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=3048761 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=3108044 bucket=pan kinds=Consolidation dt_same=Some(-519) dt_any=Some(43) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=3626245 bucket=pan kinds=Consolidation dt_same=Some(-116) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=4131236 bucket=pan kinds=Consolidation dt_same=Some(67) dt_any=Some(67) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=4364844 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=736245 bucket=pan kinds=Consolidation,Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(2)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=1595881 bucket=mixed kinds=Trend,Consolidation dt_same=Some(586) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=1845523 bucket=pan kinds=Consolidation,Consolidation dt_same=Some(499) dt_any=Some(-329) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=3754521 bucket=pan kinds=Consolidation,Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=85045 bucket=pan kinds=Consolidation,Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=162411 bucket=mixed kinds=Trend,Consolidation dt_same=Some(191) dt_any=Some(-152) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=1748106 bucket=mixed kinds=Trend,Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(1)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=3066864 bucket=pan kinds=Consolidation,Consolidation dt_same=Some(-142) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=3115054 bucket=pan kinds=Consolidation,Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(2)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=3633992 bucket=mixed kinds=Trend,Consolidation dt_same=Some(412) dt_any=Some(234) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=4149549 bucket=pan kinds=Consolidation,Consolidation dt_same=Some(322) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=795426 bucket=mixed kinds=Trend,Consolidation,Consolidation dt_same=Some(208) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=97889 bucket=pan kinds=Consolidation,Consolidation,Consolidation dt_same=Some(-179) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=167458 bucket=mixed kinds=Trend,Trend,Consolidation dt_same=Some(-54) dt_any=Some(-54) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=1833453 bucket=mixed kinds=Trend,Consolidation,Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(3)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=3640074 bucket=mixed kinds=Consolidation,Trend,Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=1993659 bucket=mixed kinds=Trend,Trend,Consolidation,Consolidation dt_same=Some(380) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=137910 bucket=trend kinds=Trend dt_same=Some(108) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=A side=Short as_of=4613598 judge_max=1348880 bucket=pan kinds=Consolidation dt_same=Some(77) dt_any=Some(60) near_lvl=Some(0)
P100_CERT caliber=A side=Long as_of=4613598 judge_max=2556965 bucket=trend kinds=Trend dt_same=Some(344) dt_any=Some(0) near_lvl=Some(0)
P100_SUMMARY caliber=A n=41 w0=15 w5=15 w30=15 w240=29 w1440=41 median_abs_dt=108
P100_CERT caliber=B side=Long as_of=4613598 judge_max=612465 bucket=pan kinds=Consolidation dt_same=Some(42) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=665571 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=707523 bucket=pan kinds=Consolidation dt_same=Some(-147) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=922759 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=1590498 bucket=pan kinds=Consolidation dt_same=Some(-352) dt_any=Some(-352) near_lvl=Some(0)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=1661820 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=1837547 bucket=pan kinds=Consolidation dt_same=Some(436) dt_any=Some(-39) near_lvl=Some(0)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=2109551 bucket=pan kinds=Consolidation dt_same=Some(-230) dt_any=Some(-182) near_lvl=Some(0)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=2125061 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(1)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=2563943 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(1)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=3747181 bucket=pan kinds=Consolidation dt_same=Some(-215) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=2174906 bucket=trend kinds=Trend dt_same=Some(1191) dt_any=Some(-40) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=81324 bucket=pan kinds=Consolidation dt_same=Some(-79) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=153753 bucket=pan kinds=Consolidation dt_same=Some(526) dt_any=Some(-110) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=1744395 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=2261563 bucket=pan kinds=Consolidation dt_same=Some(-273) dt_any=Some(149) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=3048761 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=3108044 bucket=pan kinds=Consolidation dt_same=Some(-519) dt_any=Some(43) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=3626245 bucket=pan kinds=Consolidation dt_same=Some(-116) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=4131236 bucket=pan kinds=Consolidation dt_same=Some(67) dt_any=Some(67) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=4364844 bucket=pan kinds=Consolidation dt_same=Some(0) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=153753 bucket=pan kinds=Consolidation,Consolidation dt_same=Some(526) dt_any=Some(-110) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=137910 bucket=trend kinds=Trend dt_same=Some(108) dt_any=Some(0) near_lvl=Some(0)
P100_CERT caliber=B side=Short as_of=4613598 judge_max=1348880 bucket=pan kinds=Consolidation dt_same=Some(77) dt_any=Some(60) near_lvl=Some(0)
P100_CERT caliber=B side=Long as_of=4613598 judge_max=2556965 bucket=trend kinds=Trend dt_same=Some(344) dt_any=Some(0) near_lvl=Some(0)
P100_SUMMARY caliber=B n=25 w0=8 w5=8 w30=8 w240=17 w1440=25 median_abs_dt=108
P100_REVERSE w=240 bsp_covered=60/28417 rate=0.002111
P100_REVERSE w=1440 bsp_covered=345/28417 rate=0.012141
P100_DONE
```

## 3. 结论

1. **对账通过（#105 口径）**：A=41 / B=25 全部证书在 1440 bar 窗口内 100% 有同侧 BSP 对应，
   无 >1440 失联（最大 |dt|=1191，A/B 同一张 trend Short 证书 judge_max=2174906）。
2. **形态变化**：同 bar 重合率由旧口径 ~65% 降至 ~35%，median|dt| 由 0 升至 108；
   证书与 BSP 的关系应从「同 bar 重合」改述为「近邻绑定」，但空间一致性（全覆盖、无错位）不变。
3. **强度分层阈值 240 有实证空隙支撑**：strong 组（|dt|≤240）最大 |dt|=230，
   weak 组（240<|dt|≤1440）最小 |dt|=273——240 阈值落在 230↔273 的自然真空带内，
   新口径下分层边界无需调整。
4. **反向覆盖率量级不变**（0.21% / 1.21%），证书密度比 BSP 低约两个数量级的判断继续成立。
5. near_lvl 87.9% 落 L0，lvl 硬映射不可行的旧判断继续成立。

## 4. 对 #101 裁定的输入建议

1. **证书不能作独立入场源**（维持草案条款 1 的 sidecar 打标）：反向覆盖 w1440 仅 1.214%
   （345/28,417），独立入场将放弃约 99% 的 BSP 结构覆盖；证书角色是已确认 BSP 上的
   高置信标注，不是信号源。旧报告同结论（nest-cert-bsp-recon-20260717.md:26,32）。
2. **打标强度分层依据成立**：|dt|≤240 → strong 在新口径下 A 29/41（70.7%）、B 17/25（68.0%）；
   240<|dt|≤1440 → weak 共 20 张（A 12 / B 8），dts 全部落在 273–1191；unbound=0。
   草案条款 3（cert-bsp-binding-ruling-DRAFT-20260717.md:19-20）的分档边界有 230↔273
   真空带实证支撑，**不需要按级别自适应改造**——weak 组内 A/B 的 lvl 分布与 strong 组无显著差异
   （weak 20 张 near_lvl 全为 L0/L1），按 lvl 分档没有区分度收益。
3. **打标语义应按「近邻绑定」表述**：新口径下同 bar 重合仅 ~35%，median|dt|=108；
   建议 #101 文本把「过半精确同 bar」类表述（草案 :8）更新为「全部落于 1440 bar 邻域、
   约七成落于 240 bar 内」，避免裁定前提引用旧口径数字。
4. **weak 组唯一离群需留痕**：trend Short 证书（judge_max=2174906，A/B 各一张）dt_same=1191
   且 dt_any=-40（40 bar 外即有反向 BSP）——方向隔离的绑定规则把它推到 1191 bar 外的同侧 BSP；
   建议 #101 对 weak 档附加「dt_any 显著小于 dt_same 时降权/标注」的归因字段（不必改绑定规则，
   探针已输出该字段，p100_cert_bsp_recon.rs:213-214）。
5. **B 口径方向分布已均衡**（Long 12 / Short 13）：草案「开放条款」中「B Long 仅 11 张、
   暂勿用于方向不对称策略权重」的谨慎前提（:35-36）在 #105 口径下依然偏保守但理由已变——
   本对账层看不到 0 张现象，方向偏置风险应等 #102 在下游层归因落地后再评估。

—— #100 收口。探针零修改；生产源码、Cargo.toml 未动；主仓未写入。

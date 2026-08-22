# S2 Lean 完整语义镜像与 Rust↔Lean 提取对拍规范（第一阶段）

> 票：GitHub issue #1080（S2）。
> 日期：2026-08-21。
> 工作线：`codex/s2-mirror`，基线 `main@355b839c295ae17009e170e1d02e3219e065eae8`。
> 本阶段产物：独立 Lean 镜像/桥骨架 + 无占位试点定理 + 可执行的完整签收计划。

## 0. 名分与证据状态

本镜像是 3a 生产单扫描装配语义的测试锁与形式化镜面，不进入生产判定，也不构成同一判断的第二档。
Rust 生产代码仍是被检实现；Lean 文件固定“给定同一装配输入，输出应是什么”。只有 Rust 提取记录经
指定窗口逐字段对拍后，才可宣称跨语言签收。

本轮按要求尝试读取：

```text
gh issue view 1080 --json body
gh issue view 1077 --json body
```

两条命令均因当前运行环境无法连接 `api.github.com` 失败。因此 #1080 与 #1077 的条文以编排者本轮
给出的票面文字为准，未冒充 live 复核。ADR 0025 的工作树文件不存在，已按指示读取：

```text
git show 6043d144fb:docs/adr/0025-scan-unify-defense-lean-mirror.md
git show f085c2bef7 -- docs/adr/0025-scan-unify-defense-lean-mirror.md
```

其中 `f085c2bef7` 的 #1060 补充是最终约束：`strict_nest_check` 不退役，3b 将其转成镜像对拍执行器；
保留每 checkpoint、末根终态、逐级全比对和 FAIL 停线，只替换对拍对象；P2 并入统一记录与
`cp_ownership` 投影，不另立第二套校验。

代码发现原计划使用 `codebase-memory-mcp`。项目枚举与快速索引两次均被环境取消，故本阶段降级为只读
`rg`、`nl` 与精确文件读取。下面行号均以 `355b839c29` 工作树为准。

## 1. 精确靶面

### 1.1 生产入口与单扫描 seam

生产入口是 `rust/src/theta_v0/classifier/scan.rs:61` 的 `merged_scan_resume`，四个直接产口由
`MergedScanOutput`（`:43-49`）固定：

1. `points: Vec<BspPoint>`；
2. `pan_divs: Vec<PanDivCert>`；
3. `grades: Vec<FirstClassGradeRecord>`；
4. `observations: Vec<CandidateObservation>`。

`pipeline.rs:1294-1373` 是 L0 与 L≥1 的生产调用点。L0 使用原线段并回退结构自锚；L≥1 把递归单元投影
成线段，显式传结构方向锚与 departure 真终点。四产口在 `pipeline.rs:1388-1400` 同一 memo key 下锁步
缓存；`cp_ownership` 则在 `pipeline.rs:1423-1431` 独立投影进 `LevelState`。

边界订正：`pipeline.rs:1374-1389` 的二类点由 07b `extract_second_resume` 另行产生，随后才并入最终
`LevelState.bsp`。因此本镜像的 `BspPoint` 产口只覆盖 3a 自身的一类/零 bit 结构候选与第三类，不能把
二类误列成 `MergedScanOutput.points` 的直接产物。

### 1.2 Prelude 镜面清单

| 编号 | 装配语义 | Rust 唯一靶面 | Lean 第一阶段定义 | 状态 |
|---|---|---|---|---|
| P-01 | resume 排序守卫：segment start 非降、center end 严增 | `scan.rs:82-94` | `segmentStartsOrdered` / `centerEndsStrict` / `orderedResumeGuard` | 已建骨架与反例定理 |
| P-02 | 生产方向锚等于结构方向；平行数组等长 | `scan.rs:95-114` | `SegmentRow.anchor` / `anchorsAreStructural` | 已建骨架；长度 wire 未接 |
| P-03 | 最近 confirmed 中枢：`partition_point(end<=start)-1` | `signal.rs:202-228`、`scan.rs:146-150` | `nearestConfirmedCenterIdx` | 已建函数与唯一性定理 |
| P-04 | 局部趋势门按入边 ownership；C0 无入边 | `decompose.rs:198-218`、`scan.rs:151-156` | `incomingBlockAt` / `resolveTrendGate` | 已建函数与正/反试点 |
| P-05 | 本级盘整门：Consolidation 且 lift=0 | `decompose.rs:220-253`、`scan.rs:157-160` | `resolveConsolidationLiftZero` | 已建函数；尚未进统一扫描 driver |
| P-06 | `first_match_idx` 保留首三元组匹配；生产 strict-end 下 `pos==c_idx` | `signal.rs:1816-1834`、`scan.rs:91-94,151-156` | `firstMatchIdx` / `FirstMatchedBy` | 已建首匹配与唯一性；strict-end 等价尚未普遍证明 |
| P-07 | A 段与 b 包络按 `c_idx` 缓存，hit 复用，miss 才预算 | `scan.rs:311-320` | `lookupACache` / `resolveASegment` | 已建函数与 hit 透明定理 |
| P-08 | λ_C 逐段算：最后回中枢后首同向锚，不按 `c_idx` 缓存 | `divergence.rs:872-913`、`scan.rs:321-322` | `episodeWindow` / `lastReentryBoundary` / `lambdaCEpisode` | 已建函数与唯一性定理 |
| P-09 | 结构门唯一 writer：破中枢→A→λ_C；映射失败留在 gates | `signal.rs:230-325` | `firstStructuralGates` | 已建骨架与缺 A 拒绝定理 |
| P-10 | confirmed prefix 四缓存锁步；tail 重判；四产口排序 | `scan.rs:122-228` | 尚无完整 scan driver | 未竟面 |

重要区别：full 神谕会稳定排序并构造 `first_match_idx` 兼容重复三元组；生产 resume 因 center end 严格
递增，热路不建 HashMap，直接使用 `c_idx`。镜面保留“首匹配”语义供 full 对拍，但不把它谎称为生产热路
的实际分配步骤。

### 1.3 四个产口逐项清单

#### O-01 `BspPoint`

生产函数：

- 第一类/零 bit 结构候选：`signal.rs:452-604` `judge_first_from_gates`；
- 第一类字段装配：`signal.rs:1040-1061` `make_first_point`；
- 第三类真值与证书：`signal.rs:648-698` `judge_third_cert`；
- 单扫描 sink：`scan.rs:333-350,373-386`。

必须逐字段提取：`source_index`、六个 BSP bit、第三类 entry identity、`pivot_low/high`、owner
（`Center` 或 `Type1Anchor`）、`struct_break_dir`、force 五/现役四 proxy 原始字段、
`retrace_breaks_type1`。3a 对拍域必须声明：二类 bit 恒不由本扫描直接产生。

第一阶段 Lean 已落：`judgeFirstFromGates`、`judgeThirdPoint`、`BspPointMirror` 的核心字段与严格第三类
正例。尚缺完整六 bit/owner union/pivot/force/retrace wire，因此当前只能称“桥骨架”，不能称 O-01 全字段
已签收。

#### O-02 `PanDivCert`

生产函数：`signal.rs:1382-1428` `judge_pan_div`，调用门在 `scan.rs:363-370`。只有本级
Consolidation 且 lift=0 进入；A/C 映射或力度判定失败均不产证书；成功字段为
`source_index/side/center/seg_a/seg_c`。它不产 `BspPoint`，不置 six-bit。

第一阶段 Lean 已落 `assemblePanDivCert`、`PanDivCertMirror`、A/C 区间保持定理。尚缺 pan 结构定位、窄锚
到 front-anchor 回退、真实力度原语与完整 center 字段 wire。

#### O-03 `FirstClassGradeRecord`

生产类型：`signal.rs:833-863`；唯一写入点在 `judge_first_from_gates` 的 `diverged` 分支
（`signal.rs:548-572`），位于 T3-in-c 二次门之前，故 Present/Missing 两域都记。字段必须逐项提取：
`level/source_index/side/center_start_index/center_end_index/center_zd/center_zg/grade`。

第一阶段 Lean `assembleFirstClassGrade` 与 `GradeParity` 已覆盖以上字段；`Grade`/Rust wire 同时保留
Present 的 leave/retest 两区间，以及 Missing 的五种 reason。定理
`grade_projection_does_not_require_t3_present` 锁定“只依赖 diverged，不依赖 T3 Present”。

#### O-04 `CandidateObservation`

生产类型：`cand_event/observe.rs:24-44`。Trend 投影是 `make_trend_observation`（`:189-232`）；Pan 投影
是 `pan_observations_for_level`（`:234-277`）；同 key episode 腿归约是 `reduce_structural_legs` 与
`merged`（`:84-146`）。必须逐字段提取：

- `CandidateKey` 全字段（rule version、level、kind、side、前中枢、parent 指纹、`seg_a`、λ_C）；
- `center_ids`、`candidate_group_id`、`pair_id`；
- `StructuralPredicates(direction/comparable/extreme)`；
- `extreme_proof`、`third_class_proof`；
- `interval/state/first_provable_at/confirmed_at`。

归约必须锁：同 key；右端更大腿作 carrier（相等保 acc）；Extreme 析取；Comparable 随最新腿；state 重算；
首证钟优先 acc、再 leg、再首次成立 carrier 右端；最终按 `(interval,key)` 排序。Pan 观察只从
`PanDivCert` 投影，不重判结构或力度。

第一阶段 Lean 已落核心 key、谓词、interval/state/clock、Trend/Pan 投影、`mergeStructuralLeg` 与未排序
列表归约。尚缺 rule version、两个 stable ID、`center_ids`、两个 proof 字段、与 Rust FNV 算法的逐位
wire，以及归约后的 canonical `(interval,key)` 排序。

### 1.4 `c_p` 生命周期逐项清单

生产对象：`recursive_tower.rs:515-553` `CpScanOwnership/CpLifecycleStatus`；pipeline 附着与更新：

1. 初始化：`cp_scan_ownership`（`recursive_tower.rs:741-772`），初态 Pending；有 departure 时只建开放左端；
2. P=0 clear / P>0 truncate / frontier pop：`pipeline.rs:901-919,958-982,1019-1059`，与 centers、
   upper_moves、win_meta 锁步；
3. dirty 失效：`recursive_tower.rs:1574-1666`，terminal 脏回 Pending 并清闭合证书，只有 review 脏则保
   Closed/third、清复核结果；
4. 同身份稳定重继承：`pipeline.rs:1093-1140`，身份四项为 b_center_id、b_center、departure_move_id、
   departure_interval；
5. 尾部 extend 后统一推进：`pipeline.rs:1190-1206`；
6. Pending→Closed：`recursive_tower.rs:1864-2015`，复用 `judge_third_cert`，命中后原子写 lifecycle、
   confirm、完整 c 结构、third 与完整趋势证据；Closed 在 stable revision 内吸收；
7. 长度不变量：`pipeline.rs:1222-1226`，centers/upper_moves/cp_ownership/win_meta 1:1。

第一阶段 Lean 已落 `initCpOwnership`、`cpDependenciesStableBefore`、`invalidateDirtyCp`、`reinheritCp`、
`advanceCpLifecycle` 与四条试点定理。当前证书只保留 terminal/review/confirm/third/qualified 的最小形状；
完整 ElementId、Center、CpStructureIdentity、ThirdClassInCp、FullTrendQualificationEvidence 和列表锁步 driver
尚未接入。

## 2. 桥定义

新文件：`formal/Origin/UnifiedScanMirrorBridge.lean`。不修改 EngineBridge、LedgerBridge、
BspEventBridge 或任何 Rust 文件。

桥沿用既有三类做法：

1. `Rust*Tag`/`Rust*Extraction` 与 Lean 镜像结构是独立类型，避免同源结构整体相等造成伪桥；
2. `BspParity/PanParity/GradeParity/CandidateParity/CpParity` 明列字段，不用总数或摘要替代；
3. `ListParity` 强制同长、同序、逐项满足对应关系；
4. `OutputParity` 对四产口分别检查，不允许“一个产口多、另一个少但合计相等”；
5. `FirstBspCheck/ThirdBspCheck/PanCheck/GradeCheck/TrendObservationCheck/PanObservationCheck/
   StableCpAdvanceCheck` 会现场运行单产口 Lean 函数后比对 Rust wire；但这些入口仍接收已经裁判过的
   Lean 侧结构输入，尚未绑定独立 raw Rust input wire；
6. `OutputParity` 是底层诊断关系，允许显式传入 Lean 输出，**不得作为最终防绕过入口**；
7. `bsp_bridge_rejects_side_mismatch` 给出负例，证明 BSP 字段桥不是恒真关系。

最终防绕过执行器必须实现的目标数据流（本阶段尚未实现）：

```text
Rust merged_scan_resume + LevelState.cp_ownership
  -> 每 checkpoint / 末根逐级序列化 Rust*Extraction
  -> Lean 解码同一 prelude 输入
  -> Lean 现场运行镜像函数
  -> 各产口 ListParity + cp parity
  -> 任一字段 mismatch 非零退出，strict_nest_check 停线
```

当前没有独立 raw input wire、完整 `mergedScanMirror` driver、Rust serializer、Lean 文件/JSON parser、
runner 或 `strict_nest_check` 接线；因此准确名分是“逐字段 parity 与单产口桥骨架已定义、顶层防绕过桥和
真实对拍未接”。wire 格式应在下一切片选择仓内可复现格式，并固定整数/布尔/枚举编码；不得用 Rust Debug
串作为长期协议。

## 3. 试点定理清单

| 类别 | 定理 | 锁定内容 |
|---|---|---|
| 排序守卫 | `ordered_resume_guard_rejects_unsorted_segments` | 乱序 resume 输入拒绝 |
| 最近中枢 | `nearest_confirmed_center_unique` / `nearest_confirmed_center_selects_latest_prefix` | 唯一且具体选择已确认前缀末项 |
| 首匹配 | `first_match_unique` / `first_match_keeps_first_duplicate_key` | 唯一且重复三元组保留首项 |
| 趋势门 | `trend_gate_reads_incoming_relation` / `trend_gate_rejects_c0` | 入边块给方向，C0 不借未来方向 |
| 盘整门 | `consolidation_gate_accepts_c0_lift_zero` | C0 按首块归属，本级 lift=0 放行 |
| A 缓存 | `a_segment_cache_hit_is_transparent` | hit 结果不被 fresh 重算覆盖 |
| λ_C | `lambda_c_episode_unique` / `lambda_c_reentry_restarts_episode` | 唯一且回中枢后从新腿重启 |
| 结构门 | `structural_gate_rejects_missing_a` | 缺 A 不构造候选身份 |
| 第一类 BSP | `first_projection_keeps_structural_candidate` | 成功输出必保 structure-break 身份 |
| 第三类 BSP | `third_projection_sets_only_third_bit` | 严格第三类买正例只置 third bit |
| Pan | `pan_projection_preserves_both_intervals` | 证书保持 I(A)/I(C) |
| Grade | `grade_projection_does_not_require_t3_present` | diverged 即记真实 grade，不要求 Present |
| Trend Observation | `trend_observation_first_clock_is_conditional` | 只在 Provisional 落首证钟 |
| Pan Observation | `pan_observation_is_confirmed` | Pan 证书投影直接 Confirmed |
| episode 归约 | `merge_structural_leg_extreme_is_or` | Extreme 一旦成立不被后腿撤销 |
| c_p 初始化 | `cp_initializes_pending` | 新对象 Pending 且无闭合证书 |
| c_p stable 推进 | `cp_closed_absorbing_in_stable_revision` | Closed 在同 revision 吸收 |
| c_p dirty | `cp_dirty_terminal_reopens_pending` | terminal 依赖脏则回 Pending |
| c_p 重继承 | `cp_reinherit_stable_identity` | 同身份且依赖稳定才继承旧态 |
| 四产口桥 | `output_parity_exposes_each_port` | 对拍义务不能折成合计 |
| 桥非平凡 | `bsp_bridge_rejects_side_mismatch` | side 冲突不得桥接 |

所有试点均为普通 Lean 定理，无证明占位、外加公理或承认式声明。

## 4. 构建验证

formal 构建配置：`formal/lean-toolchain` 固定 `leanprover/lean4:v4.31.0`；`lakefile.toml` 的 Origin 库使用
`srcDir = "."`。本阶段不改既有 `lakefile.toml`，采用独立新文件编译：

```text
cd formal
lake env lean Origin/UnifiedScanMirrorBridge.lean
```

2026-08-21 本工作树实跑结果：退出码 0，标准输出/错误输出为空。

文本门：

```text
grep -c sorry formal/Origin/UnifiedScanMirrorBridge.lean
0
```

说明：新文件尚未登记为 Origin root，因此默认 `lake build` 不会自动覆盖它；本阶段的可复现验证命令是
上述单文件命令。登记 root 属下一切片集成动作，不能把“单文件可编译”描述成“默认全构建已接线”。

## 5. 签收五件事执行计划（#1077 D4 批准版）

### 5.1 窗口、数量与不重叠证明

按统一输入流的零基 bar 序号固定四窗：

| 窗 | 半开区间 | 长度 | 角色 |
|---|---:|---:|---|
| W20 | `[0, 20_000)` | 20k | 小窗快速门 |
| W100 | `[20_000, 120_000)` | 100k | 中窗独立门 |
| W300 | `[120_000, 420_000)` | 300k | 大窗独立门 |
| W500 | `[0, 500_000)` | 500k | 重窗，允许且刻意覆盖前三窗 |

前三窗不重叠的机械证明是 `20_000 ≤ 20_000`、`120_000 ≤ 120_000`，并且
`W20.end ≤ W300.start`。实现 runner 时把 `(start,end)` 写入结果头并断言任意两轻窗满足
`a.end ≤ b.start || b.end ≤ a.start`；W500 标记 `heavy_overlap=true`，不得混入不重叠断言。

每个窗口必须各自提取至少 100 条“可进入任一对拍产口或 cp 生命周期检查的记录”。不足 100 直接判窗失败，
不能用其他窗口补足，也不能把四窗相加后宣称达标。

### 5.2 每窗各自 0 mismatch

每窗独立输出：

```text
window_id, start, end, checkpoint_count, extraction_event_count,
bsp_mismatch, pan_mismatch, grade_mismatch, observation_mismatch, cp_mismatch, total_mismatch
```

通过条件是每一个窗口的五个 mismatch 计数均为 0，继而 `total_mismatch=0`。禁止只看四窗合计；任一窗口
非零即总签收失败，并以首个 mismatch 的完整字段路径、Rust 值、Lean 值、level、side、checkpoint、bar
非零退出。

### 5.3 逐产口枚举

每 checkpoint 与末根终态、每个 level 都必须依次枚举：

1. `MergedScanOutput.points`；
2. `MergedScanOutput.pan_divs`；
3. `MergedScanOutput.grades`；
4. `MergedScanOutput.observations`；
5. `LevelState.cp_ownership`。

每个列表检查长度、canonical 顺序和逐字段内容。空列表是合法值，但必须以 `count=0` 明示，不能省略产口。
`points` 的 direct-3a 与 pipeline 后加二类要分开导出，避免口径串线。

### 5.4 `level × side × 链终态` 分层

结果同时生成三维列联表：

- `level`：所有实际出现的 `u32` 级别，另报最大级与缺级；
- `side`：Long / Short；无方向的 cp Pending 对象单列 `NotYetKnown`，不得猜方向；
- `chain_terminal`：`Pending / Closed / NoCpApplicable`。

每格报告 extraction 数与各产口 mismatch。对于某个窗口未出现的格子，明确写 0 并列入覆盖缺口；不得删除空格
来制造 100% 分层覆盖。最终签收至少要求两个 side 都在四窗合计出现，Pending/Closed 都出现；若真实数据没有
覆盖，增加独立窗口或 fixture，不能降低分层要求。

### 5.5 覆盖率、点估计与下界方法

报告两种覆盖率，不能互相替代：

1. **字段覆盖率**：实际进入 parity 的字段数 / 本 spec 逐产口声明字段数。签收要求 100%；
2. **分层覆盖率**：非空 `level×side×终态` 格数 / 运行前由实际 level 集合展开的目标格数。报告点估计，
   并逐项列空格；不以一个百分比掩盖缺失类别。

匹配率点估计为 `matches / compared_fields`。每窗 0 mismatch 仍是硬门；另以二项分布 Clopper-Pearson
单侧 95% 方法报告真实 mismatch 概率上界：零失败、n 次比较时上界
`1 - 0.05^(1/n)`。该统计只描述有限抽样不确定性，不放宽“每窗 0 mismatch”。

## 6. 下一切片接线顺序

1. 补全 Lean 记录与 parity：O-01/O-02/O-04 目前缺的全字段，以及 cp 完整证书字段；
2. 定义独立 raw Rust input wire，只允许其单向解码成 Lean 输入，移除已裁判 Bool/结构的注入通道；
3. 补 `mergedScanMirror` driver：ordered resume、frontier freeze、四缓存锁步、advance+tail、排序与归约；
4. 固定 Rust wire schema 与版本号，写只读提取器；
5. 写 Lean 解码/runner，禁止接受调用方注入任意 Lean 输出；
6. 将 runner 接到 `strict_nest_check` 的每 checkpoint 与末根硬门；
7. 登记 Origin root，跑单文件、Origin 库与靶向 Rust 测试；
8. 跑 W20/W100/W300，逐窗满足后再跑 W500；产出不可变结果包与分层表。

## 7. REPORT（Phase 2，2026-08-21）

### 7.1 执行器与证据包

Phase 2 新增只读捕获 seam `rust/src/theta_v0/classifier/diag/s2_mirror_capture.rs`、执行器
`rust/src/bin/s2_lean_mirror_extract.rs` 与 Lean 执行层
`formal/Origin/UnifiedScanMirrorRunner.lean`。捕获默认关闭；验收 bin 才打开 thread-local 槽，生产
`merged_scan_resume`、`judge_first_from_gates`、`make_trend_observation`、
`reduce_structural_legs` 与 `LevelState.cp_ownership` 仍是唯一数据源。没有改写生产判据或引入 fallback。

固定命令：

```text
cd rust
cargo run --release --bin s2_lean_mirror_extract -- \
  ../analysis/data_cache/btc_1m_full.json \
  ../analysis/s2-lean-mirror-20260821
```

执行器把 Lean 检查按每片最多 500 条切分，4 个 worker 并行执行；任一片非零即整窗非零退出。manifest、
fixture 与逐窗 JSON 均在 `analysis/s2-lean-mirror-20260821/`。前三轻窗的半开区间不重叠断言由 bin 单测
`approved_windows_keep_three_light_ranges_disjoint` 机械锁定；W500 明写 `heavy_overlap=true`。

### 7.2 四窗实跑读数

| 窗 | 半开区间 | checkpoint | 提取/枚举记录 | 比较字段 | mismatch | 产口对象 `prelude/point/pan/grade/obs/reduce/cp` | 95% 零失败上界 |
|---|---:|---:|---:|---:|---:|---:|---:|
| W20 | `[0,20000)` | 4 | 250 | 3,320 | **0** | `11/0/23/0/23/11/105` | 9.019220e-4 |
| W100 | `[20000,120000)` | 20 | 4,742 | 105,682 | **0** | `55/20/819/20/842/55/2528` | 2.834626e-5 |
| W300 | `[120000,420000)` | 60 | 40,627 | 1,012,257 | **0** | `176/171/7178/171/7421/176/24013` | 2.959454e-6 |
| W500 | `[0,500000)` | 100 | 112,535 | 2,776,161 | **0** | `302/378/21862/378/22382/302/64666` | 1.079091e-6 |

四窗均超过 100 条下限，且各自产出 `mismatch_fields=0`；没有用四窗合计替代逐窗硬门。W20 的
`points`/`grades` 对象数为 0 是真实空域，不拿其他窗补数；两产口仍随 11 次 scan 各自枚举并在 JSON 的
`port_enumerations` 明列。其余窗已有非空 point/grade 对象。

### 7.3 分层表

逐窗 JSON 的 `candidate_layers` 完整展开实际 level × Long/Short × Open/Closed/Invalidated；空格保留
`extraction_count=0`，不删除、不充数。`cp_layers` 另按 spec 将无方向对象列为 NotYetKnown ×
Pending/Closed：

| 窗 | 实际 level | candidate 非空格 / 空格 | cp Pending / Closed | 分层 mismatch |
|---|---|---:|---:|---:|
| W20 | `0,1,2` | `4 / 14` | `69 / 36` | **逐格 0** |
| W100 | `0,1,2,3` | `10 / 14` | `1370 / 1158` | **逐格 0** |
| W300 | `0,1,2,3,4` | `20 / 10` | `11758 / 12255` | **逐格 0** |
| W500 | `0,1,2,3,4` | `24 / 6` | `33764 / 30902` | **逐格 0** |

### 7.4 完成标准逐件裁决

| 完成标准 / 签收项 | Phase 2 状态 | 证据 / 缺口 |
|---|---|---|
| spec 与 Phase 1 桥 | **已签** | 本文件；`formal/Origin/UnifiedScanMirrorBridge.lean` 未改 |
| Rust 提取执行器真实接线 | **已签（只读 seam）** | release 四窗均真实运行；任一 Lean chunk 失败会使 bin 非零 |
| 四窗逐窗 0 mismatch | **已签（当前桥声明字段）** | §7.2 四个 JSON 各自 `mismatch_fields=0` |
| 每窗 ≥100 记录 | **已签** | `250 / 4742 / 40627 / 112535` |
| 逐产口枚举 | **已签（含空产口）** | JSON `port_enumerations`；W20 point/grade 对象空域照实保留 |
| `level×side×终态` 分层与空格 | **已签** | JSON `candidate_layers`/`cp_layers`；§7.3 摘要 |
| 点估计与 95% 上界 | **已签** | 点估计四窗均 0 mismatch；§7.2 用 `1-0.05^(1/n)` |
| 每类装配步骤无 `sorry` 定理 | **已签（已覆盖的执行层）** | Phase 1 试点 + runner 的 reduction/cp/ForceL/T3 定理；文本门另跑 |
| 默认 Origin build 覆盖 | **已签** | `formal/lakefile.toml` 已登记 bridge 与 runner roots |
| Lean 完整全字段镜像 | **未签** | Phase 1 REPORT 所列 O-01 完整 six-bit/owner union/pivot/force/retrace，O-04 rule version/FNV/center_ids/proof/canonical sort，以及 cp 完整证书仍未补齐；本轮不得把 0 mismatch 膨胀成这些未比较字段已签 |
| 完整 raw-input `mergedScanMirror` / frontier 四缓存 driver | **未签** | runner 已从 raw ForceL bit 与固定首对 segment 现场重算一类/T3，并现场跑 observation/reduction；但 P-10 完整 prefix-cache/frontier driver 仍未形式化 |

Phase 2 结论：**执行器、四窗、逐产口枚举、分层表、≥100 下限和当前桥字段的逐窗 0 mismatch 已有真实
证据；但“Lean 完整全字段镜像”与 P-10 顶层 driver 仍未签，因此 #1080 的“完整语义镜像”总签收不得
报绿。**

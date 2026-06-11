# C 段边界修复验证报告（纯 Rust 链路，OKLO 447K）

2026-06-11 · 接续前 session 的 C 段修复任务（engine_bsp_gap_diagnosis 修复A + B2）。
验证方式：crate 内集成测试 `rust/src/c_segment_verify.rs`（`cargo test --release
c_segment_oklo -- --ignored --nocapture`），不走 Python→PyO3 链路。

## 1. 结论

**修复A + 修复B2 验证通过。** 两项修复已在工作树（`rust/src/level.rs` /
`divergence.rs` / `orchestrator.rs` / `buysellpoint.rs` / `segment_layers.rs` /
`bi_zhongshu_bsp.rs`，+250/−26 行），74 个既有单测全过，OKLO 447K 全量逐 bar
per-level BSP 计数达到诊断预registered 判据：

| 层（ladder 口径） | 修复前（诊断基线） | 修复后（本次实测） | 判据 |
|---|---|---|---|
| ladder2 笔中枢级 type1 confirmed | 瞬态闪烁（终态≈1） | **374** | ≫1 ✓ |
| ladder2 笔中枢级 type2 confirmed | 0 | **360** | >0 ✓ |
| ladder3 走势级（L1）type1/type2 confirmed | —（C段恒空致稀疏） | 60 / 58 | — |
| ladder4 递归 L2 type1 confirmed | 0（16/16 c_empty，全 type3） | **8** | >0 ✓ |
| ladder4 递归 L2 type2 confirmed | 0 | **8** | >0 ✓ |
| ladder5 递归 L3 type1/type2 confirmed | 0 | 1 / 1 | — |

全量计数落盘：`analysis/data_cache/c_segment_fix_rust_verify.json`。

与诊断文档（engine_bsp_gap_diagnosis.md §B2 模拟）的对照：模拟预测 ladder2
373 confirmed type1 / +360 confirmed type2；实测 **374 / 360**。type2 精确吻合；
type1 +1 差异来源 = 诊断是"仅终态"的模拟近似，实测是真实增量实现（含 pending
move 的 C 段处理路径差异）。诊断文档自陈"换度量则 373/360 会变，但 0→数百的
量级跃迁是本质"——量级跃迁成立。

## 2. 定义依据

- **修复A**（`level.rs::moves_from_level_zhongshus`）：递归层 move 的 seg_end
  扩展语义与 level-1（`moves.rs::group_to_move`）对齐——非末组 seg_end =
  下一组首中枢 comp_start − 1，末组 = num_components − 1。修复前直接取
  last_zs.comp_end，使趋势背驰 C 段（离开最后中枢后的组件）恒为空区间
  （c_start = comp_end+1 > c_end = comp_end）。
- **修复B2**（`divergence.rs::detect_trend_divergence`）：C 段搜索窗口不被
  mv.seg_end 截断，延伸到下一 settled 中枢 seg_end（无 → n−1），C 段终点 =
  窗口内趋势极值段。依据缠师第24课：背驰段终于走势转折点；相邻中枢首尾相接时
  转折极值落在下一中枢覆盖区内。
- 增量器一致性：`detect_dep_end` 把窗口无界的 move（无后继 settled 中枢）标记
  i64::MAX 不 commit，留在易变尾每次重算；`orchestrator.rs` 新增 volatile_floor
  约束 IncrementalSegBsp 的 stable anchor——B2 后易变 div 的 seg_c_end 可远落于
  stable_anchor 之前。

## 3. 边界条件（结论何时翻转）

1. **力度度量变更**：当前 df_macd=None 路径力度 = 振幅 × 组件跨度（521号边界，
   拓扑代理）。换 MACD 度量或改 TYPE1_CONFIRM_RATIO=0.9 阈值，374/360 的具体
   数字会变；若新度量下 confirmed 归零则结论翻转。
2. **数据基底**：OKLO 447K（alphavantage 全时段，2024-05 起）单标的单时段 =
   认识论等级 **L2**。其他标的/时段未验证；若多标的复验中递归层 type1 重新
   归零，修复有效域结论翻转。
3. **Python 侧未对齐**：本验证是纯 Rust 链路。Python 引擎
   （recursive_level_engine.py / a_divergence_v1.py 等已同步修改，git diff 在册）
   的逐位等价**未在本任务中重新差分**——若 Python↔Rust 差分发现发散，
   "修复完整"的声明翻转（数字本身不翻转）。
4. **per_level_bsp 适配器契约**：递归层 BSP 依赖"segments = 前级 moves 全列表、
   不重排不过滤"的索引自洽契约。若上游改变 LevelSnapshot 的组件编号方式，
   计数失效。

## 4. 下游推论

1. **Version I / 有机赋格的递归层信号源激活**：ladder4+ 此前结构性无 type1/type2
   （只能用 type3 或 move-settle 代理），修复后高层 type1 锚可用——
   fugue_version_i 的"动态级别归属"和 REV 配对的同锚 Buy1 闭腿在高层有了真实锚。
2. **磁带语义代次更替**：引擎 BSP 输出变化 ⟹ compute_organic_signals 磁带指纹
   变化 ⟹ 旧 V2 回测在册结果（rev_v2_paired_backtest.json）不可增量混写。
   重跑必须用 BT_TAG=enginefix 落新文件（脚本已预留该通道）。
3. **ladder2 type3 主导地位不变**：2543 type3 vs 374 type1——type3 仍是笔中枢级
   的多数事件类型，下游消费 type1 时的稀疏性假设（如 41 课门）需按新密度重校。

## 5. 谱系引用

- `project_bsp_gap_root_cause`（修复A/B2 的根因诊断：seg_end 错位 16/16 实证 +
  c_empty 1120/1121）
- 521号（candidate+MACD闸 ≡ BSP type1 confirmed 同构；力度代理边界）
- 525号（组件来源无关性——适配器契约的依据）
- 谱系 532（seg-end-trigger-axis-split，工作树中同步修改）
- 形式化有效域规则：本验证 = **L2**（单标的真实数据）；非 L3（多标的交叉验证
  未做）。

## 6. 影响声明

- **新增** `rust/src/c_segment_verify.rs`：crate 内验证测试（#[ignore] 长测试，
  显式运行），含 per_level_bsp.py 适配器的 Rust 复刻（level_bsps）+
  笔中枢级全链复刻（bi_zhongshu_bsps）。
- **新增** `rust/src/lib.rs` 模块声明 2 行（#[cfg(test)] mod c_segment_verify）。
- **新增数据** `analysis/data_cache/oklo_447k_ohlc.f64`（447,739 bar × 32B，
  由 oklo_1m_databento.json 一次性转换，供 Rust 测试零依赖读取）、
  `analysis/data_cache/c_segment_fix_rust_verify.json`（计数落盘）。
- **不改动**任何引擎源文件——修复代码本身是前 session 产物，本任务只验证。
- 既有 74 个 Rust 单测全过；新测试默认 ignore，不拖慢常规 cargo test。

## 附：收尾四问（编排者 152 轮中断，最终口径）

**1. C 段修复改了哪些文件？cargo build 通过吗？**
引擎 6 文件：`rust/src/level.rs`（修复A：seg_end 扩展对齐 level-1）、
`rust/src/divergence.rs`（修复B2：C 段越界极值窗口 + detect_dep_end）、
`rust/src/orchestrator.rs`（volatile_floor 接线 + num_components 传参）、
`rust/src/buysellpoint.rs` / `segment_layers.rs` / `bi_zhongshu_bsp.rs`
（增量器签名适配），合计 +250/−26 行。Python oracle 同步 4 文件
（a_divergence_v1 / a_zhongshu_level / recursive_level_engine /
a_nested_divergence）。**cargo build --release 通过，74 个单测全过**
（需 PATH 前置 /usr/bin 绕过 cc 劫持）。全部已 commit。

**2. 修复后 type1/type2 数量变化？**（OKLO 447K，confirmed 计数）

| 层 | type1 修复前→后 | type2 修复前→后 |
|---|---|---|
| ladder2 笔中枢级 | 1 → **374** | 0 → **360** |
| ladder3 走势级 L1 | 稀疏 → 60 | → 58 |
| ladder4 递归 L2 | 0 → **8** | 0 → **8** |
| ladder5 递归 L3 | 0 → 1 | 0 → 1 |

诊断模拟预测 373/360：type2 精确吻合，type1 +1（模拟近似 vs 真实增量实现）。
Rust 自验证断言已收紧为 ladder2 type1 > 100、ladder4 type1 > 0、type2 > 0
（c_segment_verify.rs，实测全过）。笔/线段/中枢层：源码零改动（修复不触
stroke/segment/zhongshu/bi_engine），结构输出不变。

**3. entry 能到几了？**
**3 → 4**（rev-paired 工位 commit 9e0504c622）：OKLO 新磁带入场归因
ladder2 160 / ladder3 32 / **ladder4 3** 笔。tranche 定义域（需 entry≥4）
从空集变非空：V1r tranche 加码 0→12 次（行为面首读 OKLO −27.4pp，n=12）。

**4. 回测跑了吗？数字？**
跑了——enginefix 代次三标的全变体在册
（`rev_v2_paired_backtest_enginefix.json`），O0≡P5 bit-exact 三标的全 PASS：

| 变体 | OKLO（BH+307%）| QQQ（BH+175%）| BRN（BH+87%）|
|---|---|---|---|
| P5 基线 | +1088.83% | +100.62% | +28.25% |
| V2o（仅震荡型）| **+1625.81%**（Δ+537pp）| +89.34%（Δ−11.3pp）| +94.15%（Δ+65.9pp）|
| V2p（开全 kind）| +670.44%（Δ−418pp）| +66.28%（Δ−34.3pp）| **+185.73%**（Δ+157.5pp）|

核心判决：震荡型腿三标的净现金全正（胜率 51.7-52.5%）；**逃逸型 OKLO/QQQ
强负但 BRN 转正（+1.66万）**——"逃逸型一致为负"否证收窄为 5/6，kind 消融
是标的依赖的。V1f OKLO 旧 +607.5pp 被证伪为 C 段 bug 伪影（修复后 +57.6pp）。
详见 rev_v2_rust_backtest.md / rev_v2_enginefix_impact.md。

**Python 等价测试已按编排者裁决终止**（定义变了，旧 oracle 不适用——
等价测试无意义；验证主轴 = Rust 自验证 + O0≡P5 守卫 + 三标的回测复现）。

## 附：任务卡第三/四步状态

- **第三步（REV 配对修正）**：已由前序 commit（bd1e439914）完成——
  `trading/level_operating_unit.rs` 含 rev_paired（kind-aware 开腿：震荡型/逃逸型）、
  同锚配对闭腿（ZD 触线 / 同锚 Buy1）、theta_depth 深度门槛，配套单测在本次
  74 过测之列。本任务无需重复实现。
- **第四步（回测）**：纯 Rust 回测的前提（Rust 信号层产 SignalTape）不存在——
  磁带只能由 Python compute_organic_signals 产出（M2 信号层里程碑）。严格形式 =
  maturin 重建 PyO3 扩展（让 Python 信号层吃到修复后引擎）+
  `BT_TAG=enginefix` nohup 后台重跑（脚本预留通道，不混旧表）。状态见
  rev_v2_rust_backtest.md（如启动）。

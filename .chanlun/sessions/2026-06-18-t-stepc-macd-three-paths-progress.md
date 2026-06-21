# T 步骤c MACD 三条路实装 — 进度交接（2026-06-18）

## 编排者裁决（已 escalate 并裁决）
- escalation: `.chanlun/escalations/2026-06-18-1752-t-stepc-macd-scope-vs-direction.md`
- 裁决：**三条路实测对照**（§6.7 选项C「数据划线」）。纯结构(当前) / T+MACD AND(5条件∧MACD背驰) / T+MACD OR(5条件∨MACD背驰)。三路同数据同操作层，唯一区别=步骤c判定方式。8标的×3=24组**回测**（操作层收益，非vs v3 nf对照）。
- memory: `project_t_stepc_macd_three_paths`

## 架构（已定案）
- 三模式唯一分叉=步骤c力度判据。形态门G(≥2中枢/c创新高/第37课条件2·3·5)三模式共享。
- S=结构力度衰减(嵌套深度/振幅,现状)；M=MACD面积衰减(C<A,按方向Σ非负面积)。
- Structural=G∧F∧S；And=G∧F∧S∧M；Or=G∧[(F∧S)∨M]（M在G内可绕过F∧S）。
- MACD不污染T纯结构定位：Unit携area_pos/area_neg(均≥0)，a₀线段层经merged_to_raw+macd_area_for_range注入，encapsulate求和上传(与inner_zhongshu_count同构)。
- **坐标系**：T的Unit.start_bar/end_bar=segment端点=merged bar；MACD hist在raw bar；必须经merged_to_raw转换。ffi应把BSP.bar转raw bar供回测。
- 操作层必须中性(type1买进/卖出零判定)——否则run_swing_trading内嵌entry_div_ok/exit_div_ok会与step c MACD耦合，违反"唯一区别"(逻辑必然)。

## Part 1 Rust 进度
### ✅ 已完成
- `types.rs`: Unit加area_pos/area_neg字段；stroke()设area=0.0；新增`PerfectionMode{Structural,And,Or}`enum
- `divergence.rs`: 模块文档头改三模式(去"零MACD")；import加PerfectionMode；新增`leg_macd_force(units,dir)`(按方向Σ area_pos/area_neg)；`judge_divergence(t,mode)`；`judge_trend_divergence(t,mode)`(F/S/M布尔化,保留所有对抗核验注释)；`judge_consolidation_divergence(t,mode)`

### ✅ Part 1 全部完成（2026-06-18 后续 session，commit `ffca89af0f`；types/divergence 上次被回滚后本次重做全部 5 文件）
- `operator.rs`: `apply_t(units,level,mode)`；`judge_divergence(t,mode)`；`encapsulate` 聚合 area(Σ求和上传)；测试 mk 闭包加 area + apply_t 调用加 mode；顶部 import 移除 unused `Direction`(下放 tests)
- `mod.rs`: `iterate(a0,mode)`；`apply_t(&current,k,mode)`；`pub use PerfectionMode`；模块头去「不含MACD」声明；测试调用全加 mode
- `ffi.rs`: **方案调整(偏离原 hist+m2r 方案)**——`run_recursive_t(segs, seg_areas=None, mode=None)`(`#[pyo3(signature)]` 向后兼容旧单参数调用)；area 由 Python 端自算(macd_area_for_range，绿柱.abs())经 seg_areas 传入，规避「orchestrator 未暴露 m2r/hist」缺口，保 T standalone；BSP.bar 仍输出 merged 坐标(raw 转换留回测器)；头去「零MACD」声明
- `lib.rs`: line2425 run_recursive_t 注册不变(wrap_pyfunction 自动)，确认 ✓
- 测试: 全部 judge_divergence/iterate/apply_t 调用加 `PerfectionMode::Structural` 保持现有断言 bit-exact；mk 闭包加 area_pos/area_neg；新增 3 三模式单测(And收紧/Or绕过力度S/Or绕过条件5)
- build/test: `cargo build` ✓；`cargo test` recursive_t 30 + 全量 464 全过 ✓
- **未做(本次任务范围外)**：`cargo build --release` + maturin 重建 python 扩展(回测前需)；用户本次明确「不需要跑回测」

## Part 2 待做：中性回测器
- 新脚本 `analysis/t_three_paths_backtest.py`
- 复用 `fugue_v2_full_backtest.load_ohlc` + `SYMBOL_FILES`（8标的:CL/BRN/DX/GC/ES/QQQ/BTC/OKLO）
- 流程：orchestrator(enable_macd_divergence=True,require_settled_subseg=True)逐bar→current_segments()→取merged_to_raw+MACD hist→run_recursive_t(seg_in, hist, m2r, mode)→T BSP(raw bar)
- **关键缺口**：当前orchestrator FFI **未暴露** merged_to_raw / MACD hist 给Python！需在lib.rs给RecursiveOrchestrator加方法暴露(orchestrator.rs有self.bi.merged_to_raw()+OnlineMacdState)。或在run_recursive_t内部重算MACD(传close+segon raw)。**待决**：方案A(orchestrator暴露m2r+hist)vs方案B(run_recursive_t收close自算macd_batch)。方案B更简单(close已有,compute_macd_batch已bit-exact)但需segment的raw bar端点——segment i0/i1是merged,仍需m2r。所以无论如何需要m2r暴露。→ 倾向：orchestrator加`current_merged_to_raw()`+`current_macd_hist()`FFI方法。
- 中性操作层：type1_buy满仓进/type1_sell清仓出，单级别(先move/L1=T.level0)或全type1。用close[raw_bar]成交。算compound/sharpe/mdd vs BH。

## Part 3 待做
- 8标的×3模式跑回测(大数据CL/GC 5.5M、BTC 4.6M bar，单组数分钟，后台跑)
- 对照报告 `analysis/t_three_paths_backtest.md`(24组+regime分布+结果包六要素)
- 重跑 `analysis/t_vs_v3_comparison.py` 看匹配率(任务步骤6)

## 严格性提醒
- 实装后必须同步改divergence.rs/ffi.rs文档「零MACD」声明(已改divergence.rs头,ffi.rs待改)——否则声明膨胀
- 认识论：管线L1，回测结论L2/L3；OR放宽/AND收紧是有效域读数非先验

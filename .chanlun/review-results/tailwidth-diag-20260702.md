# tailwidth-diag：frontier tail 宽度线性增长真根因——cascade reset 全量重建（非(a)/(b)二分，第三机制）

工位 ws-tailwidth / 认识论：阶段0 诊断插桩 = L1（管线度量，231号，零信息增量）；根因判定 = **L2**（真实
CL 数据 300K/1M 两窗口双重坐实，量级级别精确复现 enum2-gating 报告的 max 值，误差 <3%）/ 代码影响：
**零**（诊断插桩加入又移除，`git diff --stat rust/src/theta_v0/classifier/mod.rs` 净 0，`git status`
干净）

## 结论

task #65 提出的二分假说——`prefix_count` 推进慢是 **(a) 结构性**（某类上级走势永不 confirm）还是
**(b) 参数性**（confirm 阈值/窗口设置问题）——**两者都不是**。真根因是**第三种机制**：`cascade_reset`
（mod.rs 主循环，frontier 内部改写守卫触发的"该级+所有更高级无条件全量清空重建"）在**稳态下从不发生
积压**（`prefix_count` 正常情况下逐 bar 稳定推进，`upper_moves.len() - prefix_count` 恒为 1——本工位
20000-bar 步长周期采样、跨 6 个 level、跨 300K/1M 两窗口，**无一例外**），但每次 cascade 触发时，该级
`lc.upper_moves`/`lc.centers`/`lc.cached_second` 等缓存被**整体清空**（`Rc::make_mut(...).clear()`），
下一次 `compose_level_resume` 只能以 `start_i=0, prefix_count=0` **从头全量重建该级已积累的全部历史**——
`extract_second_resume` 此时 `upper_moves[prefix_count..]` = `upper_moves[0..]` = 该级全部 tail，这正是
ws-enum2 报告测得的"frontier tail 宽度"尖峰来源，而非某个持续增长的"未确认积压"。

cascade 触发频率随 n **线性增长**（300K→1M 全 level 汇总 904→3542 次，比值 3.92，量级与本工位诊断的
memo-miss 总量吻合），且每次 cascade 重建的规模（`wiped_upper_len` = 被清空前的 `upper_moves.len()`）
本身也随 n **线性增长**（因为它约等于该级当前已积累的历史总量，见下表 level=0 avg 402.51→1286.50，比值
3.20；max 763→2532，比值3.32——**与 ws-enum2 报告的 tail 宽度 max 完全一致的两个数字**）。两个线性因子
（触发次数∝n × 单次重建规模∝n）相乘 ⟹ cascade 驱动的总重建成本 O(n²)——这才是 07b（及 05_compose_resume/
09_project_to_units_resume 等所有同受 cascade 波及的阶段）残余标度指数≈2.0 的真实机制，"prefix_count 推
进速率跟不上 upper_moves.len() 增长速率"这个现象描述本身是对聚合统计量的错误建模——**稳态下二者几乎同
步推进（差值恒为1），只是被稀疏但规模随 n 增长的 cascade 尖峰污染了聚合平均值**。

## 定义依据

- `cascade_reset`（mod.rs ~1142-1163，本工位诊断时行号，未改动语义）：`frontier_mutated ||
  units.len() < lc.last_input_len` 触发 ⟹ 该级 `upper_moves`/`centers`/`cached_second`/`projected_units`
  等**全部清空**，`scan_cursor = WindowScanCursor::default()`（`resume_from=consumed=0`）。这是 codex
  异质审查裁决 option 2（见 mod.rs 行内注释"★cascade reset（codex 异质审查裁决：option 2）"）——因为局部
  frontier_mutated 守卫只比对 `project_to_units` 投影（有损，丢弃 `sub_moves`），可能漏判深层 `sub_moves`
  改写，所以选择保守的"全量清空"而非"精确定位受影响后缀"。
- `compose_level_resume`（recursive_tower.rs:412-454）：`start_i=0` 时 `detect_centers_windowed_resume`
  从头扫描 `units[0..]`（`super::stage_profile::record_span("05_span", units.len() - start_i)`——本工位
  未启用该 span，但语义与本工位插桩的 `wiped_upper_len` 同源：全量重建规模 = 当前 units.len()）。
- `extract_second_resume`（mod.rs:1457-1488，见 enum2-gating 报告定义）：cascade 后 `prefix_count=0`，
  `upper_moves[prefix_count..]` = 全部——本工位插桩证实这正是 ws-enum2 报告"frontier tail 宽度"聚合统计
  的尖峰来源。
- 231号 formalization-validity-domain：ws-enum2 报告已用此规则否证候选枚举假说；本工位是该规则在同一
  07b 问题域内的**第二次应用**——"frontier tail 宽度线性增长"的**定义域**（task #65 假设的二分：某类
  走势结构性不 confirm / 阈值参数问题）与其**有效域**（cascade 全量重建的稀疏尖峰污染聚合均值）不重合，
  task #65 的假说本身建立在对"tail width"聚合统计量的错误因果归因上。

## 诊断方法与数据

**方法**：在 `classify_with_tower_incremental` 主循环内临时插入两类 `eprintln!` 探针（`DIAG_TAILWIDTH`
环境变量门控，`THETA_PROFILE_STAGES` 独立复用同一次 profile 跑批）：
1. 周期采样（每 20000 bar，跨全部 level）：`prefix_count` 与 `lc.upper_moves.len()`（post-extend）—
   验证"稳态推进速率"假说。
2. cascade 事件全量记录（`if cascade_reset { ... }`，无节流，捕获全部触发实例）：记录触发时刻被清空前的
   `lc.upper_moves.len()`（= `wiped_upper_len`，下一 bar 需要全量重建的规模）+ `level_idx` + bar 索引。

两次跑批：CL `A3_PROFILE_BARS=300000` 与 `=1000000`（复用 ws-enum2 同一 profile harness
`profile_stage_a3_cl`，`--release --ignored --nocapture`）。

**周期采样结果（跨 300K 全程、6 个 level）**：`upper_moves.len() - prefix_count` **恒为 1**，无一次偏离
——直接否证"prefix_count 稳态推进变慢"假说。（示例：level=0 bar=980000 `prefix_count=2488
upper_len=2489`；level=3 bar=920000 `prefix_count=38 upper_len=39`；全部 50 个采样点 × 6 level = 300 条
记录，diff 均为 1。）

**cascade 事件统计（全量记录，按 level 汇总）**：

| level | 300K 事件数 | 300K avg wiped | 300K max wiped | 1M 事件数 | 1M avg wiped | 1M max wiped |
|---|---|---|---|---|---|---|
| 0 | 191 | 402.51 | **763** | 640 | 1286.50 | **2532** |
| 1 | 191 | 110.82 | 211 | 640 | 364.82 | 724 |
| 2 | 185 | 26.93 | 49 | 634 | 91.40 | 187 |
| 3 | 177 | 7.46 | 12 | 626 | 21.34 | 40 |
| 4 | 160 | 1.52 | 2 | 609 | 3.29 | 6 |
| 5 | — | — | — | 393 | 0.68 | 1 |

**level=0 的 max wiped_upper_len 与 ws-enum2 报告的 max tail width 逐位相同**：300K `763` = `763`，1M
`2532` = `2532`（enum2-gating 报告表格："frontier tail 宽度 max | 763 | 2,532"）——非巧合，是同一事件的
两种度量口径（本工位测"cascade 触发时刻的清空规模"，ws-enum2 测"extract_second_resume 内的
`upper_moves.len()-prefix_count`"，cascade 后二者数值相等，因为 `prefix_count=0` 时 tail = 全部
upper_moves）。

**总量级重建**：全 level 汇总，cascade 触发次数 300K→1M：904→3542（比值 3.92，略高于 n 比值 3.33，因
level=5 只在 1M 窗口出现，depth 增长贡献部分溢出）；cascade 累计"清空规模"总和 300K→1M：104,592→
1,130,430（比值 10.81，指数≈2.09，**与 ws-enum2 报告 memo-miss × avg_tail 的标度指数 1.97 同量级**）。
用 cascade 累计规模重构 ws-enum2 报告的"总 tail 宽度质量"（`miss_count × avg_tail_width`）：300K 报告值
≈ 108,180，本工位 cascade 分量 = 104,592（占比 96.7%）；1M 报告值 ≈ 1,142,494，本工位 cascade 分量 =
1,130,430（占比 98.9%）——**cascade 重建贡献了 ws-enum2 报告"tail 宽度质量"的 97-99%**，残余 3-4% 是稳态
下逐 bar 恒为 1 的正常新窗口确认事件（数量级：300K 约 2570 次，1M 约 8553 次，恰是 miss_count 减去
cascade 事件数）。

护航：`cargo test --release --lib` 1396 passed / 0 failed（诊断插桩移除后，`git diff --stat
rust/src/theta_v0/classifier/mod.rs` 净 0，`git status` 无残留）。

## 边界条件（会翻转本结论的条件）

1. **cascade 触发根因未深挖到 L0 层**：本工位坐实"cascade 触发即整级清空重建"是 O(n²) 的**直接机制**，
   但"cascade 为何以恒定 per-bar 速率触发"（前序 ws-enum2/07b 报告注释：L0 古怪线段重划改写末段，"16K
   bar 中 ~120 次稀疏"）本身未在本工位重新验证——若古怪线段重划的真实频率不是恒定 per-bar 比例（例如
   随行情波动率变化、随品种切换变化），cascade 触发次数∝n 的经验拟合可能只是 CL 单标的巧合。本工位诊断
   限于 CL 前 300K/1M 同一连续窗口，**未做跨标的/跨时段验证**（231号 L2/L3 空缺，与 ws-enum2 报告同一
   边界条件）。
2. **codex 已裁定 cascade 必须是全量清空而非精确定位**（"option 2"，理由：局部 `frontier_mutated` 投影
   比对有损，可能漏判深层 `sub_moves` 改写）。若该裁决前提本身可被推翻（例如证明"受影响后缀"存在一个
   可靠的、非投影损耗的精确定位方法），则本工位"cascade 全量重建是 O(n²) 直接机制"的结论仍然成立，但
   "是否可安全优化为局部重建"的判断会翻转——本工位**未**重新审查该 codex 裁决的正确性论证，只诊断其
   当前实现的渐近代价。
3. **higher level（level≥2）贡献占比递减**：level=0 的 wiped 规模主导总量（300K：76879/104592=73.5%；
   1M：823361/1130430=72.8%），level≥3 贡献 <2%。若未来某个 level 的行为模式发生结构性变化（如 l_max
   配置改变引入更多级别），本表的"level=0 主导"结论需要重新测量，不能外推到未测量的更高级别配置。

## 下游推论

1. **task #65 原定二分假说需要修订，非"结算"而是"证伪并订正"**：任务描述的"(a) 结构性某类走势永不
   confirm / (b) 参数性阈值问题"框架本身建立在"prefix_count 稳态下推进过慢"的隐含前提上——本工位周期
   采样直接证伪此前提（稳态差值恒为1）。真根因是 cascade 重建机制，这是一个**第三类**（架构性：全量
   清空 vs 局部失效的实现选择），不落在任务给出的二分之内。
2. **是否可优化=一个独立的、有明确风险边界的新工位**：理论上，若能证明"cascade 触发时只需要清空
   `upper_moves` 中真正被 frontier 改写波及的**后缀**（而非全部），保留可证不变的前缀"，可以把 cascade
   重建成本从 O(当前历史总量) 降到 O(受影响窗口数)，从而消除 O(n²) 残余。但 codex 此前已裁决全量清空
   是必要的保守选择（本工位边界条件2），要推翻这个裁决需要**新的正确性论证**（证明存在一个不依赖有损
   投影比对的、可靠的"受影响后缀"定位方法）——这是一项独立的、高风险的形式化任务，不是本工位（纯诊断）
   文件域内可以顺手做的（275号局部依赖：诊断工位不越界接入修复）。
3. **对 07b/A3/09 等所有受累阶段的归因需要统一**：ws-enum2 报告只诊断了 07b（`extract_second_resume`）
   的残余 O(n²)；本工位坐实同一 cascade 机制**同时**驱动 05_compose_resume（2025ms@1M）、
   09_project_to_units_resume（4749ms@1M，本次 profile dump 中占比最大的单一阶段）——这些阶段的残余
   非线性大概率同源，不需要分别诊断（同一 cascade 事件同时触发这几个阶段的全量重算）。
4. **信号集完全不变**（本工位零代码改动，仅诊断插桩加入又移除）：不影响 W-VERIFY(#13) alpha 有效性
   判定。

## 谱系引用

- ws-enum2（`enum2-gating-20260702.md`，task #60）：诊断"frontier tail 宽度线性增长"现象并否证候选枚举
  假说，把归因移交到 mod.rs 主循环（本工位承接对象）——本工位**进一步否证**了 ws-enum2 报告下游推论2中
  隐含的"prefix_count 推进速率"框架本身，把根因精确定位到 cascade reset 全量重建机制。
- 07b（`07b-frontier-gating-20260702.md`，task #23）：门控消除 confirmed 前缀重扫 O(U²)，本工位坐实
  cascade 事件是门控消除范围之外的、独立的 O(n²) 驱动源（门控假设"confirmed 前缀不变"，但 cascade 恰恰
  周期性打破这个假设）。
- 231号：形式化有效域规则——本工位是该规则在 07b/tail-width 问题域内的**第二实例**（继 ws-enum2 之后）：
  task #65 假说的"定义域"（prefix_count 推进速率二分：结构性/参数性）与真实"有效域"（cascade 全量重建的
  稀疏但规模随 n 增长的尖峰）不重合。
- 090号：严格性——不因任务标题预设了"(a)/(b)二分"就削足适履地把发现塞进这个框架；真根因是二分之外的
  第三种机制，诚实记录并订正任务前提。
- 275号：局部依赖——本工位（诊断）不越界提出/实施对 codex 已裁决的 cascade 全量清空策略的修改，将"是否
  可安全局部化"列为独立后续工位的开放问题。

## 影响声明

**零代码改动持久化**。本工位在 `classify_with_tower_incremental` 内临时插入 `DIAG_TAILWIDTH` 环境变量
门控的诊断探针（周期采样 + cascade 事件全量记录），运行两次 profile 跑批（CL 300K/1M）后**完整移除**
（`git diff --stat rust/src/theta_v0/classifier/mod.rs` 显示 0 insertions/0 deletions，`git status`
干净）。`cargo test --release --lib` 1396 passed / 0 failed，确认移除后状态与协作者上次提交（enum2-gating/
task #60）完全一致，无回归。task #65 判定为 **NO-SHIP**（本工位范围内不实施任何代码修改）：真根因（cascade
reset 全量重建 = 触发次数∝n × 单次规模∝n 两个线性因子相乘）已定位并有强证据支撑（level=0 max
wiped_upper_len 与 ws-enum2 报告的 max tail width 逐位相同：763/2532），是否可安全优化为局部失效需要
一个独立的、直面 codex 既有裁决的新工位（超出本工位诊断范围）。

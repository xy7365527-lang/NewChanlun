# 引擎 C 段边界修复：B2（C 段越界极值）+ 修复A（递归层 seg_end 对齐 level-1）

> 实施对象：`analysis/engine_bsp_gap_diagnosis.md` 诊断的两个结构缺陷（编排者已裁决 B2）。
> 数据：OKLO 447,739 bar（1min, databento）终态对照 + 三标的回测重跑。
> 认识论等级：**L2**（单标的真实数据终态对照 + 合成差分测试守卫）；
> 回测影响部分为 L2/L3（三标的）。

---

## 0. 结论（TL;DR）

C 段（趋势离开最后中枢到转折点的走势段）不再被 move 边界截断：

1. **B2（定义级）**：趋势背驰 C 段终点 = 搜索窗口 `[zs_last.seg_end+1, 下一 settled
   中枢 seg_end]`（无后继 → n-1）内的**趋势极值段**（down→最低 low，up→最高 high，
   平值取首个）。背驰段终于走势转折点（第24课），move 边界是工程分区不是概念边界。
2. **修复A（代码级）**：递归层 `moves_from_level_zhongshus` 的 seg_end 语义与
   level-1 对齐——非末组 = 下一组首中枢 comp_start−1，末组 = num_components−1。
   消除同一概念（走势边界）在两层的不一致。

两者 Rust 与 Python oracle 同步修改，笔/线段/中枢层逐位不变（md5 验证）。

### OKLO 447K 终态解锁效果（修复前 → 修复后）

| 层 | confirmed type1 | confirmed type2 | 背驰总数 |
|----|----------------|----------------|---------|
| ladder2（笔中枢） | 1（0 buy + 1 sell）→ **374**（187 buy + 187 sell） | 0 → **360**（182 buy + 178 sell） | 1 → 385 |
| ladder3（走势级） | 2 → **60**（31 buy + 29 sell） | 2 → **58** | —（经 BSP 计） |
| ladder4（递归） | 0 → **8**（**5 buy** + 3 sell） | 0 → **8** | 0 → 8 |
| ladder5（递归） | 0 → **1**（1 sell） | 0 → 1 | 0 → 1 |

- ladder2 数字与诊断探针 Sim2-B2 的预测（373 confirmed type1 / 360 type2）一致
  （374 vs 373：引擎全链与探针终态近似的 1 个边界差）。
- **趋势背驰在递归层（ladder≥4）首次存在**——type1/type2 的空定义域被打开，
  且 buy 侧非空（诊断终态模拟只见 sell 侧 2 个；引擎 A+B2 联合后 L4 有 5 个
  confirmed type1 buy）。
- type3 计数逐 ladder 完全不变（type3 不消费背驰，旁证改动面收敛）。
- 级别涌现不变：L4 中枢 47 / 走势 15、L5 中枢 3 / 走势 1，与修复前一致
  （`should_stop_recursion`/persistence 只读 zs_start/zs_end + 中枢价格，L0 论证 +
  实测双确认）。

---

## 1. 改动清单

### Rust（`cargo test` 74/74 通过）

| 文件 | 改动 |
|------|------|
| `rust/src/divergence.rs` | `detect_trend_divergence` 实施 B2（`next_settled_zs_seg_end` + `trend_extreme_seg` 首极值 tie-break）；新增 `detect_dep_end`（增量器 commit 判据：背驰检测的最大段依赖）；`IncrementalDivergences::stable_len()` |
| `rust/src/level.rs` | `moves_from_level_zhongshus(zhongshus, num_components)` 修复A；`group_to_move` 增加 next_seg_start/num_components 扩展（与 level-1 `moves.rs` 逐字一致）；锚点 first_seg_s0/last_seg_s1 不扩展 |
| `rust/src/orchestrator.rs` | `LevelEngine::process` 传 `Some(comps.len())`；`update_bsps_incremental` 传 divs_stable_len + volatile_floor |
| `rust/src/segment_layers.rs` | `IncrementalSegDivergences` commit 判据 `mv.seg_end < anchor` → `detect_dep_end < anchor`（B2 窗口可越出固定区）；`IncrementalSegBsp` div frontier 改单调推进（B2 后 `div.seg_c_end` 跨 move 非全局单调，不可二分）+ anchor 门控 `volatile_floor`（易变 div 的 C 段极值可远落于 n−SAFE_W 之前，不可提前 finalize） |
| `rust/src/buysellpoint.rs` | `IncrementalBsp` 同款 frontier/门控修正；**顺带修复潜伏 bug**：tail_type1 边界原为 src_from（与 stable_t1 重叠双重供源 → type2 重复）——B2 前因 closed move 的 type1 结构性不存在而不可达，B2 解锁后由 `diff_lcg_medium` 差分测试当场暴露，按 `IncrementalSegBsp` bar 350200 修复确立的 stable_anchor 不相交边界修正 |
| `rust/src/bi_zhongshu_bsp.rs` | 接线 divs_stable_len |

### Python oracle（与 Rust 逐位一致，合成 12000 bar 逐 bar 六层等价测试通过）

| 文件 | 改动 |
|------|------|
| `src/newchan/a_divergence_v1.py` | `_detect_trend_divergence` B2（`_next_settled_zs_seg_end` + `_trend_extreme_seg`） |
| `src/newchan/a_zhongshu_level.py` | `moves_from_level_zhongshus(zhongshus, num_components=)` 修复A |
| `src/newchan/core/recursion/recursive_level_engine.py` | 传 `num_components=len(components)` |
| `src/newchan/a_nested_divergence.py` | `_level_trend_ac_segments` B2 镜像（同一概念同一定义）；**区间套同向链定理**：`_drill_down_mid_levels`/`_finalize_with_level1` 增加方向过滤（链逐级定位同一转折点 ⟹ 各级背驰必然同向——此过滤在 C 段恒空时代不可达，B2 解锁次级背驰后成为必要） |

### 测试更新（定义变更对应的 golden 调整，testing-override 生成态例外）

- `tests/test_nested_divergence.py`：链方向断言从"全场景单 top"改为"链内同向 + top 链存在"
- `tests/test_divergence_bar_range.py`：fixture 价格补连续（原两 move 间 35→95 跳空，
  B2 窗口会把下一 move 的起点误判为本 move 极值——合成数据缺陷，非定义缺陷）
- `rust/src/level.rs` 单测：seg_end 期望更新 + 新增非末组扩展测试

---

## 2. 增量器架构影响（B2 的连带必然修正）

B2 打破了两个增量器赖以工作的前提，必须同步收紧（否则 bit-exact 契约破裂）：

1. **`div.seg_c_end` 不再跨 move 单调**：趋势 C 段可越入下一中枢覆盖区，与后继
   盘整背驰形成局部逆序 → `partition_point` 二分不可用，改为只在永久前缀内推进的
   单调 frontier（`divs_stable_len` 边界）。
2. **易变背驰的段依赖可远超其 move 边界**：pending/未 commit move 的 C 段极值可落在
   `n − SAFE_WINDOW` 之前 → BSP finalize anchor 增加 `volatile_floor` 上限
   （= 第一个未 commit move 的 seg_start；其全部背驰段引用 ≥ 该值，单调前进）。
   `IncrementalSegDivergences` 的 commit 判据同理从 `seg_end < anchor` 收紧为
   `detect_dep_end < anchor`。

守卫：`bi_zhongshu_bsp::differential_tests`（LCG 逐前缀 inc≡full）、
`segment_layers::incremental_tests`、`moves/zhongshu/orchestrator` 各差分测试全部通过；
合成 12000 bar Rust↔Python 逐 bar 六层 bit-exact 通过。

---

## 3. bit-exact 验证（修复约束：笔/线段/中枢层不变）

OKLO 447K 终态，修复前/后引擎全量重跑对比（`analysis/_c_seg_fix_capture.py`，
`analysis/data_cache/c_seg_fix_{before,after}.json`）：

| 层 | n | md5 相同 |
|----|---|---------|
| strokes（笔） | 29,695 | ✅ |
| segments（线段） | 3,217 | ✅ |
| zhongshus（线段中枢） | 585 | ✅ |
| zhongshus（笔中枢） | 3,566 | ✅ |
| moves（L1 走势，构造未改） | 191 | ✅ |
| 递归层中枢 L4/L5 | 47 / 3 | ✅（zs_md5 相同） |
| 递归层走势 L4/L5 | 15 / 1 | 计数与结构不变；seg_end 按修复A 扩展（moves_md5 变化 = 预期改动本身） |

pytest 回归：全套件 4,856 通过；37 个失败全部为既有环境性失败
（pytest-asyncio 缺失的 async 组、`.chanlun/blocks` 仓库状态类），与本修复无关
（其测试不导入任何被改模块）。

**未完成项（诚实声明）**：`test_bitexact_large_real_streaming`（BZ 真实全量、
@pytest.mark.slow、Python orchestrator 逐 bar）两次被系统 jetsam 杀进程
（exit 144，无断言失败输出），未跑完。同层覆盖由以下替代提供：合成 12000 bar
Python↔Rust 逐 bar 六层 bit-exact（含/不含 MACD 两模式）通过 + 三标的真实数据
（含 BRN 2.4M bar）organic_signals 差分守卫全程通过（Rust delta 接口 vs 布尔导出
逐位）+ OKLO 447K 前后捕获不变层 md5 一致。

---

## 4. 回测影响（三标的重跑，OKLO 447K / QQQ 728K / BRN 2.42M bar）

两条管线全部重跑并通过各自守卫（O0≡P5 逐笔逐 trace、磁带级回归锚先重生成
`interval_nesting_reverse_backtest.json` 后逐位复现、organic_signals 差分守卫
全程通过——含 BRN 2.4M bar）。BH 各标的不变（同一数据），漂移全部来自信号定义。

### 4.1 主轴信号全口径漂移（P5/B0 自身消费 buy1/sell1）

| 标的 | P5 复利（前→后） | 交易数（前→后） | B0（前→后） |
|------|----------------|----------------|------------|
| OKLO | +740.9% → **+1088.8%** | 220 → 195 | +290.3% → +492.9% |
| QQQ | +108.2% → +100.6% | 510 → 422 | +95.6% → +122.7% |
| BRN | +488.4% → **+28.3%** | 1329 → 910 | +356.9% → +70.4% |

type1 从 1 个涨到 374 个（L2）直接重排 master 进出场。**旧在册结论与新口径
不可同表比较**——所有引用修复前 P5/E/I 数字的结论需重新评估。

### 4.2 entry / tranche（诊断 §5 的预测兑现）

- **entry 上限 3 → 4**：OKLO 终态 ladder4 confirmed type1 buy 0 → 5。
- **tranche 区间 (2, entry−1] = (2,3]**：从空集变 **1 层**（非空定义域）。
- **REV T1 触发的 ladder 分布**（修复前→修复后）：
  修复前三标的全部 `{segment}` 单层；修复后 `move(L1)` 层三标的非空
  （OKLO 56 / QQQ 57 / BRN 381 次 T1 尝试），**BRN `recL2`（递归层）首次出现**
  （T1 尝试 6 / rev 开 9）——递归层反向操作定义域从 ∅ 打开。
- v2 Rust 交易层的 C4 tranche 递归建仓仍待 M2 信号层 D3 行（REV 变体 fail-fast
  状态不变）；本次解锁的是其前提（递归层 confirmed type1 非空）。

### 4.3 REV 判决变化（organic_fugue 预注册判据，Δ = vs_B0(X) − vs_B0(P5)）

| 判据 | 修复前 | 修复后 |
|------|--------|--------|
| 1. REV 假设 Δ(O1) | 全负（OKLO −626 / QQQ −80 / BRN −450）→ 否证，框架收缩 P5 | **混合**：OKLO −947 / QQQ −74 / **BRN +49**（O2：+60）——BRN 成为首个 REV 正域标的，否证从"全域"收缩为"2/3 标的" |
| 3. 规模假设 Δ(O3)>Δ(O2) | 2/3 且因 REV 否证降级 | **3/3 成立**（OKLO +110 / QQQ +5 / BRN +45 pp） |
| 4. earning | 触发率 0，无定义域 | 仍 0（OKLO O3 出现 1 笔 earning，触发率仍≈0） |
| 5. rev 腿独立质量 | OKLO rev avg_diff +0.135 净现金 +55639 | OKLO −0.129 / −35199（恶化）；QQQ ≈持平负；BRN −0.002 / −1675（腿数 2431→4265 翻近倍，净现金转负） |

### 4.4 REV 胜率：未提升（诚实结论）

| 标的 | rev 腿胜率（O2，前→后） | rev 腿数（前→后） |
|------|----------------------|------------------|
| OKLO | 50.9% → 48.4% | 407 → 670 |
| QQQ | 44.6% → 44.7% | 807 → 1104 |
| BRN | 47.3% → 46.2% | 2431 → 4265 |

"更精确的背驰信号（锚在趋势极值段）→ REV 胜率提升"的预期**未兑现**：
修复的实际效果是**定义域扩大**（可触发的 T1 翻近一倍、扩展到更高 ladder），
新增腿的质量摊薄了均值。背驰锚更准 ≠ 反向操作更赚——锚定准确性与
段尺度反向假设本身的有效性是两个独立命题（后者在 OKLO/QQQ 仍被否证）。
BRN（2.4M bar 期货，含长期下行/震荡 regime）转正提示 REV 有效域可能是
regime 依赖而非全域性质——与"强趋势满仓标的上反向操作必然摊薄"一致。

---

## 5. 结果包六要素

1. **结论**：B2 + 修复A 落地，趋势背驰 C 段以走势转折点（趋势极值段）为终点，
   递归层 move 边界与 level-1 统一。OKLO 447K：confirmed type1 1→374（L2）/
   2→60（L3）/ 0→8（L4）/ 0→1（L5）；type2 从全链 2 个到 0→360/58/8/1。
   笔/线段/中枢层逐位不变。
2. **定义依据**：第24课趋势背驰（C 段终于"走势类型完成时"=转折点，非中枢边界）；
   第17/21课 type2（检测代码零改动，经 B2 自然解锁）；第27课区间套（同向链定理）；
   level-1 move 边界语义（`moves.rs` next_seg_start−1 / num_segments−1）。
3. **边界条件**：①若改用 MACD 背驰路径（enable_macd），B2 的窗口/极值定义同样生效
   （C 段范围变化影响 macd_area 区间），但本次验证全部在价格振幅 fallback 路径
   （E/I/有机赋格均 enable_macd=False）；macd 路径仅有全量重算守卫，无增量器差分。
   ②B2 窗口上限取"下一 settled 中枢 seg_end"——若未来中枢定义改变（如允许设非
   首尾相接中枢），窗口语义需复核。③pending（末 move）窗口上限 = n−1，其背驰
   仍是流式瞬态（每 bar 重评估）——B2 不改变 pending 期 fire 语义，只改变锚位置
   （极值段而非末段）。④374/360 等数字依赖振幅力度与 0.9 确认阈值；换度量数字会变，
   但"0→数百"的量级跃迁来自 C 段从空变非空，对度量不敏感。
4. **下游推论**：①type1 锚（= div.seg_c_end = 趋势极值段）首次持久存在于 settled
   结构 → type2 检测、区间套递归定位、REV 腿的 t1 锚定全部获得非空输入；
   ②entry/tranche：L4 confirmed buy1 非空 → entry 上限 3→4，tranche 区间 (2,3]
   从空集变 1 层；③所有消费 buy1/type1/背驰流的在册回测口径漂移，需重评
   （本报告 §4 为首轮重评）；④`IncrementalBsp` 潜伏的 type2 双重供源 bug 已修——
   该 bug 在旧定义下不可达，任何依赖旧增量器的结果不受影响。
5. **谱系引用**：532（seg-end-trigger 轴分裂）相关结算见 `.chanlun/genealogy/`；
   005b（对象否定对象——move 边界语义反向污染背驰对象定义域的修复）；
   090（严格性：同一概念两处定义的统一）；521（PH settle 形态学边界不受影响）；
   区间套配对修复 P1-P7（bsp_events 级别隔离架构不变）。
   诊断原文：`analysis/engine_bsp_gap_diagnosis.md`。**不确定**是否已有编号谱系记录
   "C 段越界极值定义"的结算——本次为编排者直接裁决（任务卡），建议谱系工位补录。
6. **影响声明**：改动 Rust 引擎 6 文件 + Python 4 文件 + 测试 3 文件（见 §1 清单）；
   笔/线段/中枢/L1走势层输出逐位不变；背驰/BSP/递归 move seg_end 层按新定义漂移；
   `buysellpoints_from_level`/`divergences_from_moves_v1` 纯函数接口签名不变，
   `moves_from_level_zhongshus` 增加 num_components 参数（Rust 必填 Option /
   Python 关键字默认 None 与 level-1 num_segments 对称）；增量器
   `IncrementalBsp::update`/`IncrementalSegBsp::update` 签名变更（内部 API）。

# p117：BSP 修复施工图 S2/S4 —— 第一类路径 anchor 门降级（provenance 锚 → 结构方向）

日期：2026-07-17 ｜ 性质：**设计文档（施工图），生产源码零改动** ｜ worktree：`/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`）
依据判定链：`p112-trend-predicate-caseaudit-20260717.md` §0/§4/§5/§6-4（754 对归属与 S2/S4 归因）、
`doc-trend-divergence-predicate-20260717.md:156`（anchor 门 = 非教义附加门）、
`.chanlun/genealogy/settled/686-q7-fallback-direction-anchor-separation-narrows-121-decision-a.md`（Q7-#1 裁定C 原文与翻转条款）、
`p115-predicate-caliber-impl-20260717.md`（R1/R2/R3 实装面）。

## 0. 结论先行：R3 后这 4 案的状态 —— **未被救回，anchor 门修正设计必要**

**静态判定（git diff HEAD 实测，逐 hunk 核对）**：R1–R3 的改动 hunk 全部落在
① `level_view.rs`（D2 侧 seg_c 全离开段 / `trend_confirm_time` / nest Cons 分支）、
② `divergence.rs`（`same_color_area`/`same_dir_hist_peak`/`segments_diverge_or` 新增 + 测试）、
③ `signal.rs` 的 `trend_third_class_in_c`（新增）、`locate_pan_div_structure_front_anchor`（新增）、
`judge_pan_div`（R2/R3 调用序）、`judge_segment` 的 **kind_consol 分支**（仅加 `dif` 参数）。
**击杀这 4 案的第一类路径三消费点——`judge_first_cached:294` 锚门、`locate_departure_move_a`
（`judge_segment:1237` 调用）、`departure_move_c_start`（`judge_segment:1239` 调用）——零 hunk 覆盖。**
R3 的 A′ 锚扩发生在 **pan 域**（`signal.rs:676` → `judge_pan_div:756` 与 `level_view.rs:744-755` nest Cons 分支）；
S2/S4 是**趋势第一类 BSP 域**（`judge_segment:1231-1246` 第一分支）。两域代码零相交 ⟹ R3 在结构上不可能救回这 4 案。

**实证判定（p116 全量重放进行中 dump，`/tmp/p116_probe_dump.txt`，prefix pass 已越过前两案坐标）**：

- `DIV level=1 end=996247 kind=trend side=Long` 在 dump 中 ⟹ 案 1（turn=996216）在 R1 后 D2 侧仍确认，
  且确认时点 t*=996247 恰 = p112 实测 `t3_ext_hit=Some((996216, 996247))` 的回试段终点（R1 全合取口径的预测值，逐位吻合）。
- `DIV level=1 end=1024341 kind=trend side=Short` 在 dump 中 ⟹ 案 4（turn=1024293）同理确认于 t*=1024341
  （= p112 `t3_ext_hit=Some((1024293, 1024341))` 回试端）。
- 全部 4 案坐标 ±250–500 bar 窗内 **TERM 行 = 0**（dump 已有 81 条 TERM）⟹ BSP 终端 `confirm_side` 在这些坐标
  及其 R1 新 t* 上**仍然全灭**。案 2/3（turn=1943369 / 2230678）prefix pass 落笔时（1.5M/4.6M）尚未推进到，
  由静态判定覆盖（同一批未触函数、同一 S2 击杀标签），重放完成后按 §6 验证计划复核。

**结论**：4 案未被 R3（或 R1/R2）救回。本项不关闭，进入修法设计（§3）。本设计即 p112 §6-4
「BSP 非教义 anchor 资格门……若主人裁定降级/去除」裁定项的施工图；同时也是 686 号谱系**翻转条款第一支**
（「若领域定义正式授权『盘整 ownership 单元 endpoint 方向可作上级趋势方向锚』……本号订正方向翻转」）
的**窄域授权申请**——授权范围仅限第一类路径、仅限 τ 门已开、仅限单元行程方向 = τ（§2/§3）。

## 1. 病因（逐案坐标 + 代码行号 + p112 归因）

### 1.1 四案坐标（源：`/tmp/p112_full.txt` P112_CASE 行；`/tmp/p109_full2.txt` P109_CHAIN_DETAIL）

| # | turn | side/dir | seg_a（=b） | seg_c（=c，#105 单腿） | chains | t3_ext（立即三买） | area a→c | 门链首杀 | p112_full.txt 行 |
|---|---|---|---|---|---:|---|---|---|
| 1 | 996216 | Long/Down | (995646, 995953) | (996028, 996216) | 2 | (996216, 996247) | 105.80e9 → 55.89e9 | **S2** | :43 |
| 2 | 1943369 | Short/Up | (1942665, 1943178) | (1943295, 1943369) | 4 | (1943369, 1943417) | 408.76e9 → 144.38e9 | **S2** | :85 |
| 3 | 2230678 | Long/Down | (2229972, 2230371) | (2230539, 2230678) | 6 | (2230678, 2230806) | 693.83e9 → 184.36e9 | **S2** | :108 |
| 4 | 1024293 | Short/Up | (1023464, 1023932) | (1024114, 1024293) | 3 | (1024293, 1024341) | 87.35e9 → 26.66e9 | **S4** | :46 |

p109 链坐标：`P109_CHAIN_DETAIL id=311/312`（案 1）、`id=337/338/339`（案 4）——「L1 1 个背驰确认基例
全部无 confirm_side BSP，any_bsp_at_turn=false」。案 2 的 BSP 账本为 `points=1 confirm=false
any_bits=true third_same_side=false`（turn 上有**反向**点，p112 §4 账本实况三案之一），佐证坐标可读、
方向被锚门拦。四案教义全合取逐项成立（p112 §3 逐案列：T1=1 T2=1 T3_ext=1 T4 四口径全 1 T5=1）。

p112 归因引用：§0「S2 非教义 anchor 资格门 3（1.9%）＋ S4 anchor 筛选致 A 段不可配 1（0.6%）」；
§4 门链表 S2/S4 行；§5 代表样本（turn=996216「门链仅差 anchor 一环」、turn=1024293「同参数 D2 侧
anchors 全 Some 可配」）；§6-4 修法建议（合计 2.6%，「降级/去除」裁定项）。

### 1.2 击杀机制（生产代码 文件:行号）

**S2（3 案）——`judge_first_cached` 锚资格门**（`rust/src/theta_v0/classifier/signal.rs:294-300`）：

```rust
let (broke, is_sell) = match (anchor_dir, trend_dir) {
    (Some(Direction::Down), Direction::Down) if end.price < last_center.zd => (true, false),
    (Some(Direction::Up),   Direction::Up)   if last_center.zg < end.price => (true, true),
    _ => (false, false),   // anchor_dir=None（fallback 单元）⟹ broke 恒 false，几何不再评估
};
```

`anchor_dir` 来自 `judge_segment:1241` 传入的 `anchors[i]`；`anchors` = 级别-N 的 provenance 锚数组
`units_anchors[i] = center_own_dir_at(pb, i)`（`mod.rs:439` 全量 / `mod.rs:2060-2063` 增量，与
`project_to_units` 方向派生同一来源）。`center_own_dir_at`（`decompose.rs:180-188`）返回 `None` ⟺
i==0 或关系 R(i-1,i) 按 ownership 落 **Consolidation 块**（017:44 盘整无方向）。3 案的破中枢单元恰为
fallback（其 L0 走势落在 L0 盘整块 ownership），锚门在几何破核心评估前击杀——τ 门（S1）已开、
τ 方向与 D2 趋势方向一致（p112 探针 `bsp_gate_diag`，`p112_trend_predicate_caseaudit.rs:349-355`）。

**S4（1 案）——`locate_departure_move_a` 锚筛选致 A 段不可配**
（`rust/src/theta_v0/classifier/divergence.rs:767-789`）：

- `:778` `episode_start_in` 首同向段选取过滤 `**a == Some(dir)`（`divergence.rs:824`）；
- `:783` A 区间候选过滤 `**a == Some(trend_dir) && s.start_index >= lambda_a`。

案 4 的级别-1 A 窗（`[prev_center.end, last_center.end]`）内同向单元全 fallback ⟹ 两过滤均空 ⟹
`None` ⟹ `judge_first_cached:305-307`「无 A 候选 ⟹ A/C 无法配对」返回。同参数 D2 侧（L0 段全 Some
自锚）可配（p112 §5）——击杀变量只有锚 provenance。

**根因定性**：τ 门（级别-N decompose Trend 块门，`decompose.rs:159-169` 经 `judge_segment` 的
`gate_dir`）已在消费级保证「第一类只在趋势中判」（定义域）；锚门再叠加一层**次级别块 provenance**
资格——要求构成 A/C 的单元自身也是次级别趋势走势类型。doc-trend:156 判：「非教义附加门——BSP 多出
工程性收缩，无教义依据（Q7-#1 是 provenance 裁定，非教义）」。该门是 686 号裁定为堵 fallback 方向泄漏
而设（实装时实测旧基线 77–95% 为伪信号，§6 风险章正面处理），非 037/061 谓词组件。

## 2. 教义依据（直读主仓 `docs/chanlun/text/blog/` 原文核对，课号:段号（行号））

- **017:44 / 017:46**：「缠中说禅盘整：……只包含一个缠中说禅走势中枢」「缠中说禅趋势：……至少包含
  两个以上依次同向的缠中说禅走势中枢」。→ 趋势/盘整定义域的对象是**走势类型**。
- **027:14**：「趋势，一定有至少两个同级别中枢，对于背驰来说……肯定是至少是第二个中枢之后」。
  → 第一类定义域由「≥2 同级别中枢的趋势」给出；生产 τ 门（级别-N decompose Trend 块，块内 ≥2 中枢、
  pos > 块首）即此定义域的实装。4 案 τ 门全开（S1 通过）⟹ **定义域已满足**，锚门不是定义域的唯一承担者。
- **037:16**：「没有趋势，没有背驰……A、B 是同级别的中枢」。→ 趋势前提约束的是 A/B 中枢关系，不约束
  次级别单元的 provenance。
- **037:18**：「c 必然是次级别的，也就是说，c 至少包含对 B 的一个第三类买卖点」。→ 对 c 的结构要求
  落在 c 整体（含三买），p112 T3_ext 实测 4 案全成立。
- **037:20**：「如果 a+A+b+B+c 是上涨，c 一定要创出新高；……是下跌，c 一定要创出新低」。
  → **c 的方向性以价格几何表达**（新高/新低），方向由趋势形式给定（上涨趋势的 c 向上），不是对 c 的
  次次级别构成单元另设趋势资格。本设计保留「单元行程方向 = τ」检查（§3），即此条的单元级对应。
- **037:22**：「由于 c 包含 B 的第三类买卖点，则 c 至少包含两个次级别中枢」。→ c 是次级别**趋势**
  （≥2 次级别中枢）是对 c 整体的要求；c 的**末段单元**自身是次级别盘整走势不与本条冲突
  （盘整走势作 c 的一段不消解 c 含两次级别中枢的事实）。
- **024:24**：「A、B、C 段在一个大的趋势里，其中 A 之前已经有一个中枢，而 B 是这个大趋势的另一个中枢」。
  → A/B/C 框架的比较段由中枢位置关系定义，无 provenance 条件。
- **061:28**：「只要是围绕一中枢的两段走势都可以比较力度」。（娇注同段：盘背比较先最近同向一段再中枢
  两头。）→ 力度比较的两段 = 围绕中枢 B 的两段走势（进入段 b / 离开段 c），位置语义；未要求比较段的
  构成单元具备趋势 provenance。

**综合**：037/061/024 对围绕 B 的两段（A 段/C 段）的全部要求 = 位置（进/出 B）+ 行程方向（037:20
新高/低几何）+ c 内部结构（037:18/22，c 整体口径）。「构成 A/C 的级别-N 单元自身是次级别趋势走势类型」
**不在任何一条教义要求中**——它是 686 的工程资格（provenance 裁定，doc-trend:156 定性一致）。
定义域「没有趋势，没有背驰」（037:16、017:46、027:14）由消费级 τ 门承担且在 4 案已成立。
fallback 单元的 `fold_direction`（`recursive_tower.rs:208-221`：与 `classify_relation` 同源的外缘 hi
比较）是**几何行程事实**——在「τ 门开 ∧ 单元行程方向 = τ ∧ 端点破核心」合取下用作方向锚，
不把该单元当作趋势方向**来源**（τ 才是），不违反 017:44/第31课「盘整无方向」的定义域语义。
此即 686 翻转条款第一支所指的「领域定义正式授权」的窄域形态。

## 3. 修法设计（精确到函数/hunk；涉及文件清单）

### 3.0 唯一语义变更

**`judge_segment` 第一类分支的方向锚来源：provenance 锚（`anchors`）→ 单元结构方向（`anchors_self`）。**
「降级非去除」：方向匹配检查整体保留（单元行程方向须 = τ，037:20 单元级对应；端点破核心几何保留；
A 段同向筛选保留），仅更换方向**来源**。判据函数体（`judge_first_cached`/`locate_departure_move_a`/
`departure_move_c_start`/`episode_start_in`）**零改动**——它们消费传入的锚切片，降级发生在调用点。

### 3.1 hunk 方案（`rust/src/theta_v0/classifier/signal.rs`，`judge_segment:1231-1246` 第一分支）

```diff
     if let Some((pos, dir)) = gate_dir {
         let prev_center = &centers_sorted[pos - 1];
+        // ★p117（686 翻转条款第一支窄域授权，p112 §6-4 裁定项）：第一类路径方向锚降级——
+        // provenance 锚（anchors，次级别块 ownership 资格）→ 单元结构方向（anchors_self）。
+        // 定义域「趋势中」（037:16/027:14）由本分支 τ 门（gate_dir，级别-N decompose Trend 块）
+        // 已承担；行程方向=τ（037:20）与破核心几何在判据函数内保留。三消费点同源切换（禁部分修：
+        // broke 门/λ_C/A 段是同一「第一类方向锚」语义，部分切换会使 judge_first_cached:310
+        // 「broke ⟹ λ_C 必 Some」契约出现模糊地带）。三类分支（下方 anchors[i-1]）不动——
+        // 本案集三类零证据，686 对三类的保护整体保留。
         let a_seg_entry = *a_seg_cache
             .entry(c_idx)
-            .or_insert_with(|| locate_departure_move_a(sorted, anchors, prev_center, c, dir));
-        let c_start_entry = departure_move_c_start(sorted, anchors, c, dir, seg.start_index);
+            .or_insert_with(|| locate_departure_move_a(sorted, anchors_self, prev_center, c, dir));
+        let c_start_entry = departure_move_c_start(sorted, anchors_self, c, dir, seg.start_index);
         if let Some(pf) = judge_first_cached(
-            c, dir, seg, anchors[i], hist, dif, closes_tick, close_src, a_seg_entry, c_start_entry,
+            c, dir, seg, anchors_self[i], hist, dif, closes_tick, close_src, a_seg_entry, c_start_entry,
             gauge,
        ) {
```

签名零改（`judge_segment:1216-1217` 本已同时持有 `anchors`/`anchors_self`；`anchors` 仍服务三类分支
`:1264`）。`a_seg_cache` 缓存键（c_idx）语义不变（同函数同键，仅实参换源）。

### 3.2 同步项（同语义镜像，675 号 meta-rule：探针/漏斗与生产同路径）

- `signal.rs` `type1_funnel_dx`（`#[cfg(test)]`，`:1359` broke 匹配 / `:1371` `locate_departure_move_a` /
  `:1375` `departure_move_c_start`）：三处 `anchors` → `&anchors_owned`，与生产锁步；
  `:1402-1403` parity 断言不变（自动成为回归）。
- `mod.rs:2634` 既有测试 `q7_ruling_c_fallback_unit_not_direction_anchor_but_stays_member`
  **更新**（语义更新的必然结果，同 p115 既有夹具更新先例）：第一类两断言翻转——
  `:2655`（A 单元 fallback）0 → **1**、`:2658`（C 单元 fallback）0 → **1**（该夹具 C 单元
  direction=Down=τ，救回路径与生产一致）；三类断言 `:2668`（leave fallback ⟹ buy3=0）**保留**；
  成员身份对照 `:2653`/`:2671`（全锚 ⟹ 1）**保留**。测试改名/注释指向新裁定
  （建议 `q7_ruling_c_first_class_structural_direction_third_class_provenance_kept`），
  断言文本把「不得作方向锚」改写为窄域授权后的精确语义（第一类：结构方向锚；三类：provenance 锚保留）。

### 3.3 注释扫尾（声明与实际能力一致，禁旧注描述旧行为）

- `signal.rs:292-293`（`judge_first_cached` 内 Q7-#1 注）：补「第一类调用点经 p117 窄域授权传结构方向锚；
  直接调用方传 provenance 锚时裁定C 行为保留（判据函数语义未动）」。
- `signal.rs:919`、`:923-927`（`extract_signals_with_hist_anchored` 头注 Q7-#1 段）：补窄域授权指向，
  并注明「本函数第一类路径经 `judge_segment` 消费 `anchors_self`，`anchors` 仅余三类用途」。
- `divergence.rs:779`、`:805-807`（`locate_departure_move_a`/`episode_start_in` 的 Q7-#1 注）：
  补「生产第一类调用方（p117 后）传结构方向锚（self），本过滤退化为结构同向筛选；provenance 锚
  消费方仅余探针仪器」。**函数体零改**。
- `mod.rs:261-263`（`extract_first_third_for_level` 头注）：补「第一类方向锚经 p117 降级；
  `anchors` 实参仍须传（三类 leave 锚消费）」。

### 3.4 显式不做（禁补丁、禁越界）

- **三类 leave 锚门不动**（`judge_segment:1264` 仍传 `anchors[i - 1]`；`recursive_tower.rs:632/:1787`
  cp 召回审计同）——本案集三类零证据，686 对三类的保护整体保留。
- **pan 域不动**（`judge_pan_div` 早已用 `anchors_self`，`signal.rs:1251`——本切换后第一类与 pan 在
  方向来源上对齐，反倒是语义统一）。
- **不删 `anchors` 参数/数组构造**（`mod.rs:439/:2060`）：三类与审计仍在消费；删除属越界清理。
- **拒绝备选 (ii)「τ 继承纯几何」**（`anchor_dir = anchors[i].or(Some(dir))`，broke 退化为纯几何）：
  会接受行程反向但终点越界的单元（如 fold=Up 而 end < zd 的下方反弹单元），违反 037:20 行程方向要求
  ⟹ 误放。方案 (i)（结构方向）是本设计的唯一推荐。
- **拒绝部分修**（只改 broke 门、不改 λ_C/A 段）：`judge_first_cached:309-312` 的
  「broke ⟹ λ_C 必 Some」契约要求三消费点同一方向语义，部分修制造 debug_assert 模糊地带（090 禁）。

### 3.5 涉及文件清单（实装面）

| 文件 | 改动 | 性质 |
|---|---|---|
| `rust/src/theta_v0/classifier/signal.rs` | `judge_segment` 3 行实参 + 注释；`type1_funnel_dx` 3 行实参；tests 新增（§5） | 生产 3 行 + 测试 |
| `rust/src/theta_v0/classifier/mod.rs` | `q7_ruling_c_...` 测试更新（两断言翻转 + 改名）；`:261-263` 注释 | **测试/注释 only，生产零改** |
| `rust/src/theta_v0/classifier/divergence.rs` | `:779`、`:805-807` 注释 | **注释 only，函数体零改** |

`level_view.rs` / `recursive_tower.rs` / `decompose.rs` / parser / `Cargo.toml`：**零触碰**。

## 4. bit-exact 冲突面（触及对象清单 + 是否碰主干）

**主干触碰 = 无。** 本修全部落在 BSP **判定层**（centers/units/anchors 的消费侧），不触及：

- 笔/线段/inclusion（parser 与 L0 段账本）——零文件交集；
- 中枢检测（`detect_centers_*`）、走势分解（`decompose.rs`）、单元投影（`project_to_units{,_resume}`）、
  塔 compose（`recursive_tower.rs` 生产路径）——零文件交集；
- 级别构造数据流：`BspPoint`/`PanDivCert` 是 `LevelState` 的**叶子产物**——下一级输入 =
  `project_to_units(upper_moves, moves)`（`mod.rs:437`/`mod.rs:2057`），不读 bsp；塔/中枢/单元对
  本修的输出无回喂 ⟹ **塔/笔/线段/中枢路径 diff=0 由构造保证**（验收口径的第一维直接满足）。

**同构建内 bit-exact（full ≡ resume）保持**：

- full（`extract_signals_with_hist_anchored`）与 resume（`extract_first_third_resume`）共享同一
  `judge_segment`（`signal.rs:1196-1202` 头注契约），切换在共享函数内一处完成 ⟹ 两路径同步变更；
  07a parity 对拍（`signal.rs:1186-1194` debug_assert）与 `mod.rs:1400` 投影神谕不受影响（投影未动）。
- P116_BIT_EXACT 的 `moves_centers_bsp_pan_diff` 是**同构建内** full-vs-resume 对拍（非新旧构建对比），
  保持 0；`lifecycle_cp_ownership=1` 为基线既有 frontier 工件（p115 §5.0 基线副本逐字复现），与本修无关。

**新旧构建差异（本修的目的，非违例）**：级别≥1 第一类 BSP 账本**单调加宽**——
- L0：`extract` 传 `anchor_dirs=None` ⟹ `anchors ≡ anchors_self`（`signal.rs:961-962/:1116-1117`），
  前后逐位恒等（686 豁免路径同型，既有 L0 测试即回归锁）；
- 级别≥1 无 fallback 单元的链：`anchors == anchors_self`（`project_to_units:902-903` 同一 unwrap 源），
  逐位恒等；
- 差异面 = 含 fallback 单元且进入第一类评估的判定，**可枚举**（实测由 §6 验证计划报告）。
  声明：`judge_first_cached:310` 的「broke ⟹ λ_C 必 Some」契约在新口径下仍成立——broke 通过要求
  `seg.direction == τ` ⟹ seg 自身在结构同向过滤集内；`start ≥ boundary` 关系与锚来源无关（几何）。

## 5. 单测计划（测试名 + 断言 + 数据构造；全部合成夹具，零数据依赖）

夹具复用 `signal.rs` tests 既有 `dc(zd,zg,dd,gg,ei)`/`seg(dir,si,ei,sp,ep)`/`closes_seq` 与
`first_buy_extracted_with_trend_divergence`（`signal.rs:1569`）的 A/B/C 结构（C0[300,400]→C1[100,200]
下跌趋势；A=[3,5] Down 急跌；B=[5,7] Up；C=[9,11] Down 缓动破 zd=100，手算面积 A=23.26 > C=9.09）。
级别-N 输入经 `extract_signals_with_hist_anchored(centers, segs, Some(anchor_dirs), ...)` 显式锚数组模拟。

| # | 测试名（signal.rs tests） | 数据构造要点 | 断言 |
|---|---|---|---|
| 1 | `first_buy_fallback_c_unit_rescued_via_structural_direction` | 上夹具 + `anchor_dirs=[Some(Down),Some(Up),None]`（C 单元 fallback，结构方向 Down=τ） | `buy1.len()==1`；`source_index==11`；`pivot_low==80`；`center.is_none()`（**S2 救回型**） |
| 2 | `first_buy_fallback_c_unit_opposite_travel_still_rejected` | C 单元改 `seg(Up, 9, 11, 60, 80)`（结构方向 Up ≠ τ=Down，端点 80 < zd=100 仍越界）+ anchor None | `buy1.len()==0` 且无 struct_break 候选 @11（037:20 行程方向保留；锁死 τ-继承式误放，方案 (ii) 的负例锁） |
| 3 | `first_buy_fallback_a_window_pairable_via_structural_direction` | 上夹具 + `anchor_dirs=[None,Some(Up),Some(Down)]`（A 单元 fallback） | `buy1.len()==1`（**S4 救回型**：A 窗结构同向筛选配对成功） |
| 4 | `first_buy_anchored_set_bit_identical_after_downgrade` | 上夹具跑两遍：`anchor_dirs=Some([Some(Down),Some(Up),Some(Down)])` vs `None`（自锚） | 两跑 `Vec<BspPoint>` 逐字段相等（单调加宽、零误杀的单元级锁） |
| 5 | `judge_first_cached_provenance_gate_preserved_for_direct_callers` | 直调 `judge_first_cached(c1, Down, c_seg, None, ...)`（provenance None 实参） | 返回 `None`（判据函数语义未动；降级在调用点的契约锁） |
| 6 | `first_buy_fallback_episode_lambda_c_consistent` | 夹具 1 基础上于 C 单元前插入回中枢段（B 段端点重回 [zd,zg] 内侧）使 λ_C 落在 B 段之后 | `buy1.len()==1` 且 `seg_c.0 == λ_C`（经 `force`/候选区间断言 λ_C 取结构同向 episode 起点——三消费点同源一致性锁） |

既有测试更新（`mod.rs`）：`q7_ruling_c_...` 按 §3.2 翻转两断言并改名；三类/成员身份断言保留。
回归基线：`cargo test --lib` 全绿（基线 1647 + 新增 6）；`type1_funnel_dx` parity 断言（`:1402-1403`）
自动覆盖 funnel 镜像正确性；实装者须核 funnel 现有测试的 anchor 覆盖（若仅 L0 自锚，L0 恒等已锁，
可不加例——如实记录在实装报告）。
禁前视：`anchors_self` 是同 bar 已确认结构数据（与 `anchors` 同坐标系同可得性），不引入任何未来信息。

## 6. 风险与边界（误放/误杀评估）

**误杀 = 0（单调加宽，可证明）**：现产第一类点要求 `anchors[i]==Some(τ)`；而 anchor=Some(d) ⟹
`unit.direction==d`（`project_to_units:902-903` 与锚同一 `center_own_dir_at` 源，fallback 才走
`fold_direction`）⟹ 结构方向 = τ，新口径下同过。故无任何现产第一类点因本修消失。

**误放（正面，686 数据在中央）**：686 实装锚门时实测旧基线 L1 349→18 / L2 102→14 / L3 31→7
（一类总计 1057→614，「旧基线 77–95% 是 fallback 方向泄漏伪信号」）——该池是本修的**理论量级参照**
（2026-07-03 旧代码态、含面积过滤后的 bit 计数；门链其后多轮演进，**非本修产量承诺**）。本修相对
686 前状态的收窄防线有三：
1. **行程方向匹配保留**（方案 (i)）：结构方向 ≠ τ 的单元仍拒（§5 测试 2 锁）——686 时代「不论来源
   直接消费」的全开口被收窄为「行程方向必须与消费级 τ 一致」；
2. **力度过滤保留**：buy1/sell1 bit 仍需面积 C<A（`AbcDivergence::diverges`）；未过者按 P2-R2 仅以
   零 bit `struct_break` 候选进样本——**声明：Cand 样本池（struct_break_dir=Some）会变宽**（样本层，
   非信号层，下游 χ² 可否证）；
3. **终端口径收窄**：754 链语境下的有效增量 = 新增 `confirm_side` 终端事件，须经身份门/终端门与
   D2 t* 坐标匹配——以 p116 重放 TERM 集 diff 实测为准，而非 raw BSP 计数。

**实装后验证计划（裁定材料，逐项如实报告）**：
① 重跑 p112 探针（只读复用，单遍 ~3.4s）：4 案门链重诊断——预期 S2/S4 不再是首杀；
如实报告新首杀位置（S3/S5 或 S6）。**诚实条件**：S2 3 案救回以「C 单元结构方向 = τ」为前提——
现有 dump 不含级别-1 单元外缘，无法离线判定（几何强倾向成立：单元终点越 τ 向边沿；高瘦单元
反例存在，不预设）；S4 案救回高置信（A 窗 ∃ 结构同向单元由 τ 关系方向与中枢形成保证；episode
边界边缘情形已在 §1.2 分析）。② p116 全量重放：TERM 集 vs 修前 diff——新增终端事件逐案过
p112 式 T1–T5 全合取探针，报告 误放率（合取不成立者占比）与 4 案救回实况；P116_BIT_EXACT 五维
diff=0 复核。③ `cargo test --lib` 全绿 + §5 新增 6 测试。

**边界**：
- 三类 anchor 门不动（本案集零证据）；pan 域早已结构方向（本修反而统一两条路径的方向来源语义）。
- S5（级别-1 A/C 区间面积 C<A）对 4 案是**新测量**——D2 侧 T5=1（L0 坐标区间）不蕴涵级别-1
  单元区间同号；p112 §4 的 S5=0 是「到达 S5 者」的统计，不含这 4 案。
- 与 686 的冲突如实声明：本修 = 686 翻转条款第一支的**窄域授权申请**（限第一类路径、τ 门开、
  行程方向 = τ）；不主张 `fold_direction` 与 ownership 方向全局等价（翻转条款第二支不使用）。
  **若主人不授权，本项关闭为「裁定维持 Q7-#1，4 案 = 裁定内代价」，不实装、不补丁。**
- 不声明任何择时 alpha；本设计只修谓词口径与教义的一致性，产量后果以重放实测为准。

## 7. 与其他三项的潜在文件冲突（本项触及：signal.rs 生产 3 行 + mod.rs 测试 + divergence.rs 注释）

| 关②其他项 | 预期触及 | 与本项冲突 |
|---|---|---|
| S0 取段/坐标错位修复（p112 §6-5，身份桥/含 turn 单元区间匹配） | nest/终端匹配层（`level_view.rs` 或新文件） | **无文件交集**；语义互补（S0 修坐标可达性，本项修可达后的锚门） |
| S1a τ 门上下文修复（p112 §6-6，归属中枢落盘整块） | `decompose.rs`（块分类/门语义） | **无文件交集**；**语义耦合**：S1a 改变 `gate_dir` 开闭 ⟹ 第一类评估入口变化 ⟹ 两项联合产量不可线性叠加，建议串行实装后**统一一次 p116 重放**对账 |
| BSP 缺 037:20 破极值项（p112 §6-3，broke 升级为破 b 包络极值） | `judge_first_cached` 判据体（`signal.rs:294-303`） | **同文件（signal.rs）相邻函数**：彼改判据体、本改调用点实参，hunk 不重叠；但两侧注释扫尾都碰 `judge_first_cached` 头注区（:265-300）⟹ **建议串行**，本项先行（diff 小、语义独立），彼项在其后重写头注时合并 |

四项共同小冲突：`signal.rs`/`mod.rs` tests mod 的追加区（append-only，git 可机械合）。
实装顺序建议（供编排者）：本项（S2/S4）→ 037:20 破极值项 → S1a → S0，每步 `cargo test --lib` 全绿 +
末步统一 p116 全量重放（避免四次全量）。

## 附：纪律与证据声明

- 本轮生产源码零改动；唯一新写文件 = 本文档。主仓 `/Users/silencehan/Projects/NewChanlun` 零写入
  （教义原文只读直引：`037-第37课.md:16/18/20/22`、`017-第17课.md:44/46`、`027-第27课.md:14`、
  `024-第24课.md:24`、`061-第61课.md:28`）。无 git mutation；未改 `Cargo.toml`；未 cargo build/test
  （p116 后台重放不受干扰——本文全部实证只读 `/tmp/p116_probe_dump.txt` 进行中方言，案 2/3 坐标
  prefix 未达，已按静态判定覆盖并列入 §6 复核项）。
- v3 硬禁令遵守：全案无概率/统计推断、无回测验证策略、无 EMH 假设；全部判据 = 教义全合取的机械检查。
- 声明与能力一致：S2 3 案的 fold=τ 前提、4 案的 S3/S5 通过性均为**实装后测量项**（本文不预设通过）；
  686 冲突与翻转条款路径已如实标注，最终实装以主人对窄域授权的裁定为准。

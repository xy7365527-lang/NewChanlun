# p117：BSP 侧修复施工图 S1a——τ 门上下文（37 案 = 24.0%）

日期：2026-07-17 ｜ 性质：**设计工位·只读**（生产源码零改动、零数据重放、主仓零写入、零 git mutation、零 cargo build/test——p116 重放在途，不干扰）
工位：主线阶段 1 关②（BSP 侧口径修复）S1a 分项设计。实装由编排者随后串行派发；本文是唯一产出。
证据链：`p112-trend-predicate-caseaudit-20260717.md`（754 归因、S1a 定义与逐案坐标，下称 p112）、`p109-chain157-gate-attrition-20260717.md:21,119-129`（754 计数与终端门定义）、`doc-trend-divergence-predicate-20260717.md`（教义判定书，下称 doc-trend）、`p115-predicate-caliber-impl-20260717.md`（R1–R3 已实装口径）、`/tmp/p112_full.txt`（154 行 P112_CASE 逐案明细）。
权威链（`AGENTS.md:3`）：博文 `docs/chanlun/text/blog/`（一级）> chan99（二级）。引用格式 `0XX:行号` = 该课 md 文件行号；全部引文已由本工位于 2026-07-18 直读主仓原文逐条核对（027:14/20/32/66/136、029:16/18、031:883、037:16/18/20/22、025:38/761、043:26、024:16/18、044:32、061:28）。
代码行号：以 worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`，含 p115 实装）当日只读核对为准。

---

## 0. 结论先行

1. **S1a 不是 τ 门的缺陷，τ 门在其定义域内判决正确。** 37 案的机械事实是：终端门要求 BSP level-1 账本在 D2@1 事件 turn 上有 confirm_side 一类点，而 level-1 一类判据的 τ 门输入（`gate1[c_idx]`，L1 中枢链 decompose）为 None——归属 L1 中枢的入边关系是 LevelExpansion（盘整块）。门没有错：L1 链在该坐标**确实**处于盘整块。错的是**提问的级别**。
2. **τ 门缺的「上下文」精确答案**：D2@1 事件的趋势对象是 **L0 中枢链**（nest level-1 投影种子 = tower[1] 窗携带核 = L0 中枢；其 decompose 块 ≡ `levels[0].moves` 同关系标签），而 BSP level-1 τ 门的对象是 **L1 中枢链**（`levels[1].centers` = L0 中枢三元组上的几何检测链）。两者差**一整级**。这个上下文不在 L1 分解的任何参数（阈值/窗口）之内——它是另一条链上的对象，门内无可补。p112 §4 的归因「全局 decompose vs per-run 块上下文差异」经本工位证明**不成立为机制**：per-run 与全局分解在被消费中枢上的门值恒等（附录 A 证明）；真正差异是**链的级别**（L0 中枢链 vs L1 中枢链）。
3. **三族修法评估**：放宽阈值＝拒绝（无教义依据，误放洪水——盘整块破中枢+面积衰减将全成假 B1/S1）；补上下文参数（门内注 L0 上下文）＝拒绝（等价于在 L1 循环里重实装 levels[0] 且级别冒充，触 BSP 生成主干，须升级裁定；其诚实形态即选项 C）；**改判据结构＝推荐，但改在终端背书而非 τ 门**——背书查法从 `levels[ℓ].bsp` 移到事件结构级别 `levels[ℓ-1].bsp`（教义：037:16 趋势级别=中枢级别、024:18 背驰必制造某级别买卖点、043:26 背驰级别≤走势级别 ⟹ 查找只能向下）。
4. **施工图（选项 C）**：H1 新 lib 查法 `nest::terminal_bits_at_event`（两口径 C-a 精确坐标 / C-b c 窗口最早 confirm_side 点，口径为裁定项）；H2 p92 bin `terminal_bits_new` 委托；H3 nest.rs 终端契约文档；H4 只读核验探针（新 bin，37 案逐案验 L0 门链）；H5 单测 4 新增 + 全量回归。**τ 门/塔/笔/线段/inclusion/中枢/BSP 生成主干零触碰**；P92_BIT_EXACT 五维（old_path/tower/moves/centers/bsp/pan）构造上 diff=0，重放复验；CERT 账本变化是修复目的本身，以对照表报告。
5. **S1a 与 S0 同源**：S0（73.4%）是同一级别/坐标混配在坐标格上的影子（L1 点 = L0 中枢窗终点 vs D2 turn = L0 腿终点），S1a 是其在 τ 门上的影子。建议 S0+S1a 合并为一项「终端背书裁定」（级别映射 × 坐标口径二维一次裁定），文件冲突见 §7。
6. **预期效果（可证伪预测，待 H4 探针与重放验证）**：32 立即案在 C-a 下即可命中（t*≈破核腿终点=L0 一类点坐标）；5 延迟案需 C-b（窗口 [c_start, t*] 最早点）；37 案 L0 门链逐项预测 = 门开（D2 趋势同链）∧ 锚全 Some（L0 自锚）∧ 破核（t3_ext 离开腿即破核腿，p112 实测 37/37 t3_ext=1）∧ A 可配（同 helper 同锚）∧ 面积过（p112 T5 复算 154/154）。任一环节实测不符即回票本设计。

---

## 1. 病因

### 1.1 案集

37 案（事件级 24.0% = 37/154；链级 197/754 = 26.1%），坐标引 `/tmp/p112_full.txt` P112_CASE 行（`bsp_gate=S1a_tau_none`）；全表见附录 B。分层：**立即三买 32 / 延迟三买 5**（p112:122 同口径）；侧向 Short 22 / Long 15；BSP 账本 35 案 turn 上无点、2 案（turn=1812973、4200339）有**反向**位点（`points=1 confirm=false any_bits=true third_same_side=false`，p112:127-129 的 3 案之二）。

代表样本（p112:137）：turn=867223 Short `seg_a=(866696,867022) seg_c=(867071,867223)` chains=4 立即三买 `(867223,867240)`；turn=1812973 Short chains=12（turn 上有反向点）；turn=3971708 Short chains=24（S1a 最大链簇，延迟三买 `(3972821,3972852)`）。

### 1.2 τ 门生产判据链（文件:行号，含 p115 后现行）

- **门定义**：`decompose.rs:159-169` `center_trend_gate`——`gate[i]=Some(d)` ⟺ 中枢 i 的入边关系 R(i-1,i) 属 Trend(d) 块（i>块首）；`decompose.rs:180-188` `center_own_dir_at`（逐点等价版，resume 路径消费，等价性测试 `center_own_dir_at_equals_trend_gate_pointwise` 锁定）。
- **门消费（full 路径）**：`signal.rs:984-985`（`decompose(centers_sorted)` + `center_trend_gate`）→ `:1040`（`nearest_confirmed_center_idx` 取归属中枢 c_idx）→ `:1045-1051`（`gate_dir` 解析：None ⟹ 不进一类）→ `:1055-1058`（`judge_segment`）→ `:1231-1246`（`if let Some((pos,dir)) = gate_dir` 才判一类，`judge_first_cached:276-368`）。门定义注释（为什么盘整/退化不产第一类）：`signal.rs:234-237`（「盘整背驰不产第一类，beichi #4 + maimai.md:56 已结算」）。
- **门消费（resume 路径）**：`signal.rs:1152-1156`（`center_own_dir_at` 逐点）→ 同一 `judge_segment`。
- **级别-N 入口**：`mod.rs:250-267` `extract_first_third_for_level`（级别-N units 还原 Segment 复用 L0 判据）；`mod.rs:404-418`（L0 走 `extract_signals_with_hist`，级别-N 走本入口）；`mod.rs:431-441`（`units = project_to_units(upper_moves, pb)`、`units_anchors = center_own_dir_at(pb,i)`）。
- **L1 中枢链产出**：`mod.rs:311-319` `classify_level`（is_l0=false ⟹ `detect_centers_geometric`）→ `recursive_tower.rs:716-798` `detect_centers_windowed_resume`（seed=连续 3 单元几何重叠 + 延伸吸收 + non-extension 终止；#148 ≥9 段升级重切）——**每 L1 中枢消费 ≥3 个 L0 中枢单元**。
- **BSP 门链复刻（p112 探针，同口径）**：`p112_trend_predicate_caseaudit.rs:884-891`（`units1 = project_to_units(tower[1], levels[0].moves)`、`anchors1[i]=center_own_dir_at(levels[0].moves,i)`、`centers1=levels[1].centers`、`gate1=center_trend_gate(centers1.len(), &levels[1].moves)`、`bsp_book=levels[1].bsp`）→ `:330-386` `bsp_gate_diag`（S0→S6 首杀标签，S1a 定义在 `:349-350`：`gate1[c_idx]==None`）。

### 1.3 S1a 击杀机制（判据输入对照）

对 37 案逐案（p112:33 定义、p112:88-93 门链定义）：turn（=seg_c.1，#105 口径）恰为某 level-1 单元 u 的终点（S0 通过）⟹ `c_idx = nearest_confirmed_center_idx(centers1, u.start_index)`（:344）⟹ `gate1[c_idx] == None`（:349-350）⟹ 一类路径不触发。`center_trend_gate` 语义（decompose.rs:159-169）下 None ⟺ c_idx==0 或 **R(c_idx-1, c_idx) 属 Consolidation 块**；p112:122 归因全部 37 案为后者（「归属中枢落盘整块」；S1b 反向 = 0，p112:122/166——非方向冲突）。c_idx==0 边角未逐案排除，列入 H4 探针核验项（§3.5）。

门的下游在 37 案上不可达：S2 锚门（`signal.rs:294`，Q7-#1 裁定C）、S3 破核、S4 A 段配对、S5 面积——p112 §4 实测 S3/S5/S6=0 是对**可达 41 案**（S0 通过者）的计数，37 案止步于 S1a。

### 1.4 「缺的上下文」：两级分解

**事实链（每环可溯源）**：

1. `tower[1]` = 每 L0 中枢一个 `RMove::Compose`（`mod.rs:386` `compose_level(units, moves_tower, is_l0=true, level=1)`；`recursive_tower.rs:847-861` 窗=seed 三段+延伸段）。
2. nest level-1 投影种子 = tower[1] 窗**携带核** = L0 中枢（`level_view.rs:321-405`；条款 1（#90 结裁）「准绳=塔 compose 携带核，#89 已证与重算 detect 逐窗 bit-equal」）。
3. nest level-1 的 `blocks = decompose(seeds)` = **L0 中枢链**的分解（探针 `p112_trend_predicate_caseaudit.rs:411-412`；生产 `p92_nest_replay_postruling.rs:648-649`）。
4. D2 pair = 该块上跨 ≥2 **L0 中枢**的趋势对（`level_view.rs:823-883` `provide_divergence_pairs`；prev/last = `seeds[end-1]/seeds[end]`）；A/C 段 = **L0 线段** episode（`lower_legs_from(&tower[0])`，`p92:627`）；确认 = `trend_confirm_time`（`level_view.rs:548-639`，R1 全合取）在 L0 坐标系。
5. BSP level-1 的 τ 门对象 = `levels[1].centers` = **L0 中枢单元三元组上的几何检测链**（`mod.rs:311-319` is_l0=false → `detect_centers_geometric`；`recursive_tower.rs:716-798`），一类判据 = L0 中枢单元破 L1 中枢（`mod.rs:250-267` → `signal.rs:1231-1246`）。

**结论**：D2@1 事件 = L0 中枢链趋势背驰（结构上 ≡ `levels[0].moves` 的趋势块、≡ `levels[0].bsp` 的一类判据对象）；BSP@1 τ 门 = L1 中枢链趋势门。37 案的「上下文」= **L0 中枢链上的 Trend 块**——它对 L1 门不可见，因为 L1 门的输入域（`levels[1].centers` 的关系标签）根本不携带这个对象。37 案中 turn 上的单元（c 段腿构成的新 L0 中枢窗）是某 L1 中枢的成员/后继，而该 L1 中枢与前一 L1 中枢是扩展关系（盘整块）——同一市场区域，L0 链读「趋势离开+背驰」、L1 链读「盘整」，两者都是各自链上的**正确**判决。

**对 p112 §4 归因的修正（附录 A 证明）**：p112:122 写「BSP 全局 decompose vs D2 per-run 块上下文差异」。若两条分解在同一链上，per-run（连续切片）与全局在被消费中枢上的门值**恒等**（关系标签逐对相同 ⟹ 块结构在切片内相同；唯一切片首中枢可能不同，而它永不是 pair 的 last 中枢）。故「per-run vs 全局」不是机制；机制是**链级别不同**（L0 中枢链 vs L1 中枢链）。此修正不改 p112 任何计数，只改 §6-6 修法方向。

---

## 2. 教义依据（全部直读主仓博文原文核对，课号:行号）

1. **趋势的级别 = 中枢的级别**：037:16「当说a+A+b+B+c中有背驰时，首先要a+A+b+B+c是一个趋势。而一个趋势，就意味着A、B是同级别的中枢」；027:14「趋势，一定有至少两个同级别中枢」。⟹ D2@1 事件的趋势级别由其 A/B 中枢（=L0 中枢）决定，是 **L0 中枢级别**的趋势背驰。
2. **买卖点存在于背驰自己的级别**：024:18「缠中说禅背驰-买卖点定理：任一背驰都必然制造某级别的买卖点，任一级别的买卖点都必然源自某级别走势的背驰。」——背书点应到**该背驰所在级别**的账本找（levels[0]），而非上一级。
3. **级别查找方向只有下界**：043:26「由于背驰的级别不可能大于当下走势的级别，例如一个30分钟级别的背驰，只可能存在于一个至少是30分钟级别的走势类型中」。⟹ 终端背书向**上**一级（levels[ℓ]）查找违反上界；向**下**（levels[ℓ-1]）合法。
4. **第一类 ⟺ 趋势背驰**：027:66「第一类买点肯定是趋势背驰构成的，而盘整背驰构成的买点，在小级别中是意义不大的……（类第一类买点）这个级别，至少应该是周线以上」。⟹ 1 分钟链上盘整背驰不产第一类；τ 门拒盘整块开一类是教义实装，不是缺陷。
5. **级别误判是错误，不是口径**：027:136（宝钢案）「你对背驰的判断是错误的，宝钢15分钟根本没有背驰……该背驰是典型的1分钟背驰」。⟹ 把 L0 中枢级别背驰贴到 L1 账本是级别误判；归属必须按真实中枢链。
6. **背驰级别=走势级别的操作直接性**：044:32「对于『背驰级别等于当下的走势级别』，如果你刚好是该级别为操作级别的，只要在顶背驰时直接全部卖出就可以」——确认与操作都锚在背驰自己的级别。
7. **否则条款（盘整归盘整背驰处理）**：037:18「c必然是次级别的……否则，就可以看成是B中枢的小级别波动，完全可以用盘整背驰来处理」；027:20「这里是把第一、三段看成两个走势类型之间的比较，这和趋势背驰里的情况有点不同」。⟹ L1 链上读出的盘整对象，其出口是盘整背驰通道（PanDivCert，`signal.rs:1248-1254`），不是第一类。
8. **背景色（L1 读盘整不奇怪）**：029:16「某级别趋势的背驰将导致该趋势最后一个中枢的级别扩展、该级别更大级别的盘整或该级别以上级别的反趋势」——L0 趋势前后在 L1 链上呈现扩展/盘整是定理允许的形态，两链读数不矛盾。

---

## 3. 修法设计

### 3.1 选项 A：放宽阈值（松 τ 门）——**拒绝**

- 形态：对盘整块中枢开门（如「弱盘整/近趋势」条件、或对 ownership 为 Consolidation 的 c_idx 给 `Some(dir)`）。
- 教义：无任何一条。反向证据充分——027:66（第一类=趋势背驰构成；盘整买点仅周线以上有意义）、037:18 否则条款、027:20、044:32；工程结算 `signal.rs:234-237`（τ 门是「假背驰=假买卖点头号缺口」的修复，`first_buy_rejected_in_consolidation_tau_gate` 测试锁定）。
- 误放评估：**洪水级**。L1 盘整块内每次破 ZG/ZD 且面积衰减都成 B1/S1——p112 §4 实测 41 可达案全部破核心成立、面积零变号，说明「盘整+破核+衰减」在数据里大量存在；放宽后这些全部置一类 bit。S1b=0（无方向冲突）恰说明当前门的方向判读健康，病不在门。
- 结论：拒绝。且碰 `center_trend_gate`/`judge_segment` = BSP 生成主干（§4），双重不可行。

### 3.2 选项 B：补上下文参数（门内注入 L0 上下文）——**拒绝（其诚实形态 = C）**

- 形态：在 `judge_segment`/`judge_first_cached` 增加入参（如 L0 块归属/趋势方向），`gate_dir=None` 时改查 L0 上下文后强行判一类。
- 拒绝理由四条，逐条可验：
  1. **对象错位**：一类判据本体（`judge_first_cached:276-368`）需要 last_center 的 zg/zd、prev_center 的 A 段、L1 锚——注入的 L0 上下文里趋势 B 是 **L0 中枢**（不同对象）；拿 L1 中枢 c（盘整块成员）配 L0 趋势方向，破核判据（:294-300）语义崩坏；若改传 L0 中枢 B，则 `a_seg_cache`（键=c_idx=L1 下标，:1235-1237）与 anchors1（L0 走势块 ownership 方向，非 L0 中枢趋势方向）全错位。实质 = 在 L1 循环里把 `levels[0]` 的一类判据重抄一遍。
  2. **级别冒充（090）**：产出点的坐标/bit 进 `levels[1].bsp` 账本，声明的是 L1 一类、内容却是 L0 中枢趋势——声明与能力不一致，违 090；与 027:136 级别误判同类。
  3. **重复登记**：同一结构若真成立，`levels[0].bsp` 本就产出该点（门同链、锚全 Some、同 helper）——B 是第二套实装，违「禁第二套力度引擎/单一来源」纪律（`mod.rs:244`）。
  4. **碰主干**：`judge_segment`/`judge_first_cached` 是 BSP 生成主干，改动即 BSP 账本 bit 变化（§4），须升级裁定。
- 结论：拒绝。B 想达到的效果 = 「让 L0 中枢趋势的一类点被终端看见」——这正是选项 C 用**查法**（而非判据注入）完成的事。

### 3.3 选项 C：改判据结构 = 终端背书级别对齐——**推荐**

- 判定：τ 门不动（它是对的）；改的是 **nest 终端背书的查法**——nest level-ℓ 事件的原子 = tower[ℓ] 窗 = levels[ℓ-1] 中枢（§1.4 事实链 1-3），其结构级别 = ℓ-1，故背书账本 = `levels[ℓ-1].bsp`（教义 §2-1/2/3/5）。对 37 案：背书改查 `levels[0].bsp`——其 τ 门（`levels[0].moves`）与 D2 同链 ⟹ 门开；锚全 Some ⟹ S2 不杀；破核腿即 t3_ext 离开腿（37/37 t3_ext=1，p112 实测）⟹ S3 过；A/λ_C 同 helper 同自锚 ⟹ S4 过；面积 p112 T5 复算 154/154 ⟹ S5 过。
- 两口径（裁定项）：
  - **C-a 精确坐标**：`p.source_index == event.turn_source ∧ p.bits.confirm_side(event.side)`（现查法平移一级）。最小改动；预测 32 立即案命中（t*≈破核腿终点），5 延迟案 miss（t* 远晚于破核时点）。
  - **C-b c 窗口最早点（推荐默认）**：`p.source_index ∈ [event.interval_b.0, event.turn_source]` 内**最早** confirm_side 点。最早性 ⟹ 该点判决所对中枢 = D2 的 B（c_start 前无更新中枢、失败离开在 c_start 之前 ⟹ 窗口内首个一类点必对 B；窗口内后出现的对 c 内新中枢的点属下一趋势，不取——误放封闭，证明见 §6-1）。对 70 延迟案形态（t* 晚于破核）鲁棒。
- 不改事件结构：`NestCandidateEvent` 字段不动（interval_b=(c_start,t*) 已够；EventKey/身份派生零迁移）。
- 与 S0 的关系：级别移位后，levels[0] 坐标格 = L0 线段终点 = D2 turn 同格——S0 的 113 案在该格上**天然可命中**（待实测）；故 S0/S1a 是同一缺陷的两个影子，建议合并裁定（§7）。

### 3.4 C 的 hunk 级施工图

- **H1（lib，`rust/src/theta_v0/classifier/nest.rs` 新增）**：
  ```rust
  /// nest level-ℓ 事件 → BSP 账本级别（S1a 裁定：事件原子=tower[ℓ]窗=levels[ℓ-1]中枢）。
  pub fn event_bsp_book_level(event_level: u32) -> Option<usize>  // ℓ≥1 ⟹ Some(ℓ-1)；ℓ=0 ⟹ None
  /// 终端背书（级别对齐）。Exact = 现查法平移；CWindow = [interval_b.0, turn_source] 最早 confirm_side。
  pub enum TerminalMatch { Exact, CWindow }
  pub fn terminal_bits_at_event(c: &Classification, e: &NestCandidateEvent, m: TerminalMatch) -> Option<BspBits>
  ```
  位置：nest.rs 装配层（`assemble_typed_certificate:465-522` 同文件，终端契约 `:479-481` 的单一来源）；`use super::Classification`（同 crate 无环）。
- **H2（bin，`rust/src/bin/p92_nest_replay_postruling.rs:940-953`）**：`terminal_bits_new` 改为委托 `nest::terminal_bits_at_event(classification, event, TerminalMatch::CWindow)`（口径常数一处，便于 C-a 敏感性对照）；同步审计 `terminal_bits_old:955-968` 的两个调用点（:268/:278，P92_MISSED 类审计路径），同口径平移并标注。
- **H3（文档，nest.rs:460-464）**：`assemble_typed_certificate` 头注补终端契约——`terminal_of` 须返回**事件结构级别**（levels[exec-1]）的 confirm_side bits；调用方须用 `terminal_bits_at_event` 或等价映射。
- **H4（只读核验探针，新 bin `rust/src/bin/p117_s1a_level_recheck.rs`，bin 自动发现、不动 Cargo.toml）**：输入 btc_1m_full.json；硬锚 37 案坐标常量表（附录 B）；逐案在 levels[0] 复刻门链（nearest L0 center、gate、broke zg/zd、`locate_departure_move_a`、`departure_move_c_start`、`AbcDivergence::diverges`、窗口内最早 confirm_side 点坐标）+ 对照 levels[1] 现状；输出 `P117_CASE ...` / `P117_RESOLVED c_a=<n> c_b=<n> of 37`。同时跑 11 个现行过门基例与 S0 抽样（回归监视）。**p116 重放完成后方可 cargo run**（本工位本轮零 cargo）。
- **H5（单测，见 §5）**。

### 3.5 前置核验（实装门禁，全只读）

1. H4 探针：37 案 L0 门链逐项实测 vs §0-6 预测（任一不符 ⟹ 回票）；c_idx==0 边角排除；反向位点 2 案的 L0 实况。
2. p116 重放产物（`/tmp/p92_ckpt_dump_v2.txt` 已有）+ 实装后重放：P92_BIT_EXACT 五维 diff=0；CERT A/B 对照（基线 66 张存续审查——p115 §6 已声明不为保产量叠旧口径，本设计同纪律）；11 过门基例存续率。
3. R1 坐标提示：p112 的 S0/S1a 切分是在 #105 坐标（turn=首腿终点）上量的；R1 后 turn_source=t*，门链分布会迁移（p116 重放在量），但 S1a 的机制（级别混配）与坐标无关，本设计对两种坐标口径均成立。

---

## 4. bit-exact 冲突面

| 对象 | 是否触碰 | 说明 |
|---|---|---|
| 塔（recursive_tower.rs：笔/线段/窗口/inclusion/中枢检测/compose） | **否** | 零改动 |
| 走势分解/τ 门（decompose.rs） | **否** | 零改动（门判决正确，不动） |
| 一类/三类判据（signal.rs judge_segment/judge_first_cached/judge_third/extract*） | **否** | 零改动 |
| BSP 结构（bsp.rs/types.rs BspPoint/BspBits） | **否** | 零改动（含 center 字段——一类点 center=None，身份 join 不可用，已弃该子选项，见 §6-1） |
| 力度原语（divergence.rs）/nest 事件生产（level_view.rs） | **否** | 零改动（事件结构不加字段） |
| **nest 装配（nest.rs）** | **是** | 新增 `event_bsp_book_level`/`TerminalMatch`/`terminal_bits_at_event` + 头注契约（H1/H3）；不改 `assemble_typed_certificate` 本体逻辑 |
| **p92 bin（p92_nest_replay_postruling.rs）** | **是** | `terminal_bits_new` 委托 + `terminal_bits_old` 调用点审计（H2） |
| **新探针 bin（p117_s1a_level_recheck.rs）** | **是（新增）** | 只读测量仪器，bin 自动发现 |
| Cargo.toml | **否** | 禁改 |

- **主干判定**：未碰塔/笔/线段/inclusion/中枢/BSP 生成路径——**无需升级裁定**。P92_BIT_EXACT 五维（old_path/tower/moves/centers/bsp/pan）在构造上不受影响（这些产物在终端查法之前、独立于查法产出），实装后重放复验 diff=0。
- **有意变化面（须报告，非隐藏）**：CERT 账本（A/B 计数、ids 中 terminal bits 来源级别）。这是修复目的本身；以「基线 66 张 + v2 重放」对照表报告，存续审查按 p115 §6 纪律（不叠旧口径保产量）。
- **被拒绝选项的主干面（备查）**：选项 A 碰 decompose.rs/signal.rs（门+一类主干）；选项 B 碰 signal.rs（judge_segment/judge_first_cached）+ anchors/cache 键——均为 BSP 生成主干，若采任一须升级裁定。

---

## 5. 单测计划（4 新增，nest.rs `#[cfg(test)]` 内；全量回归兜底）

| 测试名 | 断言 | 数据构造 |
|---|---|---|
| `event_bsp_book_level_shifts_exactly_one_down` | level=1→Some(0)、level=2→Some(1)、level=0→None、level=6→Some(5) | 纯函数直调 |
| `terminal_bits_at_event_exact_reads_child_book` | levels[0] 有 confirm 点@100 ⟹ 命中；仅 levels[1] 有 ⟹ None（新语义防回归）；side 反 ⟹ None；event.level=0 ⟹ None；level 越界 ⟹ None | 手工 `Classification{levels: vec![LevelState{bsp: Rc::new(vec![pt@100 Long conf])}, LevelState::default()]}` + `NestCandidateEvent{level:1, turn_source:100, side:Long, ..}`（全 pub 字段，无 parser 依赖） |
| `terminal_bits_at_event_c_window_picks_earliest_confirm` | 窗口 [100,200]：@99 排除、@120 命中且为最早、@130 反向忽略、@150 同向但不取（最早胜）、@201 排除；空窗 ⟹ None | 同上构造 5 点 + `interval_b=(100,200)` |
| `assemble_typed_certificate_terminal_uses_shifted_book` | 基例 level=1 confirmed + levels[0] 窗内 confirm 点 ⟹ Some(cert)；删该点 ⟹ None；点只在 levels[1]（levels[0] 空）⟹ **None**（移位后不再读 levels[1]，防旧语义回潮） | 复用 nest.rs 既有装配夹具风格（`assemble_typed_certificate` 单基例、top=1 免 rung） |
| 回归（非新增） | `cargo test --lib` 基线 1647 全绿（含 `first_buy_rejected_in_consolidation_tau_gate`、decompose 全部门测试、07a resume/full bit-exact debug_assert）；`cargo check --bins` 零错误 | 实装后跑（p116 重放完成后） |

---

## 6. 风险与边界（误放/误杀评估）

### 6.1 误放（会不会把不背驰的放进来）

1. **背驰谓词本身零放宽**：基例门仍是 `divergence_confirmed`（R1 全合取：T4 回拉0轴 ∧ T3 三买 ∧ T2 破极值 ∧ T5 力度或关系，`level_view.rs:548-639`）。本设计只动「背书去哪个账本找点」，不动「何谓背驰」。不背驰的事件根本到不了终端门。
2. **C-b 窗口 join 的身份保证（证明）**：窗口 = [c_start, t*]（c_start=`departure_move_c_start` 当前 episode 首腿，失败离开→回中枢→重新离开不桥接，`signal.rs:1031-1034` 语义）。窗口内**最早** confirm_side 一类点 p：p 的判决中枢 = 其腿前最近中枢（single-claim，`signal.rs:1038-1042`）；c 段新中枢须 ≥3 腿方能 seed，故窗口内首个破核腿的最近中枢必 = B（或更早中枢，但 c_start 起最近中枢即 B）。⟹ p 必为「破 B」的点——正是该背驰的一类点（027:66）。c 内新中枢形成后的破核点（属下一趋势）因非最早被排除。方向由 `confirm_side`（`types.rs:216-221`，conf_plus/conf_minus 析取）锁死。
3. **坐标精确口径（C-a）误放为零**（现语义平移）；代价是 5 延迟案 miss（转 C-b 覆盖）。
4. **反向位点 2 案（1812973/4200339）自然消解**：那 2 个点在 levels[1] 且 confirm=false（反向）；移位后不再被查。它们是级别混配的旁证而非「无背驰可读」（p112:128-129 同旨）。
5. **已弃子选项（备查）**：按 `BspPoint.center` 做中枢身份 join——一类点 center=None（`signal.rs:515`），改它则 BSP 账本 bit 变化（P2-R2 铁律 `bsp.rs:127-143` 附近、GOLDEN digest 翻转）⟹ 主干，弃。earliest-in-window 已结构性给出身份保证，无需动 BspPoint。

### 6.2 误杀（会不会把该过的杀掉）

1. **口径错位（有界、可测）**：L0 一类面积判据 = `AbcDivergence::diverges`（混合柱 Σ|hist|，`signal.rs:331`），R1 确认 = T5-OR（同色面积∨黄白线峰∨柱峰）。若某事件 T5-OR 过而混合面积不过 ⟹ L0 点零 buy1/sell1 bit ⟹ 背书 miss。p112 实测本 154 案混合面积 154/154 过（T5 复算），故本批为零；一般情形由 H4 探针计数，方向是**诚实判负**（非误放）。
2. **50 条现行过门链的存续**：背书从 levels[1] 移到 levels[0]，11 个过门基例须在新查法下仍命中（其确认在 #105 口径为混合面积，与 L0 判据同源，预测存续）；重放对照报告，任一失证逐案核。
3. **run 首中枢边角**：per-run 与全局门值唯一可能分叉点（附录 A）——pair 的 last 中枢永不取它，预测零影响；H4 探针顺带计数。
4. **fiat 反读（必须呈交的裁定备选）**：若编排者裁定「nest 桶号 ℓ ≡ BSP 级别 ℓ」是有意贴标签（非缺陷），则 37 案在 L1 的对象类别 = 盘整（037:18 否则条款），出口是 PanDivCert 通道（877 pan 基例同口径，p109:21），本设计整体不适用。该读法须以裁定文书明确「事件的级别身份与其中枢链级别可以不一致」，与 037:16/024:18/043:26/027:136 的直读冲突由裁定者承担。本工位按结构内容呈交，不替裁定。
5. **R1 坐标迁移**：#105→t* 的坐标变化使 p112 的 S0/S1a 切分比例待新重放重测（§3.5-3）；机制与坐标无关，设计稳健。

---

## 7. 与其他三项的潜在文件冲突

本设计触及文件清单：`rust/src/theta_v0/classifier/nest.rs`、`rust/src/bin/p92_nest_replay_postruling.rs`、新增 `rust/src/bin/p117_s1a_level_recheck.rs`、本文档。

| 工作项 | 共享文件 | 冲突级 | 处置 |
|---|---|---|---|
| **S0 取段/坐标错位（73.4%）** | nest.rs（终端契约）+ p92 bin（terminal_bits_new） | **高** | S0 的修法选项（p112 §6-5：「含 turn 的单元区间匹配」或「nest 自定义确认位」）与本设计改的是**同一查法**。强烈建议 S0+S1a 合并为一项「终端背书裁定」：级别映射（ℓ→ℓ-1）× 坐标口径（精确/区间）二维一次裁定，单 hunk 属主；若 S0 先落地区间匹配而未移位级别，背书仍查错账本，37 案不愈。**并案证据（同日 sibling 文档）**：`p117-bsp-repair-design-s0-20260717.md:17-29` 已落同一落点（nest.rs 加桥接查法 `terminal_bits_bridged` + p92 闭包切换），桥键 = 「c 责任单元」（037:22 c 内含次级别中枢在级别-k 网格的见证）。两设计的分工事实：S0 桥接留在 `levels[ℓ]` 修坐标格，**不治 S1a**——37 案在 L1 账本根本没有点可桥（τ 门在生产侧已杀）；本设计的级别移位则把 S0 的 113 案一并带回 L0 腿坐标格（S0  grid 失配在同格下消解），C-b 窗口与 S0 的「c 责任单元」在 [c_start, t*] 语义上同构（最早点 = c 首个破核腿，即 037:22 次级别中枢见证的触发腿）。合并裁定建议：级别移位为本（治 S0+S1a），S0 的单元桥作为 levels[ℓ] 留存查法（exec≥2 的 rung 级与审计路径）的口径候选 |
| **S2/S4 anchor 资格门（2.6%）** | 无（其文件 = signal.rs `judge_first_cached:294` / `locate_departure_move_a`） | 零文本冲突 | 语义交互一条：S2/S4 修复改变 `levels[1].bsp` 内容；本设计落地后 levels[1] 仅服务 nest-2 基例（exec=2）的背书，nest-1 基例不再消费——两项的产量对照须按 exec 分层报告，免误读 |
| **R1 后续 / p116 探针** | p92 bin（`terminal_bits_new:940-953` 被 p116 规则书 R2.2.1 引用为 TERM 谓词，`p116-turnpoint-anchor-existence-20260717.md:146-149`） | 中 | 本设计改该函数语义（级别移位）⟹ p116 的 C2 归因口径须同步移位，否则 p116 重跑会出幻影 C2（在 levels[ℓ] 查无点但实际背书已移 levels[ℓ-1]）。R1 本体（level_view.rs）本设计零触碰，无文本冲突 |

另：`rust/src/bin/p112_trend_predicate_caseaudit.rs`（p112 一次性仪器，门链复刻用 levels[1]）**归档不改**；其 S1a 测量由 H4 探针以新口径取代。

---

## 8. 边界与纪律声明

- 本轮零生产源码改动、零 cargo build/test（p116 重放在途不干扰）、零 git mutation、主仓 `/Users/silencehan/Projects/NewChanlun` 零写入（博文原文只读核对）；唯一新增文件 = 本文档。
- 090：本文「推荐」是设计呈交，级别移位裁定权在编排者；§6.2-4 已呈交最强反读（fiat）。未实跑任何验证（纪律禁 cargo）；§0-6 的预测、§3.5 的探针规格、§5 的测试名为实装门禁，未验证处一律标注「待 H4/重放」。
- v3 硬禁令：不含概率/统计推断、不回测验证策略、不假设 EMH；全部论证为结构/教义/代码事实。
- 教义引用全部直读主仓 `docs/chanlun/text/blog/` 原文核对（课号:行号见 §2 与文头清单）；chan99 未单独引为证据。

---

## 附录 A：per-run 与全局 decompose 门值无关性证明（对 p112:122 归因的修正）

**命题**：设 per-run 种子链 = 全局 L0 中枢链的连续切片 [k, n)（carried ≡ detect，#89 逐窗 bit-equal 已结算），则对切片内任一被 D2 pair 消费的中枢（下标 i>k），`center_trend_gate` 在 per-run 分解与全局分解下取值相同。

**证明**：`decompose` 是关系序列上的确定性左折叠（`decompose.rs:56-74` `fold_rel`）；关系 R(i-1,i)=`classify_relation(C_{i-1},C_i)` 只依赖两个中枢的值 ⟹ 切片内关系标签与全局逐对相同。门值 `gate[i]=Some(d)` ⟺ R(i-1,i) 属 Trend(d) 块且 i>块首（`decompose.rs:159-169`）：
- i ≥ k+2：切片内块边界由切片内关系标签决定，与全局在切片内的限制相同 ⟹ 门值相同。
- i = k+1（切片首个关系）：per-run 块首 = k ⟹ k+1>k 门开（若 R 为 Trend）；全局块首 ≤ k ⟹ k+1 必 > 块首，门亦开且同向 ⟹ 相同。
- i = k（切片首中枢）：per-run 恒 None；全局取决于 R(k-1,k)——**唯一可能分叉点**；但 D2 pair 的 last 中枢 = 块尾（`block.end_center > block.start_center ≥ k` ⟹ end ≥ k+1）且 prev = end-1 ≥ k，消费位不含 i=k 的门值（prev/last 是中枢对象本身，门只消费 last 所在块的 dir）。∎

**推论**：「per-run vs 全局」不产生门差；37 案的门差只能来自**链不同**（`levels[1].centers` vs L0 中枢链）——即 §1.4 的级别混配。p112 的计数不受影响（其 S1a 判定用的就是 levels[1] 全局门），受影响的是 §6-6 修法方向的表述。

## 附录 B：37 案坐标全表（引 `/tmp/p112_full.txt` P112_CASE，`bsp_gate=S1a_tau_none`）

格式：turn | side | seg_c（#105 坐标）| chains | 三买形态（立即=leave 终于 turn；延迟=leave 终值>turn）| BSP 账本实况。seg_a/t3_ext_hit/面积等全字段见 P112_CASE 原行。

| # | turn | side | seg_c | chains | 三买 | 账本 |
|---|---|---|---|---|---|---|
| 1 | 867223 | Short | (867071,867223) | 4 | 立即 (867223,867240) | none |
| 2 | 977720 | Long | (977629,977720) | 4 | 立即 | none |
| 3 | 1154662 | Long | (1154603,1154662) | 2 | 立即 | none（兼 T4-ratio10 仅败 3 案之一，p112:142） |
| 4 | 1444502 | Long | (1444331,1444502) | 10 | 立即 | none |
| 5 | 1511155 | Short | (1511033,1511155) | 5 | 立即 | none |
| 6 | 1545297 | Long | (1545202,1545297) | 6 | 立即 | none |
| 7 | 1684581 | Short | (1684405,1684581) | 12 | 立即 | none |
| 8 | 1812973 | Short | (1812944,1812973) | 12 | 立即 | points=1 反向 |
| 9 | 1890249 | Short | (1890175,1890249) | 4 | 立即 | none |
| 10 | 1963252 | Long | (1963034,1963252) | 2 | 立即 | none |
| 11 | 2035662 | Short | (2035524,2035662) | 4 | **延迟** (2063914,2064134) | none |
| 12 | 2043126 | Short | (2042972,2043126) | 2 | 立即 | none |
| 13 | 2090467 | Short | (2090302,2090467) | 8 | 立即 | none |
| 14 | 2136917 | Short | (2136843,2136917) | 6 | 立即 | none |
| 15 | 2178474 | Short | (2178454,2178474) | 12 | 立即 | none |
| 16 | 2195437 | Short | (2195268,2195437) | 6 | 立即 | none |
| 17 | 2241463 | Long | (2241312,2241463) | 6 | 立即 | none |
| 18 | 2669193 | Long | (2668906,2669193) | 6 | **延迟** (2672552,2672592) | none |
| 19 | 3595896 | Long | (3595811,3595896) | 1 | 立即 | none |
| 20 | 3602474 | Long | (3602333,3602474) | 2 | 立即 | none |
| 21 | 3818615 | Long | (3818541,3818615) | 2 | 立即 | none |
| 22 | 3828263 | Long | (3828115,3828263) | 2 | **延迟** (3858568,3858607) | none |
| 23 | 3916879 | Long | (3916719,3916879) | 4 | 立即 | none |
| 24 | 3971708 | Short | (3971407,3971708) | 24 | **延迟** (3972821,3972852) | none |
| 25 | 3989658 | Short | (3989432,3989658) | 12 | 立即 | none |
| 26 | 4015179 | Short | (4015041,4015179) | 4 | 立即 | none |
| 27 | 4018257 | Long | (4018100,4018257) | 4 | 立即 | none |
| 28 | 4032359 | Short | (4032284,4032359) | 4 | 立即 | none |
| 29 | 4044258 | Short | (4044196,4044258) | 4 | 立即 | none |
| 30 | 4108519 | Short | (4108342,4108519) | 2 | 立即 | none |
| 31 | 4172278 | Short | (4172028,4172278) | 2 | 立即 | none |
| 32 | 4186102 | Short | (4185938,4186102) | 4 | **延迟** (4189334,4189454) | none |
| 33 | 4189745 | Short | (4189641,4189745) | 4 | 立即 | none |
| 34 | 4200339 | Long | (4200242,4200339) | 4 | 立即 | points=1 反向 |
| 35 | 4298765 | Short | (4298554,4298765) | 3 | 立即 | none |
| 36 | 4321214 | Short | (4321166,4321214) | 2 | 立即 | none |
| 37 | 4324005 | Long | (4323850,4324005) | 2 | 立即 | none |

合计：chains 197（26.1% of 754）；立即 32 / 延迟 5（延迟案 = #11/#18/#22/#24/#32）；Short 22 / Long 15。

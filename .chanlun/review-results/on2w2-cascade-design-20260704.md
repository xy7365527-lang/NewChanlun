# on2w2 cascade_reset 增量化架构件（设计文档，只设计+codex审，不实装）

工位 on2w2 / 认识论：本文档全部为 **L0（纯代数/定义推导）**——失效边界定理从现有不变量（源坐标单调、
前缀不可变、frontier pop 协议）推导，零数据。计时收益预估标 **L1（管线度量，231号零信息增量）**。
代码影响：**零**（设计文档，无代码改动）。
基线 HEAD（session 起）：`cae6922876`。目标文件（只读参照，不改）：
`rust/src/theta_v0/classifier/mod.rs`（cascade 块 1195-1211 / frontier 协议 1228-1294 / 09 投影
1404-1418）、`recursive_tower.rs`（detect/compose/project resume 410-607）、`decompose.rs`
（DecomposeState 80-135）。

---

## 0. 问题陈述与背景锚

### 0.1 现状（实测机制，#65 坐实）

`classify_with_tower_incremental` 主循环（mod.rs:1151 `for level_idx in 0..=l_max`）持有一个**跨级共享
布尔** `cascade_reset`（1142 声明）。任一级检出

```
units.len() < lc.last_input_len   // (A) 段账本前缩
|| frontier_mutated               // (B) units[stable..scanned] != cached_units[stable..scanned]
```

即 `cascade_reset = true`（1196-1198），且该布尔**对本级及所有更高级无条件为真**（loop 携带变量，
上级不复位）。cascade 命中时该级**整塔前缀清空**（1199-1211）：

| 缓存字段 | 现动作（1199-1211） |
|---|---|
| `scan_cursor` | `= WindowScanCursor::default()`（resume_from=consumed=0，从头全扫） |
| `upper_moves` | `Rc::make_mut(..).clear()` |
| `centers` | `Rc::make_mut(..).clear()` |
| `decompose_state` | `.reset()`（frozen 清空，frozen_rels=0） |
| `cached_bsp` / `cached_pan_div` | `.clear()`；`cached_bsp_key = None` |
| `cached_second` / `cached_second_count` | `.clear()` / `= 0` |
| `projected_units` | `Rc::make_mut(..).clear()` |

下一 bar 该级从 `start_i=0, prefix_count=0` **全量重建该级已积累的全部历史**。

### 0.2 O(n²) 机制（#65 强证据）

`tailwidth-diag-20260702.md`（#65）双窗坐实：cascade **触发频率 ∝ n**（300K→1M 全 level 汇总
904→3542 次），**单次重建规模 ∝ n**（level0 avg wiped 402.51→1286.50，max 763→2532——与 enum2 报告
frontier tail 宽度 max **逐位相同**）。两个线性因子相乘 ⟹ cascade 驱动的总重建成本 **O(n²)**，占
`extract_second` 报告 tail 宽度质量的 **97-99%**，同源驱动 05_compose_resume / 09_project_to_units_resume
/ 07b 三阶段（H3：cascade 前缀重扫占 98.6%，avg 97.5 max 2095 units/miss）。

### 0.3 #65 NO-SHIP 的理由 + 本文档的立场

#65 判 NO-SHIP，理由：「局部失效方案需**推翻 codex option-2 裁决**（局部 frontier_mutated 投影比对
有损，可能漏判深层 sub_moves 改写）= 独立高风险形式化任务」。

**本设计的核心命题：增量失效 ≠ 推翻 option-2。** option-2 是关于**探测完备性**的裁决（投影比对
探测不到深层 sub_moves 变 ⟹ 上级不能仅靠本级投影判断 ⟹ 必须无条件向上传播）。本设计**完整保留**
无条件向上传播——探测与传播机制**一字不改**，只把**每级的响应动作**从「整塔前缀清空」改为「按
源坐标 `e` 的后缀失效」。深层 sub_moves 变仍被无条件传播捕获（§3.3 证明）。故 #65 的 NO-SHIP 理由
在本设计下**不成立**——本设计不落在它否定的方案类里。

---

## 1. 失效边界定理

### 1.1 定义

- **源坐标（source-index）**：原始 K 序下标。`UnitRange`/`Center`/`LeveledMove` 均携
  `start_index`/`end_index`（源坐标，center.rs:48 / types.rs:112 / recursive_tower.rs:103）。
- **源单调不变量（S1）**：任一级 `centers`/`upper_moves` 按 `end_index` 严格升序（每窗口一中枢，
  窗口沿 units 左折叠连续推进）；`units` 同（投影保序，center 保序）。**append-only + 前缀不可变**
  （§16，codex `project_to_units_resume` 审计坐实）。
- **脏源下界 `e`（dirty_src）**：本 bar 到当前级为止，**任一级检出的最小改变源坐标**。初值 `+∞`。

### 1.2 定理（后缀失效边界）—— ★codex审后订正：读域边界，非 end_index 边界

> **定理 T1（订正版）**：设本 bar 累积脏源下界为 `e`。定义每个已产出 center 的**读域上界**
> `read_end_src`（= 产出该 center 的窗口在 detect 中读取过的最大源坐标；含 non-extension 停止哨兵
> `units[j]`——见下 §1.2.1）。则在**每一级**，满足 `read_end_src < e` 的 centers/upper_moves 前缀
> 跨 bar **bit-identical**（可保留）；`read_end_src >= e` 的后缀可能变（必须失效重扫）。该边界在塔中
> **逐级不变**（`e` 不随级别升高而改变）。

失效响应 = 把布尔 `cascade_reset` 的「clear()」替换为「truncate 到 `read_end_src < e` 的前缀 P」，
scan 从 P 对应窗口的退出点 resume（§3.4）。现全清空是 `e = 0`（P=0）的特例。

### 1.2.1 ★为什么是读域上界，不是 end_index（codex 反例修正）

`detect_centers_windowed_resume`（recursive_tower.rs:422-476）的延伸循环
`while j < len && units[j].lo<=zg && units[j].hi>=zd`（430）在**停止时读了 `units[j]`**（首个
non-extension 哨兵），但输出 center 的 `end_index = units[j-1].end_index`（延伸吸收到 j-1）。

**codex 反例**：旧窗口 `u0..u5` 成中枢，center.end = `u5.end`；哨兵 `u6` 不触核心（停止），
`u6.start = e`。旧 center.end `< e` ⟹ 若按 end_index 边界会**误保留**。但下一 bar `u6` 改为触核心
（`j_min=6, e=u6.start`），全清重扫会把 `u6` 延伸进同一中枢 ⟹ center.end 变 `>= e`。增量误保留旧
center ⟹ **与全清分叉，unsound**。

根因：center 的**读域**（决定其形态的输入单元集）⊋ 其**输出区间** `[start,end]`——多含一个停止
哨兵 `units[j]`。故边界必须按读域（含哨兵）取，不能只看输出 end。修正：`read_end_src = units[j].start_index`（哨兵源坐标；若 `j==units.len()` 该窗口开放无哨兵 ⟹ `read_end_src = +∞`，
永不保留，归 frontier pop 常态处理）。

### 1.3 `e` 的计算（探测不变，只多记一个 min）

探测逻辑（frontier compare + len-shrink）**完全不动**。仅在 (B) frontier_mutated 命中时，把现有的
**布尔比较**换成**记录最小改变下标**：

```
// 现：frontier_mutated = units[stable..scanned] != cached_units[stable..scanned]（只出布尔）
// 改：j_min_local = [stable..scanned] 内首个 units[j] != cached_units[j] 的 j（无 ⟹ None）
//     若 Some(j) ⟹ local_e = units[j].start_index
//     e = min(e, local_e)   // 跨级传播（与现布尔 OR 同位置、同无条件语义）
```

**关键：`j_min` 取比较区间内的最小改变下标**（不是任意一个）——保证 `units[..j_min]` 全未变，
故 `e = units[j_min].start_index` 是「最小改变源坐标」的正确下界（§3.2）。

（A）len-shrink 保留 `e = 0`（全清）——见 §5「保守简化」，回缩罕见，不做增量。

---

## 2. sound 性论证

### 2.1 前缀读取域封闭（S2，核心引理）—— ★订正为读域

> **引理 L1（订正版）**：`read_end_src < e` 的 center 前缀 `centers[..P]`，其全部构成窗口的**完整
> 读域**（seed 三段 + 延伸段 + 停止哨兵）只读源坐标 `< e` 的 units，而该区域跨 bar 未变 ⟹ 前缀
> bit-identical。

证：窗口扫描是**确定性左折叠**：seed 判定读 `units[i..=i+2]`、延伸判定读 `units[j]` vs 冻结核心、
停止哨兵读 `units[j]`（首个 non-extension），均纯函数。center P-1（前缀末）的读域上界哨兵源坐标
`read_end_src < e`。由 S1 源单调，该窗口读域内所有被扫单元（含哨兵 `units[j]`，其 `start_index =
read_end_src < e`）均落源坐标 `< e` 区域，按 `e` 定义（最小改变源坐标）**逐值未变**（含被跳过的
None-支单元与哨兵，其下标 `< j_min`）⟹ 折叠轨迹到 P-1 窗口退出为止逐位复现 ⟹ 前缀 bit-identical。
**关键订正**：边界按读域哨兵 `read_end_src`（含 `units[j]`）取，覆盖 codex §1.2.1 反例——哨兵翻转
（non-extension→extension）时其源坐标 `>= e` ⟹ 该 center `read_end_src >= e` ⟹ 落失效后缀，不误保留。∎

### 2.2 后缀完整覆盖改变（无漏）—— ★订正为读域

改变单元的最小下标 `j_min`（§1.3 取 min 保证 `units[..j_min]` 未变）⟹ 最小改变源坐标
`e = units[j_min].start_index`。任何**读域**含改变单元 `j >= j_min` 的 center（无论 `j` 是延伸段
还是停止哨兵）⟹ 其 `read_end_src = units[哨兵].start_index >= units[j].start_index >= e`（S1 单调，
哨兵下标 >= j）⟹ 落失效后缀 `read_end_src >= e`。**含哨兵读域 ⟹ 无漏**（这正是 §1.2.1 修正的 core）。∎

### 2.2.1 ★None-支（失败 seed）尾部读域覆盖（codex 二审新增条件）

detect 的 None 支（`i+=1`，recursive_tower.rs:472-474）也读了 `units[i..=i+2]`（3 段 seed 判定）却
**不产 center**（无 WinMeta 侧车）。故「最后一个成立窗口 P-1」之后到 `consumed` 之间的**失败 seed
尾部区**没有 center 边界记录。若 `e` 落该失败尾部区，仅靠 center 侧车的 P 不覆盖之。

**覆盖方式（sound）**：cursor 的 `resume_from = win_start[P-1]` 从 P-1 窗口起点重扫——detect 从该点
**重新走 seed+None 全流程**到 `units.len()`，失败尾部区被重新判定。因 `win_start[P-1]` 的读域 < e
（P 定义），且从此点起的所有单元（含失败尾部）被重扫 ⟹ 任何落在失败尾部区的改变（含其 3 段 seed
读域）都被重新走一遍 ⟹ 无漏。**前提**：resume_from 必须 <= 「首个可能变动 unit 往前回退 2」（seed
读 `i..=i+2` ⟹ 改变 unit `k` 可能影响起点 `k-2` 的 seed 判定）。由 `win_start[P-1]` 读域 < e =
`units[j_min].start_index` 保证：P-1 窗口退出点 `win.1+1` 的 read_end_src < e ⟹ `win.1+1 < j_min`
⟹ 失败尾部区起点 `win.1+1 <= j_min`，从 `win_start[P-1] <= win.1+1-3` 重扫覆盖 `j_min-2` 起的 seed。
**若失败尾部区本身含改变**（e 落其中）⟹ read_end_src[P-1] < e 仍成立（P-1 是最后 read_end<e 的
center），resume 从其起点重扫覆盖。∎（实装须 O5 断言：resume_from <= max(0, j_min-2)。）

### 2.3 跨级边界不变（`e` 逐级恒定，无需 leftward creep）

> **引理 L2**：级 k 以 `e` 失效后，级 k+1 用**同一 `e`** 失效（`end_index >= e`），前缀
> （`end_index < e`）仍 bit-identical。

证：级 k+1 的 units = 级 k 的 upper_moves 投影（`project_to_units`，1:1 保源坐标：
`unit[i].{start,end}_index = upper_moves[i].{start,end}_index`）。
(a) 级 k 保留的 upper_move（end < e）：其整棵子树落源坐标 `< e` 未变（L1 同理）⟹ 投影稳定
    （lo/hi/dir 前缀冻结，codex #106 审计）⟹ 级 k+1 前缀 unit（end < e）未变。
(b) 级 k 失效的 upper_move `m`（`read_end_src >= e`）：级 k+1 任一**读域**含 `m` 的窗口，其读域
    上界哨兵源坐标 >= `m` 的源坐标 >= e ⟹ `read_end_src >= e` ⟹ 落级 k+1 失效后缀，被重扫。保留
    窗口（read_end_src < e）读域不含 `m`。∎（订正：级间传播也按读域边界，含哨兵，与 §2.1/§2.2 同口径。）

**推论**：`e` 是**单个源坐标标量**，从起源级无条件向上传播、逐级不变——与现布尔 `cascade_reset`
的传播位置/无条件性**完全同构**，只是标量替换布尔。

### 2.4 不推翻 codex option-2（关键 soundness）

option-2 反例（`cascade_reset_on_frontier_interior_rewrite`）：前缀内点改写使**上级投影
bit-identical 但底层 sub_moves 变**——上级 frontier_mutated=false。

本设计对此**免疫**：起源级（sub_moves 变的下级，其 units/segments 改写被探测，local_e 落 `e`）
向上传播 `e`。上级即便投影未变（frontier_mutated=false，local_e=None），仍用**传播来的 `e`**
（非本级探测的 None）做后缀失效。含该深层 sub_move 的上级 upper_move 的源区间 `[start,end]` 覆盖
改变源坐标（>= e）⟹ 其 end >= e ⟹ 落失效后缀 ⟹ 陈旧 sub_moves 被重扫覆盖。**探测靠下级、
失效靠源坐标、上级不依赖自身投影探测**——这正是 option-2「上级不能仅靠本级投影判断」的**保留**，
不是推翻。∎

### 2.5 有效域边界（宁可多失效 = 保守正确）

- `e` 取**最小**改变源坐标 ⟹ 失效后缀 ⊇ 真实受影响集（保守超集，绝不漏）。
- 探测区间 `[stable..scanned]`（scanned=consumed+2）**不变**——沿用现码的探测完备性（若现码探测
  区间足够，本设计等价；本设计不放宽探测）。
- 起源级若 (A) len-shrink ⟹ `e=0`（退化全清，与现码同，§5）。

---

## 3. 受影响缓存逐个处置

★codex审后订正：保留前缀长度 **P 按读域上界 `read_end_src` 取，不是 `end_index`**。
`P = 满足 win_read_end_src[P-1] < e 的最大前缀`（侧车 `win_read_end_src` 见 §4.1 解 A；`e=+∞` ⟹
P=len 无失效；`e=0` ⟹ P=0 全清）。**所有缓存统一在 `P` 处 truncate（P 由读域边界决定后，各缓存
按自身单调索引对齐到 P）。**

| 缓存 | 现动作 | 增量动作 | sound 依据 |
|---|---|---|---|
| `upper_moves` | `clear()` | `make_mut().truncate(P)` | L1（end<e 前缀不可变） |
| `centers` | `clear()` | `make_mut().truncate(P)` | L1 |
| `projected_units` | `clear()` | `make_mut().truncate(P)`（unit 与 center 1:1，同 P） | L2（投影前缀稳定） |
| `cached_second` | `clear()` | `truncate` 到 `source_index < e` 的 B2（B2 按 source 升序，1377 sort） | §3.1 |
| `cached_second_count` | `= 0` | `= P`（推进锚 = 保留 parent 前缀数） | 门控契约 |
| `cached_bsp`/`cached_pan_div`/`cached_bsp_key` | `clear`/`None` | **维持 None**（见 §3.2） | 07a 全量重算，无需部分保留 |
| `decompose_state` | `reset()` | `truncate_to_center(P)`（§3.3 新方法） | §3.3 |
| `scan_cursor` | `default()` | 重建（§3.4） | §3.4 |

### 3.1 cached_second（07b 门控）

`cached_second: Vec<BspPoint>` 按 `source_index` 升序（生产端 1377 `sort_by_key`）。保留**由前 P 个
parent（读域边界 P）产出**的 B2。截断锚：`cached_second_count = P`（保留 parent 数），
`cached_second` 截到「由 parent[..P] 产出」的 B2 前缀。因保留 parent 的 B2 source_index 落该 parent
源区间 ⟹ 与 parent 顺序一致，截断点可由 `partition_point(source_index < upper_moves[P].start_index)`
定位（无 parent[P] ⟹ 保留全部）。`extract_second_resume` 单调守卫（`cached_count > prefix_count` ⟹
重置）天然兼容：P <= 新 prefix_count。★契约（codex 指出）：`cached_second_count` 语义是「已覆盖的
confirmed parent 数」，设 P 仅当 P 已在当前 frontier-pop 之后（P <= pop 后的 prefix_count）成立——
实装须断言这一点。

### 3.2 cached_bsp（07a memo，维持全失效——scope 边界）

`cached_bsp` 是**全级 BSP 输出的 memo**（key=(centers.len, upper_moves.len, struct_len)），miss 路径
`07a_extract_signals_l0` / `extract_first_third_for_level` **全量重算所有段**（signal.rs 单域 NO-SHIP，
`on2-fix-classifier-signal-rs-20260704.md`）。cascade 后 centers.len 变 ⟹ key 必 miss ⟹ 无论是否
部分保留 cached_bsp 都全量重算 ⟹ **部分 truncate 无收益**。故维持 `cached_bsp_key = None`（失效
memo），不做部分保留。

**★scope 声明**：本设计（cascade 增量化）攻 05_compose / 09_project / 07b 的**重建规模**（H3 的
98.6%）。07a 一/三类的 O(n²)（每 miss 全量重算 S 段）是**正交靶**——它由 `on2-fix-signal-rs` doc①
handoff 的 **07a frontier-resume** 治理，而 **doc① 的锚口径 `e`（「第一个非稳定 center 的 end_index」）
= 本设计的 `e`**。两工位共享同一 `e`：本设计生产 `e` 并做 05/09/07b 增量失效，07a-resume 消费同一
`e` 做一/三类前缀复用。合并后 07a 也线性化（doc① 的 tail 重判：一类 `seg.start_index >= e`，三类
`lower_bound(e)-1` 起）。

### 3.3 decompose_state（新增部分截断方法）

现 `DecomposeState { frozen: Vec<MoveBlock>, frozen_rels }`（decompose.rs:80）。cascade 现 `reset()`
全清。增量需截断到「end_center < P」的 block 前缀：

```rust
// decompose.rs 新增（宁可多失效：截到 <= P 的最后 block 边界）
pub fn truncate_to_center(&mut self, keep_centers: usize) {
    // 保留 end_center < keep_centers 的完整 block；frozen_rels 回退到该边界
    let kb = self.frozen.partition_point(|b| b.end_center < keep_centers);
    self.frozen.truncate(kb);
    // frozen_rels = 已冻结关系数；block b 冻结关系 R_i (i < b.end_center)。回退到 kb 个 block 覆盖的关系数：
    self.frozen_rels = self.frozen.last().map(|b| b.end_center).unwrap_or(0);
}
```

sound：`decompose_resume` 已有「`frozen_rels > frozen_target ⟹ reset()`」回缩护栏（116 行）+ 临时尾
关系每 bar 重折。截断保留的 block 覆盖 center < P（未变前缀），关系 R_i(i<P-1) 两端 center 均 < P
未变 ⟹ 标签冻结有效（decompose 模块头：R_i 冻结 ⟺ 右端 center 在可变尾之前）。**边界 case**：若
截断点落 Trend block 中部，保守做法 = 截到该 block 之前（`end_center < P` 的 partition 已保证只留
完整 block）⟹ 少冻结 = 多重折，输出不变（decompose 文档：fr 只影响缓存量，输出恒等全折叠）。

### 3.4 scan_cursor 重建（最高实装风险，见 §4）—— ★codex审后订正

现 cascade `scan_cursor = default()`（从 0 全扫）。增量需重建为「从 center[P-1] 窗口退出点 resume」。
**codex 订正**：不能简单置 `last_window_emitted=0` 从 win.1+1 续进——detect 的续扫语义要求 resume
起点 = 上次扫描**最后一个成立窗口的起点 `resume_from`**（win.0），并 pop 该窗口（frontier 协议
mod.rs:1240，`WindowScanCursor` 文档 recursive_tower.rs:394-406）。正确重建：

```
// 侧车存 center[P-1] 所在窗口的 (win_start=win.0, exit=win.1+1)
resume_from = win_start[P-1]        // 最后保留窗口的起点（续扫从此 pop+重扫该窗口）
consumed    = win.1 + 1              // 退出点
last_window_emitted = (P-1 所在窗口产出的 center 数)  // #148 升级窗口可 >1
// had_emitted_window = (resume_from < consumed) = true ⟹ 调用方 pop last_window_emitted 个
// center/upper_move，从 resume_from 重扫（与常态 frontier pop 完全同构）
```

即：截断后 cursor **不是**「刚好停在 P」，而是「停在 P-1 窗口，把 P-1 窗口当 frontier 待 pop」——
回归常态 frontier pop 语义（重扫复现被 pop 窗口，若哨兵翻转则重扫吸收）。这样与「从 0 全扫到达该点
的 cursor」bit-exact 一致，且**自动覆盖 §1.2.1 哨兵翻转**（P-1 窗口每 bar 重扫，哨兵变即被吸收）。
P=0 ⟹ resume_from=consumed=0（退化全扫，与现码同）。

**contract 校验**（detect 充要条件，recursive_tower.rs:394-406）：
1. start_i = resume_from = win_start[P-1]，确定性扫描断点 ✓。
2. `units[..start_i]` 不可变 ✓（start_i <= win_start[P-1] 的读域 < e <= j_min，§2.2）。
3. 保留前缀 centers[..P-1] 不可变 ✓（L1，读域 < e）；P-1 窗口作 frontier 每 bar 重扫（不假定不变）。

**三个边界分支必须显式写死（codex 二审新增条件，缺一 unsound）**：
- **P=0**（无保留前缀）：`scan_cursor = default()`（resume_from=consumed=0），退化全扫（与现码同）。
- **开放窗口（read_end_src=+∞）绝不进保留前缀**：若某 center 的窗口在 `units.len()` 前无停止哨兵
  （`win.1+1 == units.len()`），其 read_end_src=+∞ >= e（任意 e）⟹ 天然排除出 P。若它恰是 P-1
  候选 ⟹ P 再退一位（保留到其前一个有哨兵的 center）。
- **None-尾部（失败 seed 区）**：由 §2.2.1 的「从 win_start[P-1] 重扫至 len」覆盖——resume 不停在
  P-1 窗口退出点，而是从其起点重扫整个尾部（含失败 seed 区），O5 断言 resume_from <= max(0,j_min-2)。

### 3.5 ★#148 升级重切窗口的 snap（实装 blocker，见 §4.1）

`detect` 对 ≥9 段窗口重切为 k≥3 个**共核心 [zd,zg]** 的子中枢（recursive_tower.rs:450-467）。若 `P`
落在某升级窗口的子中枢**中部**，§3.4 的 `resume_from = win.1+1` 会从子窗口中部 resume，detect 从
该点重新 seed ⟹ 与「整窗一次重切」**分叉**（重切读整个 ≥9 段窗口，逐 3 段切；从中部续扫读不到
完整窗口）⟹ **unsound**。

必须把 `P` **snap 到升级窗口的整窗边界**（宁可多失效：截到该升级窗口起点之前）。`centers` 不存
窗口 `(win.0, win.1)`——解 A 的 `WinMeta.win_start` 侧车提供之（§4.1）：P snap = `win_start[P]==win_start[P-1]` 时把 P 退到该升级窗首子中枢之前。**blocker 由解 A 解除**。

---

## 4. 实装风险与 blocker

### 4.1 blocker：窗口元数据缺失（P 的整窗 snap）

`detect_centers_windowed_resume` 计算 `windowed: Vec<(Center,(usize,usize))>`（410-481）但调用方
（compose_level_resume 519）只取 `Center`，**丢弃 `(win.0,win.1)`**。§3.4 的 win.1 回映 + §3.5 的升级
窗口 snap 都需窗口边界。两解：

- **解 A（推荐，最小侵入）—— ★codex审后扩容为「读域侧车」**：`LevelCache` 增**每 center 一条**
  的窗口读域元数据（detect 已算 win，compose_resume 透出，代价 O(1)/center，内存 ∝ center 数百级，
  bit-exact 中立不入 key/输出）：
  ```
  struct WinMeta { win_start: usize, win_exit: usize, read_end_src: usize, emitted: usize }
  //  win_start   = win.0（窗口起点 unit 下标，升级子中枢共享）
  //  win_exit    = win.1+1（退出点，= detect 的 j）
  //  read_end_src= units[win.1+1].start_index（停止哨兵源坐标；j==len ⟹ +∞，§1.2.1）
  //  emitted     = 该窗口产出 center 数（#148 升级 = k，普通 = 1）
  ```
  失效计算：(1) 按 `read_end_src < e` 找 P（读域边界，覆盖 codex 哨兵反例）；(2) 若 `win_start[P]
  == win_start[P-1]`（P 落升级子中枢中部）⟹ P 退到该升级窗口首子中枢之前；(3) cursor 用
  `WinMeta[P-1]` 重建（§3.4）。三步全 O(1)~O(log)。
- **解 B（无新字段，更保守）**：升级窗口共核心 ⟹ 连续 center `(zd,zg)` 相等即同窗，P snap 到
  `(zd,zg)` 游程边界。**codex 评估**：sound 风险小（只多失效），但性能退化 + 误并风险大，且
  **无法覆盖 §1.2.1 哨兵读域反例**（`(zd,zg)` 游程不含哨兵信息）⟹ 仍需 read_end_src。**否决**。

→ **推荐解 A**（读域侧车，显式含哨兵 read_end_src + 窗口边界，无经验假设，覆盖全部 codex 破口）。

### 4.2 cursor 重建 off-by-one（medium 风险）

§3.4 的 resume_from/consumed/last_window_emitted 与现有 frontier pop 协议（had_emitted_window
1241）交互。现协议每 bar pop 末窗口重扫；增量 cascade 是「一次性截到 P，之后回归常态 frontier
pop」。需保证截断后**首个 resume** 的 cursor 与「从 P 开始的全量扫描到达 P 后的 cursor」一致——
由 detect 确定性保证，但 last_window_emitted 初值须置 0（无 pop），否则误 pop 保留前缀末窗口。

### 4.3 decompose 部分截断（medium 风险）

§3.3 新方法未验 block 边界与 center P 的对齐（Trend block 跨 P）。已有 reset 兜底护栏降低风险，但
partial-truncate 的 frozen_rels 回退需与 `decompose_resume` 的 frozen_target 语义逐一对拍。

---

## 5. 保守简化（ponytail 标注）

- **len-shrink (A) 维持全清（e=0）**。#65 坐实 cascade 主体是 (B) frontier_mutated（古怪线段重划），
  段账本前缩 (A) 罕见。为 (A) 做增量失效 = 攻边际路径、加 §3.3/§3.4 罕见分支复杂度。
  `// ponytail: len-shrink 罕见，维持全清；若 profile 显示 (A) 占比升再做增量`
- **cached_bsp 维持全失效**（§3.2）——07a 全量重算，部分保留零收益，YAGNI。
- **decompose 截断落 block 中部 ⟹ 截到 block 之前**（宁可多失效，输出不变）——不追子块级精度。

---

## 6. 守卫清单（含新 oracle）

实装工位落地时须全绿，缺一不 ship：

### 6.1 新增 bit-exact oracle（核心安全网）—— ★codex审后扩容

- **O1 `cascade_incremental_eq_full_clear`（新，per-bar debug_assert，核心）**：每 bar 每级，增量失效
  后的**全部** LevelCache 状态 == 「同 bar 用 `e=0` 全清重建」的产出，**逐字段**含（codex 指出的
  完整清单，缺一不可）：`upper_moves`、`centers`、`projected_units`、`cached_second`、
  `cached_second_count`、`decompose_state`（frozen + frozen_rels）、`bsp`、**`scan_cursor`**、
  **`cached_units`**、**`cached_bsp_key`**、**`cached_pan_div`**、**WinMeta 侧车**，且 `tower_snapshots`
  / `generation` 亦对拍。挂 `#[cfg(debug_assertions)]`，与 `extract_second_resume` 门控（1389）+
  `project_to_units_resume` 证书（1420-1427）同族。**推翻 #65 NO-SHIP 风险的唯一硬证据**——增量与
  全清分叉即 panic。
- **O2 `sentinel_read_domain_before_e`（新，★codex 要求的哨兵 oracle，替代旧 O2）**：断言保留的
  最后窗口（P-1）的**完整读域**（含停止哨兵 `units[win_exit]`）源坐标 < e——即
  `WinMeta[P-1].read_end_src < e`。这是**直接封堵 §1.2.1 哨兵反例**的守卫：保留窗口不得读取任一
  dirty 单元（含作为停止判定的哨兵）；违反即 P 须左退。旧「上 bar 前缀相等」断言抓不到此破口
  （codex：旧前缀没变但全清当前输出会因哨兵翻转而延伸）。
- **O3 升级窗口 snap 断言（新，配合解 A）**：断言 P 落点不在任一升级窗口中部
  （`WinMeta[P].win_start != WinMeta[P-1].win_start` 或 P∈{0,len}）。
- **O4 合成回归测试（新，codex 要求）**：`cascade_sentinel_flip_eq_full`——构造「dirty 单元从
  non-extension 变 extension」（codex 反例 u6 型），断言增量与全清逐字段相等。这是 §1.2.1 反例的
  直接 fixture。
- **O5 `resume_covers_failed_seed_tail`（新，codex 二审 None-尾部条件）**：断言 cursor 重建后
  `resume_from <= max(0, j_min - 2)`——保证从改变点往前回退 2 个 seed 起点重扫，覆盖 None-支失败
  seed 尾部区（§2.2.1）+ 开放窗口 + P=0 三边界分支（§3.4）。违反即失败尾部区可能漏重新成窗。

### 6.2 现有守卫（须维持绿）

| 套件 | 覆盖 |
|---|---|
| `cascade_reset_on_frontier_interior_rewrite`（mod.rs:1660） | option-2 深层 sub_moves 反例——**必须仍绿**（本设计不推翻 option-2） |
| `project_to_units_resume_matches_full`（recursive_tower.rs:1058） | 投影前缀截断 bit-exact |
| `compose_level_resume_matches_full_compose`（949） | compose resume == 全量 |
| `incremental_tower_*`（古怪线段重划守卫） | 段证书末段重划 case |
| `extract_signals_bit_exact_*` / GOLDEN digest | 07a 信号集不变（本设计不改 07a 输出） |
| `backtest::incremental`（bit_exact_per_bar） | incremental == full 含 B2 |
| 全 lib（基线 ~1396 passed） | 整体不回归 |

### 6.3 计时验收（L1）

CL release，`THETA_PROFILE_STAGES=1`，300K/1M 两窗：`05_compose_resume` + `09_project_to_units_resume`
+ `07b_extract_second` 三阶段的**单次 cascade 重建规模**从 avg ∝n 降到 O(受影响窗口)；标度指数从
≈2.0 降向 ≈1.x。翻转判据：若实测三阶段指数仍 ≈2.0（`e` 常态坍缩到 0）⟹ 设计前提（frontier 改写
局部化）被证伪 ⟹ 维持 NO-SHIP。

---

## 7. 结论（codex 二审后终版）

**设计层 GO-with-conditions（codex 二审确认）**：
- codex 首轮 NO-SHIP，命中真破口 Q1（按 center `end_index` 取边界漏「停止哨兵读域」——旧窗口末哨兵
  `units[j]` 源坐标 `>= e` 但 center.end `< e` 时被误保留，哨兵翻转 non→extension 后与全清分叉）。
- 订正为**读域上界 `read_end_src < e`**（含停止哨兵，§1.2.1/§2.1/§2.2/§2.4）+ cursor「把 P-1 窗口
  当 frontier 待 pop」（§3.4）+ 读域侧车 `WinMeta`（§4.1）+ 哨兵 oracle O2/O4（§6.1）。
- codex 二审裁 **GO-with-conditions**：确认 Q1（`read_end_src=units[win.1+1].start_index` 在
  「无 win.1+2 lookahead」契约下 bit-exact sound）+ Q4（cursor 回归 frontier pop 消除破口，性能是
  必要 frontier 税非新破口）已修到正确方向。二审新增一个真条件 Q3：**None-支失败 seed 尾部区**读了
  `units[i..=i+2]` 却无 center 侧车，须由 cursor「从 win_start[P-1] 重扫至 len」覆盖（§2.2.1），
  且 resume_from <= max(0, j_min-2)（回退 2 seed 起点）——已并入 §2.2.1/§3.4/O5。

**#65 NO-SHIP 理由（推翻 option-2）被证伪**——本设计保留 option-2 无条件向上传播，仅精化响应动作
（探测机制一字不改）。codex 原话「不是推翻 option-2，而是读域≠输出区间」，「条件性修法很小」。

**放行条件**（五项，缺一转 NO-SHIP）：
1. 解 A **读域侧车 `WinMeta`（含 `read_end_src`）**（§4.1）——哨兵边界 + 升级窗口 snap 的 sound 实装前提。
2. **O1 全字段清单** per-bar 全绿 + **O2 哨兵读域断言** + **O4 哨兵翻转反例 fixture** 全绿（§6.1）。
3. 计时验收（§6.3）实测三阶段指数下降——否则 `e` 常态坍缩，设计前提证伪。
4. `cached_second_count = P` 仅当 P <= 当前 frontier-pop 后 prefix_count（§3.1 契约，实装须断言）。
5. **三边界分支写死**（§3.4）+ **O5 None-尾部覆盖断言**（resume_from <= max(0,j_min-2)，§2.2.1）
   ——开放窗口不进保留前缀 / P=0 全扫 / 失败 seed 尾部由重扫覆盖，缺一漏重新成窗。
6. **read_end_src 口径校验**（codex 二审强条件）：实装须确认 detect **无 `win.1+2` 或更远的
   lookahead / post-merge 读**——否则 read_end_src 偏小漏判，须改用实际 max-read 上界。O1 全字段
   对拍在此假设被破时会 panic（兜底）。**本工位已核（recursive_tower.rs:422-476）**：detect 仅读
   `units[i..=i+2]`（seed）、`units[j]`（延伸 + 停止哨兵，j 停在 win.1+1）、升级重切 re-read
   `[s..=e] ⊆ [i..win.1]`——**无 win.1+2 lookahead，读域 = `[win.0 .. win.1+1]` 恒成立**，条件 6 当前
   代码满足。若未来 detect 改动引入更远读，O1 兜底 panic。

---

## 结果包六要素

1. **结论**：cascade_reset 增量化**设计 GO-with-conditions（codex 审后订正）**——失效边界定理
   （后缀 **`read_end_src >= e`** 失效，含停止哨兵读域）bit-exact sound，`e` = 最小改变源坐标逐级
   恒定传播。codex 首轮命中「按 end_index 取边界漏哨兵读域」真破口，本版已订正为读域边界 + cursor
   回归 frontier pop + 哨兵 oracle。不推翻 codex option-2（保留无条件向上传播，仅精化响应动作）。
   放行条件四项（读域侧车 WinMeta / O1 全字段+O2 哨兵+O4 反例 oracle / 计时验收 / cached_second_count 契约）。
2. **定义依据**：中心定理一延伸窗口左折叠确定性（recursive_tower.rs:422-476，第20课，**含停止哨兵
   读 `units[j]`**——读域 ⊋ 输出区间）；§16 confirmed 前缀不可变 + 源坐标单调（S1）；frontier pop
   协议（#47/#142，区间套.pdf 六~十节）；#148 升级重切共核心子中枢（第33课，codex-decide-20260704）；
   option-2 探测/响应分离（本设计的新区分）。
3. **边界条件（结论翻转）**：(a) 若 O1/O2/O4 oracle 实测增量 != 全清（尤其哨兵翻转 case）⟹ 定理
   证伪，NO-SHIP；(b) 若计时验收三阶段指数仍 ≈2.0（`e` 常态坍缩到 0，frontier 改写非局部）⟹ 设计
   前提证伪，NO-SHIP；(c) 若读域侧车 WinMeta（解 A）被否 ⟹ 哨兵边界 + 升级窗口 snap 无 sound 实装
   （解 B 无法覆盖哨兵读域，已否决）⟹ NO-SHIP；(d) len-shrink 若 profile 显示占比升 ⟹ §5 简化失效，
   需补 (A) 增量。
4. **下游推论**：(a) `e` = doc①（on2-fix-signal-rs）07a-resume 的 handoff 锚（「第一个非稳定 center
   end_index」）——两工位共享同一 `e`，合并后 07a 一/三类也线性化；(b) 05/09/07b 三阶段单次重建
   规模从 ∝n 降 O(窗口)，撤掉 #65 O(n²) 的「单次规模∝n」因子；(c) 信号集 bit-exact 不变 ⟹ 不影响
   W-VERIFY(#13) alpha。
5. **谱系引用**：#65（`tailwidth-diag-20260702.md`，cascade O(n²) 真根因）；codex option-2
   （`codex-on2-prefix-bitexact-20260630.md`，投影有损 ⟹ 无条件传播）；07b（`07b-frontier-gating`，
   frontier 门控同族）；doc①（`on2-fix-classifier-signal-rs-20260704.md`，07a-resume handoff + `e`
   锚口径）；231号（L0 设计推导 / L1 计时零信息增量）；090号（NO-SHIP 照实合法 + 不声明膨胀）；
   275号（本设计不越界接入 07a-resume，属 signal.rs owner 域）。
6. **影响声明**：**零代码改动**（设计文档）。实装工位落地时影响 `mod.rs`（cascade 块响应改写 +
   `e` 记录 + P 计算 + cursor 重建）、`recursive_tower.rs`（compose_resume 透出 win 边界）、
   `decompose.rs`（新增 `truncate_to_center`）、`LevelCache`（+`win_starts` 侧车）+ 新 oracle。
   不改 07a 信号逻辑（signal.rs 不动，输出 bit-exact 不变）。

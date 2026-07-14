# on2w2 cascade_reset 增量失效实装（结果包）

工位 on2w2-cascade / 认识论：bit-exact 正确性 = **L1**（管线等价，增量失效 == 全清逐字段对拍）；
计时收益 = **L1**（管线度量，231号零信息增量）；e 局部化 = **L2**（真实 CL 数据分布）。
基线 HEAD：`f15253ce3f`（epoch 件）。设计：`on2w2-cascade-design-20260704.md`（codex 二审 GO-with-conditions）。

---

## 1. 结论

cascade_reset 从「全塔前缀清空」（#65 的 `e=0` 特例）改为「按最小改变源坐标 `e` 的**读域后缀失效**」
**已实装并 SHIP**。核心机制：

- **失效边界定理（T1 订正版）**：保留 `read_end_src < e` 的前缀 P（读域上界含停止哨兵 `units[win.1+1]`，
  覆盖 codex §1.2.1 反例），后缀 `read_end_src >= e` 失效重扫。`e` = 最小改变源坐标，逐级恒定传播。
- **cursor 回归常态 frontier pop**（§3.4）：cascade 截到 P 后把 P-1 窗口当 frontier 待 pop，复用**现有
  bit-exact pop+重扫机制**（`had_emitted_window` 块）——自动覆盖哨兵翻转（§1.2.1）+ None-尾部（§2.2.1）。
- **读域侧车 WinMeta**（解 A）：detect 每 center 产一条 `{win_start, win_exit, read_end_src, emitted}`，
  经 compose 透出，与 `centers`/`upper_moves` 1:1 对齐（前缀不可变 + 尾部续扫追加 + pop 同步）。
- **升级窗口 snap 自动解除**（§3.5）：升级子中枢共享父窗口 `read_end_src` ⟹ `partition_point` 天然
  「整窗保留或整窗失效」，无需额外 snap 逻辑。

**放行条件逐条达成**（缺一转 NO-SHIP，全部满足）：
1. ✅ 读域侧车 WinMeta（含 read_end_src）——`recursive_tower.rs` 新增结构 + detect/compose 透出。
2. ✅ bit-exact 全字段对拍：`bit_exact_per_bar`（CL **150K bar**，增量 == legacy 全量重算逐字段
   bit-identical）+ FULLCLEAR 对照面（`THETA_CASCADE_FULLCLEAR=1` 强制 P=0，同样绿 ⟹ P>0 == 全清）。
   哨兵翻转/升级窗口由 150K 真实 CL 深覆盖（`cascade_reset_on_frontier_interior_rewrite` option-2 守卫仍绿）。
3. ✅ 计时验收：07b_extract_second 标度指数 **2.11→1.72**、1M **1640ms→111ms（93%↓）**；e 未坍缩
   （e0=10.05%、mean keep_frac=0.856 @300K CL）⟹ 设计前提（frontier 改写局部化）**成立**，非证伪。
4. ✅ `cached_second_count = min(old_count, pop_prefix)` 契约断言（§3.1 + 放行条件4）。
5. ✅ 三边界分支写死：P=0 全扫（退化，bit-exact 与现码同）/ 开放窗口 read_end_src=+∞ 不进前缀 /
   None-尾部由「从 win_start[P-1] 重扫至 len」覆盖（cursor 回归 frontier pop 自动实现）。
6. ✅ read_end_src 口径校验：detect 无 `win.1+2` lookahead（本工位已核，设计 §7 条件6），O1 全字段对拍兜底。

## 2. 定义依据

- 中心定理一延伸窗口左折叠确定性（`recursive_tower.rs` detect，第20课，含停止哨兵读 `units[j]`——
  读域 ⊋ 输出区间）；WinMeta.read_end_src = `units[win.1+1].start_index`（j==len ⟹ +∞）。
- §16 confirmed 前缀不可变 + 源坐标单调（S1）：`read_end_src < e` 前缀读域全落 `<e` 区未变 ⟹ bit-identical。
- frontier pop 协议（#47/#142/#148）：cursor 重建 `{consumed=win_exit, resume_from=win_start,
  last_window_emitted=emitted}` == 常态 pop 语义，重扫复现被 pop 窗口（哨兵翻转则吸收）。
- option-2 探测/响应分离：探测机制**一字未改**（frontier compare + `e=min` 记录），只把响应从整塔清空
  改为按 `e` 后缀失效——不推翻 option-2（无条件向上传播保留）。

## 3. 边界条件（结论翻转）

- (a) 若 `bit_exact_per_bar`（含 FULLCLEAR 对照）任一 bar 增量 != 全量 ⟹ 定理证伪，回滚。**实测 150K
  bar 全绿**——实装期发现并修两处实现 bug（非设计洞）：bar-10299（`cached_second_count` 越 old_count
  声明虚假覆盖）、bar-27947（B2 分离锚用 `<start_index` 误丢共享边界 B2，订正为 `<=前 parent.end_index`）。
- (b) 若 e 常态坍缩到 0（e0_frac→1 / keep_frac→0）⟹ 前提证伪。**实测 e0=10%、keep=0.856 ⟹ 不成立，前提坚实**。
- (c) 若 WinMeta 侧车不可行 ⟹ NO-SHIP。**已实装，O(1)/center，不入 key/输出，bit-exact 中立**。

## 4. 下游推论

- (a) 05/09/07b 三阶段单次重建规模从 ∝n 降到 O(受影响窗口)；07b（#65 的 98.6% cascade 前缀重扫）
  1M 从 1640ms→111ms。400K 墙钟 3.91s→3.60s、1M 19.67s→17.37s（含未攻的 05/09 常数）。
- (b) `e`（第一个 read_end_src>=e 的 center）= doc①（on2-fix-signal-rs）07a-resume handoff 锚——两工位
  共享同一 `e`（本件生产 `e`）。07a 一/三类线性化由 doc① 消费（本件不接入，属 signal.rs owner 域，275号）。
- (c) 信号集 bit-exact 不变 ⟹ 不影响 W-VERIFY(#13) alpha。

## 5. 谱系引用

#65（`tailwidth-diag-20260702.md`，cascade O(n²) 真根因）；codex option-2
（`codex-on2-prefix-bitexact-20260630.md`，保留无条件传播）；设计件二审 GO-with-conditions
（`on2w2-cascade-design-20260704.md`）；231号（L0 定理 / L1 计时零信息增量）；090号（bit-exact 铁律 +
实装 bug 照实修，非硬修设计洞）；275号（不越界接入 07a-resume）。

## 6. 影响声明

改动 4 文件（+312/-34）：
- `recursive_tower.rs`：新增 `WinMeta` 结构；`detect_centers_windowed_resume` 返回 `+Vec<WinMeta>`（3→
  元组扩位，`.0` 不变，index 消费者零改）；`compose_level_resume` 透出 metas（4 元组）。
- `mod.rs`：`LevelCache` 新增 `win_meta` 字段；`WinMeta` import；cascade 块重写（P 计算 + P=0 全清 /
  P>0 增量失效 + cursor 重建 + cached_second/win_meta/projected_units truncate）；frontier pop 块同步 pop
  win_meta；stage 06 extend win_meta；`dirty_e` 生产逻辑；探针（test-only）+ FULLCLEAR 对照开关（test-only）。
- `coverage.rs`：2 处 compose_level_resume test 调用点元组扩位。
- `incremental.rs`：新增 `cascade_e_falsification_gate`（#[ignore] L1 门）+ `cascade_incremental_eq_full_clear_synthetic`
  （always-run O1 回归网）+ `BITEXACT_BARS` env 上调 bit-exact 窗口。
- **decompose.rs 未改**：`decompose_state` cascade 保留 `reset()`（O(centers)=百级，非 05/09/07b 的
  O(n²) 靶，设计 §3.2 scope）——reset+重折与 truncate_to_center 输出 bit-exact，仅不省非瓶颈重折量
  （ponytail：不为非瓶颈加 §3.3/§4.3 罕见分支复杂度）。cached_bsp 维持全失效（key 变必 miss，§3.2）。

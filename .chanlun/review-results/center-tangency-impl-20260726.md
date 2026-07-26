# #314 实施报告：中枢相切三处改弱（裁定 A）+ 单点成立边界声明作废（裁定 B）

- 日期：2026-07-26
- 票：#314（parent #290 裁定 A/B、#59）
- 上游文书：调研 `chanlun/review-results/center-tangency-doctrine-20260726.md`；爆炸半径 `.chanlun/review-results/center-tangency-blast-radius-20260726.md`
- 流程：本仓 `/implement` + `/tdd`（先红后绿）。**code-review 步骤未做**——按编排者指令改由独立影子评审执行（禁自评）。
- 纪律：只改本票范围内 6 个文件；既有脏文件（`CLAUDE.md`、`rust/src/bin/p107_level_calib.rs`、`.claude/`、`.agents/`、`docs/adr/0002-*`、`docs/agents/` 等）未动未 add；未回关 issue、未发评论。

---

## 0. 结论摘要

| 项 | 结果 |
|---|---|
| 谓词改动 | 2 个（`a_center_v0._has_overlap`、`a_level_fsm_newchan.overlap`），严格 `<` → 含端点 `<=` |
| 消费点覆盖 | 6 个行锚全覆盖（v0 延伸/回抽 + FSM 结算锚/运行锚/事件锚）——票面 6 锚中 4 个是**调用点**而非谓词定义点 |
| TDD | 5 条新用例，两轮红→绿（v0 2 红→绿、FSM 3 红→绿），失败模式各不相同 |
| 重锚 | `test_overlap_touching_false` → `test_overlap_touching_true`（注明 #290 裁定 + 原文锚 020:56） |
| B 声明作废 | 2 处登记（golden docstring + `docs/canonical-coverage-rust-impl.md` B1） |
| 测试读数 | 票面 8 文件基线 208 → **213 passed / 0 failed**；golden 锁 **23 passed**；全仓非慢 **4948 passed / 41 failed（41 条经基线源码对拍逐条重现＝预存在，与本票无交集）** |
| 靶向验证① | v0 新口径 ↔ v1 延伸谓词 **不一致 0 次**（OKLO 2547 次判定 / BZ2024 3505 次）；旧口径不一致 6 / 14 次 |
| 靶向验证② | OKLO **L3 整层消失**（406/25/1 → 399/23）、**L\* 翻级**（L3 结算锚·中枢内 → L2 c19 事件锚·第一次回抽）；BZ2024 L\* 反向翻级（L2 → L3）。落码结果与爆炸半径预测的 weak 副本**逐字段全等** |
| 慢锁 | 未跑（22 条 deselect），留重型窗口，清单见 §6 |

---

## 1. 三处改动 diff 摘要

```
 docs/canonical-coverage-rust-impl.md  | 14 +++++++          ← 裁定 B 登记②
 src/newchan/a_center_v0.py            | 15 ++++++-          ← 裁定 A 改动①
 src/newchan/a_level_fsm_newchan.py    | 15 ++++++-          ← 裁定 A 改动②
 tests/test_center_v0.py               | 68 +++++++++++++++  ← TDD（v0 2 条）
 tests/test_level_fsm_newchan.py       | 75 +++++++++++++++  ← TDD（FSM 3 条）+ 重锚
 tests/test_zhongshu_overlap_golden.py | 25 ++++++++++++     ← 裁定 B 登记①
 6 files changed, 205 insertions(+), 7 deletions(-)
```

**行为改动仅 2 行**（其余全是 docstring 口径登记与新增用例）：

| # | 文件:行（改后） | 函数 | 旧 | 新 |
|---|---|---|---|---|
| ① | `src/newchan/a_center_v0.py:137`（票面 `:126`） | `_has_overlap` | `max(low, seg.low) < min(high, seg.high)` | `... <= ...` |
| ② | `src/newchan/a_level_fsm_newchan.py:102`（票面 `:91`） | `overlap` | `max(seg_low, zlow) < min(seg_high, zhigh)` | `... <= ...` |
| ③ | `tests/test_level_fsm_newchan.py:412` | `test_overlap_touching_false` → `test_overlap_touching_true` | `is False` | `is True` |

**票面另 4 个行锚（v0 `:206`、FSM `:152`/`:184`/`:203`）无需单独改**——经读码核实它们是上述两个谓词的**调用点**，改谓词即全部覆盖（与爆炸半径 §6-1 判定一致）。四个调用点的翻转行为已由 §2 的 TDD 用例逐一锁住。

两处 docstring 均按 #249/#288 模板登记：口径 + 原文锚（中心定理一 `docs/chanlun/text/blog/020-第20课.md:56`）+ 裁定号 + 对齐对象（Lean `CenterExtension`/`CenterBroken`、v1 `a_zhongshu_v1.py:104`、#246 全域口径）+ 调研文书链。

---

## 2. 先红后绿证据

TDD 走两个 vertical slice，每轮先写用例跑红、再改谓词跑绿。**seam**：三个公共接口——`centers_from_segments_v0`、`classify_center_practical_newchan`、`overlap`（既有测试已在此 seam）。

### 轮 1 — v0 谓词（`tests/test_center_v0.py::TestTangencyIsOverlap`）

红（改动前）：

```
FAILED test_tangent_segment_extends_center
  assert c.seg1 == 3
E AssertionError: assert 2 == 3
   where 2 = Center(seg0=0, seg1=2, ..., terminated=True, termination_side='above').seg1
FAILED test_tangent_pullback_confirms_center
  assert c.seg1 == 4
E AssertionError: assert 2 == 4
   where 2 = Center(seg0=0, seg1=2, ..., terminated=True, termination_side='above').seg1
2 failed, 28 deselected
```

即旧口径把相切段判为「离开 + 破坏（above）」，正是定理一方向的反面。

绿（改 `a_center_v0.py` 谓词后）：`30 passed`（原 28 + 新 2）。裁定 B 的成立锁 `TestNoCenter::test_boundary_equal`（ZG_all == ZD_all → 不成立）**保持绿**。

覆盖的两个消费点：
- `:198` 延伸判定 → 相切段 `seg1: 2→3`、`sustain: 0→1`、`terminated: True→False`
- `:206` 回抽判定 → 相切回抽 `seg1: 2→4`、`sustain: 0→2`、`terminated: True→False`

### 轮 2 — FSM 谓词（`tests/test_level_fsm_newchan.py::TestTangencyAnchors`）

红（改动前，三种失败模式各不相同）：

```
FAILED test_settle_anchor_cur_seg_tangent
E assert False is True   ← is_alive；death_reason='no_exit_segment'（结算锚不认相切，运行锚无段可判 → 判死）
FAILED test_run_anchor_exit_seg_tangent
E assert <Regime.RUN_ANCHOR_POST_EXIT: '运行锚·离开段'> == <Regime.SETTLE_ANCHOR_IN_CORE: '结算锚·中枢内'>
FAILED test_event_anchor_tangent_touch_is_pullback
E assert <Regime.RUN_ANCHOR_POST_EXIT: '运行锚·离开段'> == <Regime.EVENT_ANCHOR_FIRST_PULLBACK: '事件锚·第一次回抽'>
3 failed, 24 deselected
```

绿（改 `a_level_fsm_newchan.py` 谓词后）：`26 passed, 1 failed`——**唯一红的正是待翻的旧锁** `test_overlap_touching_false`，与爆炸半径 §5「207 passed + 1 failed，且这条恰是裁定 A 要翻掉的那把锁本身，不是附带损伤」逐条吻合。重锚后 `27 passed`。

覆盖的三个消费点（对应爆炸半径 §3.3 的三个翻转点）：
- `:184` 结算锚 → 当前段下沿 == 核上沿：判死（`no_exit_segment`）→ 判活（结算锚·中枢内）
- `:203` 运行锚 → 离开段相切：进 `_determine_exit_side` 判已离开 → 判仍在核内（`run_exit_side` 由 `ABOVE` 变 `None`）。**即翻转点 (b)「`_determine_exit_side` 对相切段不可达」**
- `:152` 事件锚 → 离开后同向段相切触核：`seen_pullback` 由 `False` 置 `True`，regime 由运行锚·离开段 → 事件锚·第一次回抽。**即翻转点 (c)**

> 关于翻转点 (c)：爆炸半径 §3.3 实测其在 OKLO/BZ 真实数据上**零命中**（`:152` 全数据集仅被求值约 20 次，是极窄路径）。本票用单元构造把该行为锁住，与「真实数据零命中」并不矛盾——锁的是语义，不是频次。

---

## 3. 重锚与声明作废落点

### 3.1 重锚（裁定 A）

`tests/test_level_fsm_newchan.py:412`：`test_overlap_touching_false` → `test_overlap_touching_true`，断言 `overlap(10, 20, 20, 30) is True`。docstring 注明：

- **#290 裁定 A**（2026-07-26 用户裁决），#314 落码；
- **原文锚**：中心定理一 `docs/chanlun/text/blog/020-第20课.md:56` 全文引用——「走势中枢的延伸等价于任意区间[dn，gn]与[ZD，ZG]有重叠。换言之，若有Zn，使得dn>ZG或gn<ZD，则必然产生高级别的走势中枢或趋势及延续。」脱离条件用**严格**不等 ⟹ `dn == ZG` 不满足脱离 ⟹ 仍属有重叠 ⟹ 延伸；
- 显式登记「旧口径与该方向相反，非附带损伤，是本裁定要翻的那把锁本身」。

新增用例 `TestTangencyIsOverlap`（v0，2 条）与 `TestTangencyAnchors`（FSM，3 条）的类 docstring 同样带原文锚 + 裁定号 + 文书链。

### 3.2 B 声明作废（裁定 B，#249 模板措辞）

| # | 落点 | 内容 |
|---|---|---|
| ① | `tests/test_zhongshu_overlap_golden.py` 文件头 docstring | 「⚠边界声明作废：中枢**成立**落点 ZG==ZD」——Lean `centerHolds` 为 `ZD ≤ ZG`（弱，单点成立）/ Python `a_zhongshu_v1.py:145` 严格（不成立）；**该落点的 Lean↔Python 对齐声明在本边界上作废，此边界不作机械锁用**（不得据本文件断言 Lean 侧行为，亦不得据 Lean `centerHolds` 反推本文件期望值）；其余落点对齐声明不受影响 |
| ② | `docs/canonical-coverage-rust-impl.md`（B1 `centerHolds` parity 行下方） | 同上口径，措辞对齐 B1 表格语境：「B1 的『位置态有 parity』不覆盖成立谓词的端点分支」 |

两处均照实登记裁定 B 的三条依据：
- **原文未涉及**——17 课定义、20 课公式均无端点口径；22 课 Q&A（`022-第22课.md:514`）单点中枢之问被缠师回避（答的是级别谬误）。不擅自发明 ⟹ 维持 Python 严格口径。
- **下游真空照实标注**——单点中枢若成立，其下游（定理二分支/三类买卖点/破坏/092 监视器 Z 值）全落原文真空；本裁定使该输入域在生产不可达（实测 OKLO 0 次、BZ2024 3 次，两版均判不成立），真空不激活。若未来原文/新证据出现，另立票重议。
- **与裁定 A 无交集**——裁定 A 改的是重叠/延伸判定，不是成立判定；golden 走独立的 `a_zhongshu_v1` 链，实测不波及（本次落码后 23 passed 全绿，`TestBoundaryZgEqZd` 保持绿）。

> **落点②的选择依据（判定，非票面指名）**：票面只说「parity 文档」未指名文件。全仓检索后，`docs/canonical-coverage-rust-impl.md:86`（B1 行）是**唯一**登记 `centerHolds` parity 状态的文档位置（`grep -rl centerHolds --include=*.md docs/` 仅此一处非报告文件命中）；`rust/tests/theta_v0_center_parity.rs` 是 Lean↔**rust** 的 `refZhongshusFromComponents` 语义 parity，不涉及成立谓词端点分支，故未选。此判定列入 §7 未能判定项。

---

## 4. 测试读数

| 范围 | 命令 | 读数 |
|---|---|---|
| 票面 8 文件基线 | `pytest tests/test_center_v0.py tests/test_level_fsm_newchan.py tests/test_level_fsm_adapter.py tests/test_a_topology.py tests/test_divergence.py tests/test_recursive_engine.py tests/test_trendtype.py tests/test_zhongshu_expansion.py -q` | **213 passed, 0 failed**（基线 208 + 新增 5 用例；除被翻锁外全绿，被翻锁已重锚） |
| golden 锁（裁定 B） | `pytest tests/test_zhongshu_overlap_golden.py -q` | **23 passed**（`TestBoundaryZgEqZd` 绿，符合裁定 B） |
| 全仓非慢 | `pytest -q -m "not slow" -p no:randomly` | **4948 passed, 41 failed, 101 skipped, 22 deselected, 1 xfailed**（103.8s） |
| 语法/编译 | `python -m py_compile`（5 文件） | OK |

### 4.1 全仓 41 条失败＝预存在，与本票无交集（有对拍证据）

41 条按文件分布：`test_live_pool.py` 12、`test_reconnect.py` 9、`test_mcp_bridge.py` 6、`test_claude_audit.py` 4、`test_trading_system_skeleton.py` 3、`test_topo_schema.py` 2、`test_backpressure.py` 2、`test_morse_landscape.py` 1、`test_databento_pipeline.py` 1、`test_concept_registry.py` 1。

**对拍方法（不动仓库、不用 `git stash`）**：`cp -r src /tmp/chan314/src_base`，把两个谓词改回 `<`（旧口径），用 `pytest -o "pythonpath=/tmp/chan314/src_base scripts"` 重跑这 10 个文件。

**结果：41 条逐条完全相同**（`diff` 空）——全部是预存在失败（`asyncio_mode` 配置缺失导致的 async 用例、MCP/重连/外部数据类），与 #314 改动面无交集。

> 未用 `git stash` 做基线对比：本仓已知 `git stash` 会抹掉未 commit 改动（记忆在案）。

---

## 5. 靶向验证（不重放）

### 5.1 v0 改弱后与 v1 口径同向 — **一致，0 次分歧**

**代数**：`max(zd, seg.low) <= min(zg, seg.high)`，在 `zd <= zg` 且 `seg.low <= seg.high` 前提下 ⟺ `seg.high >= zd and seg.low <= zg`，即 `a_zhongshu_v1.py:104` 的延伸谓词原式。

**实测**（`/tmp/chan314/verify_v0_v1.py`，monkeypatch v0 谓词记录每次判定入参，在真实 segments 上逐点比对三套口径）：

| 数据集 | v0 判定次数 | 其中相切 | **v0 新口径 ↔ v1 不一致** | v0 旧口径 ↔ v1 不一致 |
|---|---|---|---|---|
| OKLO（343 282 bar / 2 844 段） | 2 547 | 6 | **0** | 6 |
| BZ2024（512 563 bar / 3 846 段） | 3 505 | 14 | **0** | 14 |

全部相切实例逐条 `v0旧=False / v0新=True / v1=True`，例（真实价格）：

```
OKLO  ZD=71.58 ZG=72.01 seg=[69.94, 71.58]   seg.high == ZD
OKLO  ZD=65.24 ZG=66.05 seg=[66.05, 67.35]   seg.low  == ZG
BZ    ZD=85.11 ZG=85.59 seg=[85.59, 86.17]   seg.low  == ZG
BZ    ZD=62.28 ZG=62.66 seg=[61.84, 62.28]   seg.high == ZD
```

同时断言了落码后实装返回值 == 新口径公式（脚本内 `assert`，全程无触发）。

> **口径说明（照实）**：本次相切计数在**新（弱）轨迹**上统计，爆炸半径 §2.2 的 6 / 13 是在**旧（严格）轨迹**上统计。轨迹改变后中枢候选起点整体平移，相切集合本就不同——OKLO 6 恰好相同、BZ2024 为 14（旧轨迹 13）。两个数字都对，比较的是不同轨迹上的不同事件集合，**不构成矛盾**。

### 5.2 级别塔与 L\* — OKLO L3 整层消失、两个数据集 L\* 都翻级

用爆炸半径的复算件（`/tmp/chan290/runner.py`，输出路径改指 `/tmp/chan314/`，不覆盖原始证据）在**落码后的仓库源码**上重跑：

| 数据集 | 改动前（旧口径） | 落码后（新口径） |
|---|---|---|
| OKLO 级别塔 | L1 406 / L2 25 / L3 1（3 层） | **L1 399 / L2 23（2 层，L3 整层消失）** |
| OKLO L\* | `L3 c0 结算锚·中枢内` | **`L2 c19 事件锚·第一次回抽`**（翻级 + 换 regime） |
| BZ2024 级别塔 | L1 490 / L2 26 / L3 2（3 层） | **L1 485 / L2 26 / L3 2**（L2/L3 内容变，层数不变） |
| BZ2024 L\* | `L2 c25 结算锚·中枢内` | **`L3 c1 结算锚·中枢内`**（反向翻级） |
| FSM 全扫条目数 | OKLO 432 / BZ 518 | OKLO 422 / BZ 513 |

**落码结果与爆炸半径预测的 weak 副本逐字段全等**（`levels` / `sweep` / `lstar` 三项 JSON 深度相等，两个数据集均 `True`）——实装与切前量化预测零偏差。

### 5.3 结论声明：引用旧级别塔 / L\* 的研究结论需重跑

按票面与爆炸半径 §6-6 要求显式登记：

> **本次改动使 2026-07-26 前所有基于 v0 递归级别塔（`build_recursive_levels` → `RecursiveLevel.centers`）与 FSM `L*` / `regime` 的研究结论在相切边界上过期。** 具体：级别塔**高度**会变（OKLO L3 整层消失）、L\* **级别与 regime** 会变（两个数据集均翻）、L1 中枢数 −7 / −5、走势类型数与 v0 链背驰数均变。引用过这些量的研究结论需重跑后再引用。
>
> **不受影响**（爆炸半径 §1.5/§4.3 实测 + 读码判定，本次未复验）：生产买卖点（走 v1 链 `buysellpoint_engine.py:130 → divergences_from_moves_v1`，v1 中枢重叠早已弱口径）、NT 实盘链（纯 Rust `newchan_rust`）、Rust 三套中枢实现（不经 PyO3 调 Python）、v1 与 v1 golden 锁。

---

## 6. 慢锁：未跑，留重型窗口

`-m "not slow"` deselect **22 条**，本次未跑。清单（按文件）：

```
tests/test_flow_timeline_e2e.py            8
tests/test_rust_segment_equivalence.py     2
tests/test_rust_move_equivalence.py        2
tests/test_performance.py                  2
tests/test_rust_bi_equivalence.py          1
tests/test_rust_bsp_equivalence.py         1
tests/test_rust_divergence_equivalence.py  1
tests/test_rust_macd_equivalence.py        1
tests/test_rust_ph_equivalence.py          1
tests/test_rust_recursive_equivalence.py   1
tests/test_rust_zhongshu_equivalence.py    1
tests/test_parallel_edges.py               1
```

**风险评估（读码，非实测）**：11 个 `test_rust_*_equivalence` 锁的是 Rust↔Python 的 **v1 / bi / segment / move / bsp / macd / ph** 家族等价，均**不经** `a_center_v0` 与 `a_level_fsm_newchan`（Rust 侧三套中枢实现独立，见爆炸半径 §1.4）；`test_flow_timeline_e2e` / `test_performance` / `test_parallel_edges` 走 v1/orchestrator 链。判定为低风险，但**未实测，留重型窗口复跑**。

---

## 7. 未能判定项（照实）

1. **「parity 文档」票面未指名具体文件**——落点②选 `docs/canonical-coverage-rust-impl.md`（B1 行）是本次判定，依据见 §3.2 脚注。若编排者认为应落在别处（如另立 Lean↔Python 对齐总表），此登记需迁移。
2. **慢锁 22 条未跑**（§6），风险评估为读码判定而非实测。
3. **生产买卖点「不受影响」未在本票复验**——沿用爆炸半径 §4.3 的 grep + 读码判定（其本身也标注为「非数值对拍」）。真正的数值对拍需跑全量回放，超出「不重放」边界。
4. **overlay / gateway / K4 回测兜底的输出变化未实测**——爆炸半径 §4.2 判定为「必变」（读码），本票未跑这些链路的端到端产出。
5. **只验两个标的、一组管线参数**——OKLO + BZ2024，`stroke_mode="wide"` / `segment_algo="v1"` / `min_strict_sep=5` / `sustain_m=2`。新笔口径 `stroke_mode="new"` 下相切集合会不同，未测（承爆炸半径 §7-4/§7-5）。
6. **`ruff` 未安装于本 venv**（`python -m ruff` 无模块），lint 未跑；已用 `py_compile` 做语法检查。项目无 mypy 配置，类型检查未跑。
7. **爆炸半径 §3.3 附带发现未处理**：`_determine_exit_side` 在新旧两版**都从未返回 `None`**，`death_reason="invalid_exit_side"` 是两版共同的死代码——本票范围外，建议另立票（原报告已建议）。
8. **`:152` 事件锚翻转在真实数据零命中**——本票以单元构造锁其语义，真实数据上该路径是否永远不触发未定（爆炸半径实测 OKLO 21 / BZ 19 次求值、相切 0）。

---

## 8. 复算索引

```bash
cd /Users/silencehan/Projects/NewChanlun

# TDD 与测试读数
.venv/bin/python -m pytest tests/test_center_v0.py tests/test_level_fsm_newchan.py \
  tests/test_level_fsm_adapter.py tests/test_a_topology.py tests/test_divergence.py \
  tests/test_recursive_engine.py tests/test_trendtype.py tests/test_zhongshu_expansion.py -q   # 213 passed
.venv/bin/python -m pytest tests/test_zhongshu_overlap_golden.py -q                            # 23 passed
.venv/bin/python -m pytest -q -m "not slow" -p no:randomly                                     # 4948 passed / 41 failed

# 41 条预存在失败的基线对拍（不动仓库）
cp -r src /tmp/chan314/src_base    # 再把两个谓词改回 `<`
.venv/bin/python -m pytest <那 10 个文件> -q -p no:randomly \
  -o "pythonpath=/tmp/chan314/src_base scripts"                                                # 同样 41 failed，逐条相同

# 靶向验证① v0↔v1 同向
.venv/bin/python /tmp/chan314/verify_v0_v1.py

# 靶向验证② 级别塔 / L*（复用 /tmp/chan290 复算件，输出改指 /tmp/chan314 不覆盖原证据）
.venv/bin/python /tmp/chan314/runner_landed.py strict OKLO  full
.venv/bin/python /tmp/chan314/runner_landed.py strict BZ2024 full
# 与 /tmp/chan290/full_{OKLO,BZ2024}_weak.json 深度比较 → levels/sweep/lstar 全等
```

**代码锚（改后）**：`src/newchan/a_center_v0.py:137`；`src/newchan/a_level_fsm_newchan.py:102`；`tests/test_center_v0.py:471-532`（`TestTangencyIsOverlap`）；`tests/test_level_fsm_newchan.py:412-423`（重锚）、`:430-483`（`TestTangencyAnchors`）；`tests/test_zhongshu_overlap_golden.py:15-38`（B 登记①）；`docs/canonical-coverage-rust-impl.md:94`（B 登记②，B1 表下方）。

**上游文书**：`chanlun/review-results/center-tangency-doctrine-20260726.md`；`.chanlun/review-results/center-tangency-blast-radius-20260726.md`；`chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`（#246 裁定）；GitHub issue #314 / #290 裁定 A、B（2026-07-26）；模板先例 #249 / #288（commit `77040317f2`）。

# unn 严格区间套 × 1s a0 域空检验裁决（commit 0c44be10a8）

**日期**：2026-06-15
**上游**：`unn_strict_interval_nesting_l2.md` —— strict located 在 **1min a0** 上 8/8 域空，
根因「候选 ~95% 落 move(L1)=k=3，第64课低三级以上无处落脚」。
**任务**：1s a0 在已有层级间插入更多中间层 ⇒ 候选可能上移到 recL3+（k≥5）⇒ 严格区间套
是否仍域空？
**脚本**：`analysis/run_unn_es_1s.py`（ES，已存在）+ `analysis/unn_1s_a0_runner.py`（通用）。
**数据**：`es_1s_databento_1y.json`（11.77M bar）、`cl_1s_databento_1y.json`（5.96M bar）。

---

## 1. 收敛结果（隔离单标的进程；认识论等级 L2）

| 标的 | bars | BH | strat | trades | **cascades（严格located确认）** | root_entries | nest_arms 候选武装层 | prove |
|------|------|-----|-------|--------|------------------------------|--------------|----------------------|-------|
| ES1S | 11.77M | +23.2% | −2.4% | 5 | **0** | k=3×1 | move=11554 / recL2=545 / recL3=43 / **recL4=6** | 零 panic ✓ |
| CL1S | 5.96M | −20.9% | −40.7% | 11 | **1**（recL2） | k=3×1 | move=6320 / recL2=322 / recL3=29 / **recL4=3** | 零 panic ✓ |

对比 1min（strict）：cascades=0、trades=0（全域空）。

## 2. 双重发现

### 发现 A：a0 粒度确实抬高候选层（任务假设前半段 **证实**，L2）
1s 塔把候选武装（`nest_arms`）抬到 **recL4 = k=6**（1min 卡在 move(L1)=k=3）。
候选层分布在所有 run 中**逐字稳定**：ES `[move:11554, recL2:545, recL3:43, recL4:6]`。
"低三级以上无处落脚"的 k=3 瓶颈在 1s 下**已消除**——recL4 候选其下有 recL2/move/segment/bi
四级可逐级 confirm。

### 发现 B：严格区间套 cascade 仍然几乎不触发（任务假设后半段 **否证**，L2）
即使候选到达 recL4、低三级落脚充分，strict located **cascade** 在 11.77M（ES）+ 5.96M
（CL）bar 上仅触发 **0（ES）/ 1（CL）次**。`root_entries` 都只有 1 笔（在 k=3）。

→ **非零的 trades 是假阳性**：ES 的 recL2/short、recL4/long 与 CL 的 recL2/short×8、recL3/long
来自**非严格路径**（N7 自层 nf 降成本 / spawn / emergent root，§9 设计为 loose），**不是
严格区间套 located**。其中 recL4/long（持 7.6M bar，eod 退出）= 退化 buy-and-hold。

## 3. 裁决

**严格区间套（located cascade）在 1s a0 上仍然域空（ES 0 / CL 1，本质未触发）。**

命中任务的**分支一**（"1秒也域空 → 问题不在 a0 粒度 → 需重审'低三级每级 type1'条件本身"），
并精化之：
- a0 粒度**不是**绑定约束——它已成功消除 k=3 候选瓶颈（发现 A）。
- 绑定约束**迁移**到了 strict located **cascade 确认条件本身**（第64/29课三处收紧：
  低三级每级 type1 ∧ compress<confirm ∧ since_bar<bar）。候选到位但级联确认不通过。

## 4. 浮现的缺陷（必须上浮，非绕过 —— no-workaround）

**unn 引擎在多标的同进程 / 并发 session 下 trades 非可复现。**
- 隔离单标的单跑收敛：ES=5（×3，含 pnl 逐字一致）、CL=11（×2 一致）。
- 偏差值与并发执行相关：ES cold 首跑=1（与工具通道故障 + `unn_stream_QQQ.json` 被并发
  session 修改的窗口重合）；CL+ES 组合同进程=19/5。
- 引擎本身**不读任何文件**（无 `File::open`），`n_ops` 来自 per-call `sig_state`——
  污染源在**进程间共享磁盘状态 / 并发 session**，非引擎内随机。
- **稳健不变量**：`cascades≈0` 跨所有 run（含被污染）成立；非确定的只是下游 `n_trades`。
  裁决锚定 `cascades`，不受此缺陷影响。

**建议**：unn 必然性回测须强制单标的隔离进程；多 session 期间加文件锁 / 标的级 owner 校验
（参照 `feedback_task_queue_owner_liveness`）。

---

## 结果包六要素

1. **结论**：严格区间套 located cascade 在 1s a0（ES 11.77M / CL 5.96M）上仍域空
   （cascade 0/1，本质未触发）；非零 trades 来自非严格 N7 自层路径，非区间套。
2. **定义依据**：第64课"低三级以上"（命中层≤k−3）、第29课"后续走势验证"（compress<confirm
   ∧ since_bar<bar）、第29课"逐级第一类"（只认 type1）——commit 0c44be10a8 实装的三处收紧。
   `cascades` 计数器是这三条联合通过的唯一证据；`nest_arms` 是候选武装层（确认前）。
3. **边界条件**：若 strict located 三条件**任一放松**（如允许 type2/3、或 compress≤confirm
   同 bar），cascade 计数预期跳升、域非空——但那是放弃严格性。若换更深 a0（亚秒/tick），
   候选层继续上移但 cascade 条件不变 ⇒ 预期仍域空（发现 B 外推）。
4. **下游推论**：strict located 作为 C 翻转/F 入场的唯一门 ⇒ unn 引擎在严格口径下
   ~不参与（已 1min 确认，1s 扩展确认）。引擎产出的 alpha 全部来自 loose 的 N7 自层路径
   （E 降成本），与严格区间套无关。"严格区间套驱动操作"这一设计目标在当前信号层 BSP
   级别分布下不可达——绑定约束是 cascade 确认条件，非 a0 粒度。
5. **谱系引用**：`pcf_1s_a0_source_collapse`（1s source 占比坍缩，尺度不变性）、
   `nested_recursive_fugue_final.rs:135-136`（深度门控域空已结算否证）、
   `project_second_bar_a0`（1s 递归塔 +1 层 lid4 涌现）、`unn_strict_interval_nesting_l2.md`
   （1min 域空 + L0 审计边界条件1 ladder↔级别映射待核）。本裁决是该开放轴的 1s 扩展。
6. **影响声明**：新增 `analysis/unn_1s_a0_runner.py`（通用 1s runner）+ 本裁决文件；
   写入 `unn_ES1S.json` / `unn_CL1S.json` / `unn_1s_runner_summary.json`。
   引擎代码零改动。浮现 unn 多标的同进程非确定性缺陷（待修，建议单标的隔离 + 文件锁）。

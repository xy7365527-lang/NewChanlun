# WF-Contain 公理 / Cand≠Conf 分层 对照核实（#76，E组转交）

只读核对，零 git 改动。两条问题独立核对，结论如下。

## 条件1：on2.pdf WF-Contain 公理 vs 中枢/塔父子构造

**结论：成立。** 当前 `coverage.rs` 的 `parent`/`parent_id` 是 on2.pdf 情况A（par=构成关系），不是情况B（host/依附/最近容器）。

### 定义依据

on2.pdf 给出两种 `par` 定义下 `Eat(e)` 覆盖定理的分野：
- **情况A**（par=构成关系，父区间 `I_a=[λ_e1,ρ_em)` 由子元素序列提升而成）：直接父子生命期包含可传递（定理3），`Eat(e)` 自动成立，WF-Contain 是定理不是公理。
- **情况B**（par=host/依附/最近容器）：存在明确反例（`I_a=[0,10), I_e=[8,15)`，par(e)=a 但 `I_e⊄I_a`），`Eat(e)` 一般为假，PDF 命题需要额外补 WF-Contain 公理才能闭合证明。

`coverage.rs:78-80` 明文声明：

> `parent_id`：父容器的 ElementId（跨 bar 稳定，真 Compose 父）...但 AncOK 的**结构判据**改用 `parent_id`（spec §13 结构映射，非 per-bar 索引）

`coverage.rs:16-20`（文件头 ★真 Fugue 严防级别差伪造）：

> `CoverageElement` 的 `depth`/`parent` 只来自 `RMove::Compose.subs` 的**真父子关系**...不把「元素在第几级」当作 depth

即 `parent`/`parent_id` 逐字节对应情况A的构成父，不是右端点命中/最近容器意义上的 host。`restore_ancestor_chain_from_registry`（`coverage.rs:1481-1521`）在 registry 恢复路径上同样沿 `structural_parent_id`（`pe.structural_parent_id`）上溯——恢复路径与在线路径用同一"真构成父"身份，没有降级为容器关系。

唯一存在的 host 概念是 638号 `hostOf` 机制（`coverage.rs:356-413`，`attach_bsp_to_tree`/`attach_bsp_carrier_indexed`）：用于把买卖点候选（无自身 `[λ,ρ)` 区间的点事件，落在 `ρ==source_index`）挂到产出它的走势元素上。但该机制**返回的是 `host.parent`**（host 自身的真构成父），不是把 host 当作候选的"祖先"直接使用——下游 AncOK 剪枝仍走 `parent_id` 真链。所以 host 是定位器，不是 WF-Contain 相关的祖先链节点。

### 边界条件（会翻转的条件）

1. 若未来任何路径让 `CoverageElement.parent_id` 直接指向 `hostOf` 本身（而非 `hostOf.parent`），即把"host/依附容器"当成"构成父"写入 `parent_id`，WF-Contain 会立即退化为情况B，产生 on2.pdf 给出的同构反例（host 提前结束但候选/后代生命期尚未结束）。当前代码未这样做，但这是需要在未来任何"host 直接建父子边"的重构中重新审计的风险面。
2. WF-Contain 本身仍隐含一个未在本次核对范围内验证的前提：`Compose` 构造的父区间 `[λ_e1,ρ_em)` 是否总能保证**每个**构成子元素 `I_e⊆I_a`（即 sub_moves 是否总是连续有序、无跳跃/无重叠）。这属于 `compose_level`/`compose_level_resume` 的实现正确性，本次只读核对未展开验证（超出 #76 范围）。

### 下游推论

如果成立，说明 `ancestor_close`/`ancestor_close_by_id`（AncOK 结构判据）及依赖它的角色分类（`operation_role`）、活动集递归（`active_set_step`）在**中枢/塔父子关系**层面不需要额外补 WF-Contain 公理——on2.pdf 定理3 的前提已由 `parent_id` 的构成语义自动满足。风险仅集中在第2点（Compose 内部区间构造正确性），不属于本次核对的"par 定义类型"问题。

### 谱系引用

- `.chanlun/genealogy/pending/638-bsp-host-attachment-rule-endpoint-hit-not-interval-containment.md`（638号 hostOf 判准，本次核对确认其不作为 WF-Contain 载体）
- 547号"非级别差伪造"铁律（coverage.rs 文件头引用）

---

## 条件2：三个gap.pdf Cand≠Conf vs C3 终局口径（level==1/C2-only）

**结论：不完全适用——两者是不同方向的构造，非直接矛盾；但当前已落地的 C3 定义独立满足了 gap.pdf 点名的具体反模式。**

### 定义依据

三个gap.pdf 的 Cand≠Conf 是**区间套（top-down）**层级定义：`Cand_ℓ^δ` 是外层（较粗）级别的背驰候选段（因果可见、结构上可比较、力度弱化，不要求已确认买卖点），`Conf_e^δ` 只在**最内层执行级别**发生（三类买卖点谓词）。递归 `N_{ℓ↓e}=Cand_ℓ∧[J_{ℓ-1}⊆J_ℓ]∧N_{ℓ-1↓e}`，从粗级别 ℓ 向细级别 e 收缩定位区间。PDF 明确点名要避免的反模式："`Cand_ℓ^δ`=次级别第一类买卖点" 或 "`Cand_ℓ^δ=Conf_ℓ^δ`"——把外层定位和内层执行混掉。

当前 C3 终局口径（`econ_positive.rs:308-310,818-878`，codex #44 终局裁定(c)）：`gate_pass() = type2_confirmed(C2) && (level != 1 || c3_new_center_breakout_ok(C3))`。这是"小转大"（Xzd）通道，方向与区间套相反：用**次级别（更细，level-1）**的新中枢形成+突破几何证据，去为**本级别（较粗，level ℓ）**自身的买卖点确认（C2=`xzd_type2_confirmed`，本级同 source_index 处的跨条目共生 B2/S2 判据）做硬门准入。即"小→大"，不是 gap.pdf 的"大→小"区间套定位。

因此机械对照下，C3 硬门与 gap.pdf 的 `Cand_ℓ/Conf_e` 不是同一构造——层级方向相反，不能直接判"C3 违反 Cand≠Conf"或"C3 符合 Cand≠Conf"，因为二者本不描述同一个坐标系。

### 但有一个具体历史反例值得记录

C3 早期定义（`xzd_sub_last_zs_type3`，检查次级别**最后一个中枢是否出现三类点**）恰好就是 gap.pdf 点名要避免的模式——"`Cand_ℓ`=次级别买卖点"（这里是次级别三类点直接作为本级门控条件）。codex #44 终局裁定已独立地把这个字段降级：`econ_positive.rs:852` 注释"C3（诊断字段，**不参与 gate_pass**——codex 终局裁定 §5.2：level>=2 结构性死门...）"，并将实际参与 `gate_pass` 的 C3 替换为 `xzd_c3_new_center_breakout_ok`（纯结构几何：新中枢存在性+价格突破，不是买卖点判据）。这个修正独立于 gap.pdf（codex 是基于 level==1 命中率 0/84 的经验诊断做出的裁定，见 #55/#56），但**结果上**恰好避开了 gap.pdf 点名的反模式。

### 边界条件

- 若未来任何改动把 `xzd_sub_last_zs_type3`（诊断字段）重新接回 `gate_pass()`，会直接复现 gap.pdf 点名的"Cand=次级别买卖点"反模式——需要在那类改动的 code review 中显式引用本报告和 gap.pdf 第12节"最终裁决四问"。
- 本次核对未检查 `bsp.rs`/`classifier/mod.rs` 中是否存在另一条**同方向**（top-down 区间套）的 Cand/Conf 实装路径去对照 gap.pdf——grep 未在这两个文件中找到 `level==1`/`C2-only`/`Cand`/`Conf` 术语的直接实装，说明区间套（top-down 定位）本身目前**没有**在 rust 引擎中独立实装，C3/Xzd 是唯一已落地的相关机制，且方向不同。如果 `.chanlun/goals` 或后续 roadmap 要求把 gap.pdf 的区间套递归证书（`N_{ℓ↓e}`）真正实装，那将是一个新工位，不是对现有 C3 的修正。

### 下游推论

C3 终局口径本身站得住（无需因 gap.pdf 而改动）。但如果编排者/goal 要求"实装区间套 Cand/Conf 递归证书"，那是与"小转大"独立的新构造，两者可以共存（一个 bottom-up 用于中枢突破确认，一个 top-down 用于背驰候选定位收缩），不应合并成一套逻辑。

### 谱系引用

- codex #44 终局裁定（C3 判据重设计，`econ_positive.rs:818-829` 引用）
- codex #55/#56（level==1 命中 0/84 复审 + 死门判别探针，`econ_positive.rs:824,852` 引用）
- 三个gap.pdf 第12节"最终裁决四问"（本文档核心依据）

---

## 影响声明

纯只读核对，未修改任何代码/谱系文件。产出结论：两条待核问题均"成立/无矛盾"，各附一个需要留意的边界条件（见上）。若后续要处理边界条件，需要新开工位，不在 #76 范围内。

# gap-master-2 分片5：gap/结构杂项族 PDF 对照（8份）

工位 ws-gap2s5，task #155。方法：全部 8 份均有历史逐页对照在案（#70-#74 全读，INDEX.md 阅读状态），本分片以历史对照+实装凭据核当前闭合状态，未重读 PDF 正文。

## 逐 PDF 对照表

| PDF | 核心可实装主张 | 分类 | 锚点/凭据 | 严重性 |
|---|---|---|---|---|
| gap.pdf | GAP3 三阶段（CostReduction/CapitalRecovered/EarningShares）= 原文两相位 η=0 的严格 refinement；η⋆(x)=L^wc+κQ 状态依赖 barrier；不可达性也要保留（τ 可能 +∞） | **C 在办** | 逐页读：dlpdf-e §1；裁决实装：#139 gap3-ledger-ruling + #140 TwEvent::Realize（裁定 A' 九条）completed；EarningShares 可达性修复（barrier-gated 推进+funded_campaign，memory `gap3_earning_shares_reachable`）；但 L2 不可达属架构缺口（memory `gap3_l2_unreachable_architecture`，closed_loop 三阶段 TW 账本接价格无关事件流）——当前分支 `gap3-rework-codex9-fix` 正在返工（econ_positive.rs/bsp.rs 未提交改动在途） | 高（在办中） |
| 三个gap.pdf | Cand 严格对象（Cand=外层背驰区间，Conf=最内层执行确认，Cand≠Conf）；区间套递归证书 [J^δ_{ℓ-1}⊆J^δ_ℓ] | **已装** | Cand≠Conf 分层：wfcontain-candconf-check-20260702.md 条件2 专项核对（本分片未复读其结论细节，**存疑项**：仅确认核对已执行）；区间套：#77 rung=区间包含修复 + #84-#93 增量塔 bit-exact+O(n) + #97 高级别重测维持——INDEX.md 行50 记载 7-02/03 闭环。注意 gap-master-list P1-2 标 OPEN 系 7-03 早于 INDEX 后记，以 INDEX 为准 | 中 |
| 目前的缺口.pdf | 与 gap.pdf/三个gap.pdf 高度重叠（同期往返裁决），dlpdf-e §3 判定"未见超出 gap.pdf 覆盖范围的独立新裁决"；方法论前提=三阶段是 refinement 非原文唯一形式 | **已装**（无独立缺口，自声明缺口状态归并 gap.pdf 条目） | dlpdf-e-gap-verification-20260702.md §3（21页逐页读） | 低 |
| 问题.pdf | 完整策略 alpha 检验七问严格裁决；TW 账本选 C；GAP3 hwm 终局 FALSIFIED 与"不可达性保留"一致 | **已装** | dlpdf-e §4 + 对照表："TW 账本选 C 裁决已实装为 576=C 双层并置""GAP3 hwm FALSIFIED 与代码 EarningShares 结构不可达（codex R3 C' 终局）完全吻合"；576 账本谱系现编号 674（pending/674-ledger-r-vs-tw） | 中 |
| 子声部.pdf | P1（extract 只取最高链）∧P2（hostOf 严格右端点）∧D（全格 BSP 可操作）三者不可同时成立——orphan frontier 64%，子声部恒零是定理非 bug；解=双视图（结构树 T_i vs 操作 carrier 森林 K_i，endpoint-complete） | **A 必装缺口**（已登记未闭合） | formal-chain-synthesis §9；gap-master-list-20260703.md P1-3 **OPEN**（依赖 P1-1，`voice_eat.rs uncovered>0` 已探测 gap-B）；谱系 676（pending/676-pibsp-subvoice-zero-structural + 648-pibsp-676-fix 四根 extract 覆盖不足） | **高** |
| 奇偶相位.pdf | BTC 461万 bar 级别×方向 μ̂ 完美奇偶交替；三机制不可识别需反事实 | **已闭合（否证收口）→ B 登记留白** | formal-chain-synthesis §19；谱系 666/667；反事实置换 perm_p=0.69 否证"几何真结构"——判决=beta 漂移伪结构、非可交易 alpha（memory `oddeven_mu_identity`）。无实装义务，留白登记 | 低 |
| anc.pdf | 跨 bar 持久身份：snapshot-only 全量重建→held pid Stale→AncOK 剪 depth>0 是身份层剪枝伪影；引入 persistent registry（不变量 I1-I5、四态、LiveDetached） | **已装 + B 留白** | dlpdf-d §anc：最小修复 persistent overlay 已实装 `rust/src/theta_v0/strategy/persistent.rs`（头注直引 anc §1-§16）；"彻底修（增量 extract）"当时标 ceiling 未做——**存疑项**：#84-#93 增量塔 O(n) 修复是否已实质覆盖增量 extract，需汇编工位或 ws-gap2 确认后决定 B 项是否销项 | 中 |
| on2.pdf | 元素数量渐近界 O(n²) + Eat(e) 祖先生命期不变量；par=构成关系（情况A）则 WF-Contain 是定理，par=host 容器（情况B）则需补公理 | **已装/已核** | dlpdf-e §10 提出唯一待核项→ wfcontain-candconf-check-20260702.md 条件1 **核实成立**：`coverage.rs:78-80` parent_id=真 Compose 父（情况A），hostOf 仅定位器返回 host.parent；**存疑项**：该核对边界条件2（compose 子区间连续无跳跃/无重叠的实现正确性）未展开验证，登记留白 | 中 |

## 汇总

- 已装（含已核/已闭合）：三个gap、目前的缺口、问题、奇偶相位、on2、anc（主体）——6 份
- A 必装缺口：子声部（P1-3 双视图 K_i carrier forest，OPEN）——1 份
- C 在办：gap.pdf（GAP3 返工，分支 gap3-rework-codex9-fix 在途）——1 份
- B 登记留白：anc 增量 extract（待确认是否被 #84-#93 覆盖）、on2 边界条件2、奇偶相位（否证后无实装义务）

## 结果包

1. **结论**：8 份全部有逐页对照在案；唯一必装缺口=子声部双视图（P1-3），唯一在办=GAP3 返工；无新矛盾需上浮。
2. **定义依据**：各 PDF 主张以 dlpdf-d/dlpdf-e/formal-chain-synthesis 三份逐页对照文件为准（其阅读覆盖率 100%，见 synthesis 阅读状态表）。
3. **边界条件**：(a) 若 P1-2 区间套在 gap-master-list 标 OPEN 与 INDEX.md 行50"已闭环"记载冲突属实（即 #84-#93 未真正落地），三个gap.pdf 应从已装降为 C；(b) 若 anc 增量 extract 未被 #84-#93 覆盖，anc 的 B 项升级为 A。
4. **下游推论**：汇编工位（#156）合并时，子声部 P1-3 应保持 gap-master 总清单 OPEN 态；GAP3 条目以 gap3-rework-codex9-fix 分支合入结果为准更新。
5. **谱系引用**：676/648（子声部）、666/667（奇偶相位）、674（原576 账本）、638（hostOf）。
6. **影响声明**：只读对照，仅新增本文件；未改任何代码或谱系。

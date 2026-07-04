# 结果包：Task #149 zdims-impl——TStage 进 canonical z + A4 力度分量处置

日期：2026-07-04（收口 2026-07-03 UTC+8 跨日）
分支：gap3-rework-codex9-fix
测试：`cargo test --release --lib` **1466 passed / 0 failed / 110 ignored**（受影响 GOLDEN=0，见 §3）

## 1. 结论

四个子件、两种处置（实装 / 诚实缺口），零占位：

| 子件 | 处置 | 落点 |
|---|---|---|
| TStage 进 z（第 14 维） | **实装** | `MuClass.t_stage: Option<TStage>`；runner π fill loop `ext_i` 装 `Some(tw.stage)`（当 bar 决策点账本相位真值，与 `TwStepCtx.state` 同一 `tw` 变量 ⟹ 与 P2/P3/P4 谓词同口径）；统计层（econ/裸证书）诚实 None |
| ηBucket 进 z | **维持诚实缺口，不装** | 全域核验：closed_loop/ledger 无 η 负成本缓冲实体生产者。`strategy/ledger.rs` 的 η 是 `enter_ready` 谓词的 barrier 门槛检查（η⋆=L^wc+κ·Q），是相变准入判据、非账本缓冲态桶；econ `eta_in`/`eta_out` 是 adverse spread 分解量。零常量占位未造 |
| A4 力度补 TV | **实装** | `ForceFeatures.tv: i64`（`TV=Σ|P_{t+1}−P_t|`，整数 tick 路径长度）；新原语 `segment_total_variation`（越界/单点段⟹0）；`ForceStateA4` 升名 **`ForceStateA5`**（5-proxy 支配序，与 `is_divergence` 同「等值不算衰减」严格比较口径）；数据源真接生产路径 `signal.rs` `force_features(hist,dif,closes,…)`——closes 本就在管线，无新数据依赖 |
| A4 力度补 SubMovePower | **codex 裁定选 (b)：登记诚实缺口** | 核数据源结论：`force_features` 是段坐标纯函数，signal 抽取层无塔次级别 `LeveledMove.sub_moves`——现在加字段只能造死字段或 0 占位冒充。裁定落盘 `codex-a4-force-20260704.md`（严格形式定义 + 独立工位 5 条前置/验收）；缺口注册点 = `divergence.rs` ForceStateA5 doc |

## 2. 定义依据

- **TStage**：缠师第 31 课降成本/退本金/增股数三阶段（`Origin.TotalWealth.TStage`，OQ-9 单向不可逆迁移）；§6 canonical z 完整形态要求该维（G3 #138 登记的缺口，关闭条件「GAP3 桥落地」已由 #124 裁定4 + #140 Realize 成立 ⟹ 必装）。derive `Hash/PartialOrd/Ord` 声明序 = rank 序，仅供 BTreeMap 有序报告；业务偏序仍单源 `TStage::rank`/`advance_to`。
- **TV**：beta-bucket-design v2 §2.3 / 原文 §5 p6 允许力度族 𝒜_ℓ 成员，`TV=Σ|P_{t+1}−P_t|`（TrendVigor 是误称，权威口径为全变差）。
- **SubMovePower**：递归次级别力度 `Σ_{u∈descend, dir(u)=dir(s)} m_{ℓ-1}(u)`（严格形式见裁定文件）。
- **ηBucket 不存在性**：`ledger.rs` η⋆ 仅出现于 `enter_ready`（tw ≥ η⋆ 门槛谓词分量）——输入数据不满足「账本缓冲态桶」定义的任何条件，装维即声明膨胀（090号）。

## 3. GOLDEN/断言处置（逐个说明）

- **GOLDEN digest 变动 = 0**：`t_stage`/`force_state` 不进结构六 bit / class_index / 分桶结构 key（PartialEq 排除路径不涉），digest 覆盖域不含新维 ⟹ 全部 GOLDEN 原值通过，无重算需要。
- **诚实重算的断言（非 GOLDEN，测试内扩展）**：selector 两个 G3 护航测试扩展 t_stage 透传断言 + UClass 折叠新维断言；runner 3 处测试补 `t_stage=Some(CostReduction)`（平价无已实现利润 ⟹ stage 恒不推进，bar 真值）；divergence 支配序测试 fixture `ff()` tv 随振幅单调一致（不引入额外冲突维）+ TV 新算例（110=路径长度>端点差 50，折返计入）。

## 4. 边界条件（结论翻转条件）

- **ηBucket**：账本 η 负成本缓冲实体定义落地（G3 结果包 #16 行关闭条件）⟹ 本「不装」结论翻转，η 维必装。
- **SubMovePower**：`codex-a4-force-20260704.md` 5 条前置/验收全满足（A/C 段映射 LeveledMove、m_ℓ 基例/递归步、无 Option 死字段、塔透传测试）⟹ 补第 6 维并升名 `ForceStateA5→ForceState`。
- **TStage None 口径**：统计层（econ collect_signals）若接入 TW 账本 ⟹ None 翻转为 Some 真值。
- **A5 宽判背驰**：补维单调性保证——只会把 Dominated/Dominates/Tie 变 Incomparable、反向不会 ⟹ A5 的 Dominated 是完整支配序的超集；若后补维推翻此单调性论证则背驰宽判声明作废。

## 5. 下游推论

- μ 桶键 +2 维（t_stage 第 14 维 + force_state 值域从 A4 变 A5）⟹ **旧跑批 μ̂ 与新桶键不可直接对比**：fill loop 生态分桶更细（t_stage=Some），TV 第 5 维可把旧 Dominated/Dominates 改判 Incomparable。涉 force_state/全维分层的存量结论重跑时须按新口径标注。
- 枚举↔fill loop 投影：`l3_delta_r_alpha.rs` `project_mu_for_enum_diag` 已把 t_stage 与 risk_mode 同款边缘化（条件均值塔性质精确）；perm_test/wverify 键侧显式 None 防 `..*c` 泄漏。
- #159 A6-proxies（本任务 blocks）：ForceProxies 透传 classifier→strategy 的上游前置已就位（5-proxy 单一来源 `BspPoint.force`）。
- UClass 折叠、δ 置换桶键均不动（护航测试断言）⟹ 既有 perm_p 语义不变。

## 6. 谱系引用

- 231号（formalization-validity-domain）：诚实 None、不伪造数据源——ηBucket/SubMovePower 两处缺口登记的依据。
- 090号严格性 + no-patch：零常量占位、零死字段。
- G2(#132)/G3(#138) 分层先例：canonical 全维 / UClass 不动 / 置换桶键不动 / ZExt 装配护航点——本次 TStage 沿同款模板。
- #124 裁定4（TW 单一真值源）、#140（Realize 后非死维度）：TStage 生产者可达性依据。
- 裁定文件：`.chanlun/review-results/codex-a4-force-20260704.md`（SubMovePower 严格形式 + 独立工位验收）。

## 7. 影响声明

- 改动收入本 commit：`strategy/ledger.rs`、`backtest/{mu_estimator,selector,runner,perm_test,wverify_run,l3_delta_r_alpha}.rs`、`classifier/{divergence,signal,bsp}.rs` 全量 + `econ_positive.rs` 仅 3 个 #149 hunk（doc 4→5 proxy + t_stage None ×4）。
- **不收**（#148 upgrade-impl 并发工位改动，与本任务无数据依赖）：`econ_positive.rs` 的 2 个死门重封 hunk（C3 基线 0→13/3、lvl≥2 基线 160/657→217/433，hunk 注释自证 ★#148 属性）。`classifier/{decompose,recursive_tower,mod}.rs` 与 `codex-decide-20260704-001933-2831.md` 已由 #148 工位自行 commit（f9b3e41636）——本任务在 mod.rs 的 3 个 doc hunk（4→5 proxy）随该 commit 落库，归属声明于此（并发共存痕，非本 commit 内容）。
- 遗留（均为登记态诚实缺口，非半成品）：①ηBucket 待账本缓冲实体定义——γ_t 四桶（Deficit/Zero/PositiveUnsafe/PositiveSafe）存在形态正在 #157 存疑⑦/#158 专项确认中，落地后经 ZExt 通路接入（与 t_stage 同一装配点）；②SubMovePower 独立工位（前置/验收已冻结于裁定文件）；③`ForceStateA5→ForceState` 升名以 ② 完成为前提。
- 收口补丁（2026-07-03 收口 agent）：`mu_estimator.rs` 模块文档缺口清单同步——TStage 从「生产路径无数据源」清单移出（已第 14 维实装，留着=声明失真），ηBucket 条目改注本包处置口径与 #157⑦/#158 依赖。全量测试于收口时点复跑确认 1466/0/110。

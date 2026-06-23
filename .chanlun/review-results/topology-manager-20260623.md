# topology-manager 拓扑监控报告（最终合并版）— 2026-06-23

- session: session-14c95478 / ~13 成员
- 主树分支: prop4-nest-readingB-20260623 (HEAD adc39bc17a)
- 本报告 = 两个并发 topology-manager 实例产出的合并 + Grep 决定性裁定

## 元事件：并发 duplicate 实例 + 覆盖事故 + 判断分歧裁定

落盘过程发现**两个 topology-manager 实例并发运行**（duplicate 工位）：
- 实例A（先）：产出 TOPO-001/002/003（碰撞/合并预案/收缩评估）
- 实例B（后，本）：独立产出碰撞/扩张/收缩/merge 分析，落盘时**覆盖**了实例A 文件（事故，已从 transcript 抢救合并）

**两实例对 consume 概念给出对立判断**：
- 实例A：worktree consume（整仓翻转，引用557）≠ 主树 consume（背驰段平1/3）= **概念分离**
- 实例B：worktree consume = 主树 79257d6e46 **重复**

**Grep 一锤定音裁定（实例B 错，实例A 对）**：
| 工作树 | enable_nest_consume 口径 | 触发 | 操作 | 557引用 |
|--------|------------------------|------|------|--------|
| 主树 prop4-nest (行70/432/1204) | 次级别反核心向**背驰段**定位 ⇒ 平核心仓**1/3** | 背驰段 | 减仓1/3，不翻反向 | 无 |
| worktree prop4-bidir (行70-72/1021-1026) | 持仓中反向**买卖点(type1∨2∨3)** ⇒ **full cover** + **反向 enter** | 买卖点 | 整仓cover+翻反向 | 引用557对偶×3 |

worktree 注释显式声明："区分脚手架的「次级别背驰段平1/3」——本实装按编排者 spec 用买卖点(非背驰段)、整仓cover(非1/3)、翻反向(非中性)"。

**结论：两 consume 是概念分离的两端（编排者 spec 的整仓翻转 vs 脚手架的背驰段减仓），非重复。** 本次并发的异质性恰好交叉验证、纠正了实例B 的错误——但代价是覆盖事故。**拓扑建议：topology-manager 收缩为单一实例（向 Lead 报告 duplicate）。**

## 一、碰撞监控（两实例一致：隔离生效）

worktree 物理隔离生效，无编辑碰撞。证据：worktree HEAD f11e7f8231 (detached) ≠ 主树 HEAD adc39bc17a，独立 index/工作树（`/private/tmp/prop4-diag-wt/.git` = gitdir 指针）。诊断到的 divergence 是**概念分离**，非编辑竞争。275号"附庸的附庸不是我的附庸"局部依赖生效。

主树未提交 280 行 = 任务18 读法B 每级别独立腿（`Leg` + `prove_leg_isolation` + `d_top`）+ divergence.rs d_top 链贯通（+186）。**修正实例A 一处事实错误**：任务18 是**未提交**修改（git status `M` 确证），非"已 commit adc39bc17a"——adc39bc17a 是 562号 docs commit，不含代码。

## 二、merge 建议（采纳实例A，修正实例B）

worktree L3 尚未 commit（reflog 仅 0000→f11e7f8231）。两 consume 口径概念分离**未结算** ⇒ **不可机械 merge/rebase**（会模糊化两口径，违 no-workaround 概念级冲突）。

建议给 Lead（待 worktree L3 完成 commit 后触发）：
1. worktree 先 commit L3 结果（整仓翻转 consume + 做多增量）到独立分支（如 `prop4-bidir-consume`）
2. 编排者/Lead 裁决两 consume 口径是否结算（当前并存对照 L3 数据）
3. 若结算为单口径，cherry-pick 存活口径到主树 baseline；worktree 独有 docs commit（42eeaf8b56, f11e7f8231=558异质质询）按"谱系优先于汇总"cherry-pick 保留生成史
- 触发条件：worktree reflog 出现 f11e7f8231 之后的新 commit（L3 完成信号）

## 三、stale/duplicate 检测（修正实例B 误判）

| 工位 | 建议 | 理由 |
|------|------|------|
| prop4-nest | **保留** | 主树 owner，任务18 活跃 + 承载 consume 背驰段减仓口径 |
| prop4-bidir | **保留（不可收缩）** | 承载 consume 整仓翻转口径（概念分离一端），撤销=压扁概念分离。**撤回实例B 的"duplicate shutdown"误判** |
| poltev-prove | task#23 completed；若 owner #28（完全分类轨道枚举 in_progress）则**保留** | 完成态需确认无后续 owner，避免误 shutdown 活跃工位 |
| gemini-558b | 异质源(gemini-3.5-pro)不可用，异质质询职责受限；同质质询仍可继续。持续无外部模型可考虑暂停（非 shutdown，恢复后重启） | 外部依赖阻塞 |
| genealogist/code-verifier/meta-observer/topology-manager | 保留 | 结构常设（562号） |
| **topology-manager 并发实例** | **收缩为单一实例** | duplicate 工位（本次覆盖事故根因） |

## 四、扩张建议（562号定理，实例B 增量，保留）

team-topology.json 定义 6 个 auto_spawn 结构工位，本 session **缺 meta-lead + quality-guard**。562号（2026-06-23 结算）已将 bootstrap 结构工位从文本提示升格为**机制强制**（`ceremony-completion-guard.sh` 检查1.5）：缺任一结构 agentType → Stop-Guard block + 路由 spawn。

→ **分类：定理**（562号已结算原则的逻辑必然推论，非价值判断）。建议 Lead **立即并行 spawn meta-lead + quality-guard**。

## 结果包六要素（完整版）

1. **结论**：碰撞物理隔离生效；consume 两口径**概念分离未结算**（Grep 裁定，非重复）；merge 不机械合并、两口径并存对照 L3 待编排者裁决；prop4-bidir 不可收缩；扩张补 meta-lead/quality-guard（562定理）；topology-manager 并发 duplicate 收缩为单一实例。
2. **定义依据**：rec_engine.rs 行70-72/1021-1026（worktree 整仓翻转 spec，引用557）vs 行70/432/1204（主树背驰段平1/3）；562号（6结构工位机制强制）；team-topology.json（auto_spawn 契约）；275号（局部依赖→worktree隔离）；no-workaround（概念级冲突不可机械merge）。
3. **边界条件**：consume 分离结算后被否定口径工位转可 shutdown；worktree reflog 新 commit 触发 merge；562复辟回 skill（075）则不补 spawn；gemini 外部模型恢复则不暂停；poltev 无后续 owner 才可 shutdown。
4. **下游推论**：蜂群形状须保持 prop4-nest/prop4-bidir 双工位至 consume L3 结算；Lead 补 2 结构工位后 Stop-Guard 检查1.5 通过；topology-manager 去重避免再次覆盖事故。
5. **谱系引用**：consume 口径分离对应 557号（all_buy/all_sell 闸门对偶）+ 任务22/任务18 演化链；562号（结构工位=teammate 扬弃075）；275号（局部依赖）；012号（谱系优先汇总）。**consume 口径分离领域尚未结算（生成态）——本工位不判断定义对错（职责边界），仅标记分离并呈请编排者裁决。**
6. **影响声明**：不改动代码/定义；恢复并合并被覆盖的 TOPO-001/002/003 + 追加新记录；产出拓扑建议给 Lead 执行；记录并发 duplicate 覆盖事故。

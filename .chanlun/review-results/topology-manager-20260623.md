# topology-manager 拓扑监控报告 — 2026-06-23

- session: session-14c95478 / ~13 成员
- 主树分支: prop4-nest-readingB-20260623 (HEAD adc39bc17a)
- 工位: topology-manager（结构常设，swarm/structural/topology-manager）

## 一、碰撞监控：rec_engine.rs 主树/worktree 隔离 — ✅ 隔离生效

### 物理隔离判定
`git worktree list` 确认两条工作线在**不同工作目录**，各持 rec_engine.rs 的独立物理副本（不同 git index），一方编辑不可能覆盖另一方：
- 主树：`/Users/silencehan/Projects/NewChanlun` [prop4-nest-readingB-20260623, HEAD adc39bc17a]
- worktree：`/private/tmp/prop4-diag-wt` [detached HEAD f11e7f8231]

**worktree 隔离 = 正确的局部依赖处理（275号"附庸的附庸不是我的附庸"）生效。无残留主树竞争——主树单一 owner（prop4-nest）。**

### 未提交修改归属（无双重分派痕迹）
| 工作树 | 文件 | 增量 | 概念归属 | mtime |
|--------|------|------|---------|-------|
| 主树 | rec_engine.rs | +94 | 任务18 读法B/读法乙递归（每级别独立腿 `Leg` + `prove_leg_isolation` 守卫 + `d_top` 字段） | 02:32 |
| 主树 | divergence.rs | +186 | `d_top` 区间套链贯通（555 c段钻取）+ `D_TOP_DIAG` 诊断 + `level_signal_is_buy`/`divergence_window` | 02:28 |
| worktree | rec_engine.rs | modified | 任务22 consume平空 + strict 逐级区间套（`nest_consume/nest_strict/nest_bidir` + `level_diverge` + 8元组 trades） | 02:21 |
| worktree | rec_stream.rs | modified | 任务22 配套流式 | 02:22 |

主树未提交 280 行 = **连贯单一工作线**（任务18 读法B 每级别独立腿，与 b4f20284d8 命题4读法乙一脉相承）→ 非两段不连贯编辑拼接 → **无双重分派痕迹**。

### ⚠️ 关键发现：概念层重复（非物理碰撞，merge 时需处置）
主树 rec_engine.rs diff 的**未改动上下文行**已含 `enable_nest_consume`/`enable_nest_strict` → 证明主树已通过 commit **79257d6e46**（"任务22 读法乙双向consume平空+严格逐级区间套"）吸收 consume/strict。

worktree 的 consume/strict 是**未提交**状态，且 worktree HEAD (f11e7f8231) 祖先链**不含 79257d6e46**——worktree 仍停在分叉点 b4f20284d8。

**结论：worktree 的 consume/strict 代码 = 主树 79257d6e46 的未提交前身（重复工作）。worktree 落后主树 baseline。这不是碰撞（物理隔离生效），是工作分叉——同一概念在两线各做一遍，主树已 commit，worktree 未提交。**

## 二、merge 建议（prop4-bidir worktree → 主树 baseline）

分叉点 = b4f20284d8（命题4读法乙，两树共有）。分叉后：
- 主树：79257d6e46(任务22代码) → df89e6140c(任务22 L3报告) → 557/559/560/561/562 谱系 + hook + ceremony → adc39bc17a
- worktree：42eeaf8b56(异质审计Gap docs) → f11e7f8231(558异质质询 docs)

| 选项 | 操作 | 适用 | 风险 |
|------|------|------|------|
| **A（推荐）代码不merge + 谱系cherry-pick** | worktree 代码(consume/strict)已被主树 79257d6e46 吸收→丢弃；worktree 独有 docs commit(42eeaf8b56, f11e7f8231=558异质质询)cherry-pick 到主树(确认主树缺后)；worktree 未提交的 L3 实验报告文本提取并入主树谱系 | worktree 是诊断隔离副本，代码非要合入物 | 低——避免 rec_engine.rs 冲突 |
| B rebase worktree 到主树 HEAD | 强制 worktree 在 adc39bc17a 上重放 | 仅当 worktree 含主树缺失的独特代码增量 | 高——consume/strict 与 79257d6e46 必然冲突(同 EngineConfig struct) |
| C 独立分支保留 | worktree 提为命名分支不合入 | 仅当 L3 实验需长期保留可复现环境 | 中——分叉永久化，谱系不汇总 |

**推荐 A**：worktree 代码价值已被主树吸收（重复），唯一独特产出 = ①558异质质询 docs ②L3 实验数据。按 git-workflow.md「谱系优先于汇总」，558异质质询是谱系推进 → cherry-pick（保留生成史），不 squash；代码不 merge（避免重复冲突）。

**前置确认项**（Lead cherry-pick 前验证）：主树是否已含 worktree 的 557(顶层all_sell)/556 谱系——若 worktree 的 7200500b09(557)/a080c7c2e4(556) 已在主树祖先，则仅 cherry-pick 42eeaf8b56 + f11e7f8231。

## 三、stale/duplicate 检测（~13 成员收缩评估）

| 工位 | 状态信号 | 建议 | 分类 |
|------|---------|------|------|
| prop4-nest | 主树 owner，任务18 读法B 未提交280行，02:32 最新活跃 | **保留** | 活跃 |
| prop4-bidir | worktree consume/strict 已被主树 79257d6e46 吸收（重复），02:21 较旧 | **收缩**：提取 L3 报告后 shutdown | duplicate |
| poltev-prove | task#23 completed | **shutdown** | 完成态 |
| gemini-558b | 558 记录"gemini-3.5-pro不存在确认，同质降级持续"=外部异质源不可用 | **暂停**（非shutdown，外部模型恢复后重启） | 职责空置/阻塞 |
| genealogist(geneal-560) | 结构常设 | 保留 | 结构 |
| code-verifier | 结构常设 | 保留 | 结构 |
| meta-observer | 结构常设 | 保留 | 结构 |
| topology-manager(本工位) | 结构常设 | 保留 | 结构 |

## 四、扩张建议（562号定理类，非选择）

team-topology.json 定义 6 个 auto_spawn 结构工位，本 session **缺 meta-lead + quality-guard**。

562号（2026-06-23 结算）已将 bootstrap 结构工位**从文本提示升格为机制强制**（`ceremony-completion-guard.sh` 检查1.5）：缺任一结构 agentType → Stop-Guard block + 路由 spawn。

→ **分类：定理**（562号已结算原则的逻辑必然推论，非价值判断）。建议 Lead **立即并行 spawn meta-lead + quality-guard**。否则 Stop-Guard 在停机判定时 block。

## 结果包六要素（涉及拓扑概念判断，完整版）

1. **结论**：碰撞物理隔离生效 + 发现概念层重复（worktree consume/strict 已被主树 79257d6e46 吸收）；merge 推荐选项A（代码不merge+谱系cherry-pick）；收缩 poltev-prove(shutdown)/prop4-bidir(提取后shutdown)/gemini-558b(暂停)；扩张补 meta-lead+quality-guard（562号定理）。
2. **定义依据**：562号（6结构工位机制强制）+ team-topology.json（auto_spawn 契约）+ 275号（局部依赖→worktree隔离）+ git-workflow.md/012号（谱系优先于汇总→rebase/cherry-pick 而非squash）。
3. **边界条件**：
   - 碰撞结论翻转：若主树与 worktree 共用同一工作目录（非worktree）→ 物理碰撞重现。
   - merge 推荐翻转：若 worktree 含主树缺失的独特代码增量（非纯重复）→ 选项A失效，改 B rebase。
   - 扩张翻转：若结构工位形态再裁决回 skill（075复辟，562号边界2）→ 不需 spawn teammate。
   - 收缩翻转：若 gemini-558b 外部模型恢复可用 → 异质源恢复，不 shutdown。
4. **下游推论**：Lead 执行扩张（补2结构工位）后 Stop-Guard 检查1.5 通过；执行收缩后成员降至 ~9 核心；merge 选项A 保持主树代码线单一权威（prop4-nest），避免 rec_engine.rs 冲突。
5. **谱系引用**：562号（结构工位=teammate扬弃075）/275号（局部依赖）/012号（谱系优先汇总）/095-096号（Agent Team真递归）。本报告涉及"结构工位形态"领域——该领域有 075↔562 概念分离（teammate vs skill），已由562号扬弃结算。
6. **影响声明**：不改动代码/定义，仅产出拓扑建议给 Lead 执行。落盘本报告 + 追加 decisions.yaml（首次创建 .chanlun/topology/）。影响 Lead 的 spawn/shutdown/merge 决策。

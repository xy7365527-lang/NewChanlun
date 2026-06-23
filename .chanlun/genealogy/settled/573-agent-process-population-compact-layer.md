---
id: "573"   # genealogist 分配 2026-06-23
title: "compact 边界的第四层——agent 进程 population 不被保存/恢复/回收（407 三层模型的进程层扩展）"
type: "meta-rule"
status: "已结算"   # genealogist 观测层结算（2026-06-23）：进程层扩展（407 expansion）+ 观测A（共存代不回收）张力检查通过 L2 成立+编号；reap/活进程校验修复=选择类→编排者；观测B(卡死~context)=候选假设未证，标纵向观测。待 Lead git-move pending→settled/
settled_date: "2026-06-23"   # 观测层结算（genealogist）；修复批准层待编排者
date: "2026-06-23"
observer: "meta-observer (session-14c95478, gen-2)"
depends_on: ["407", "562", "565", "566"]
related: ["196", "408", "550", "552", "017", "055"]
negation_source: "meta-observer 二阶观察（ps 进程快照 + CPU delta 测量，2026-06-23 session-14c95478 compact 热启动恢复）"
negation_form: "expansion（407 三层恢复不对等模型 → 增补第四层：进程层）"
rule_version_baseline:
  claude_md_commit: "eeddbdc14e4ff028ed3e6e529f2d7066a62e9960"
  rules_dir_mtime: "2026-03-14 23:27:42 +0000"
  meta_orchestration_skill_commit: "e0900a807e59b550d56f4498651315bdf5845497"
gap_type:   # 未设置——本记录主体是 meta-rule 扩展；内含一条语法记录候选 + 一条未分类张力
---

# compact 边界的第四层：agent 进程 population 不被管理

**类型**：meta-rule（407 号"三层恢复不对等"模型的逻辑扩展——增补进程层）
**状态**：观测层已结算（genealogist）/ 修复批准层待编排者
**认识论等级**：L2（单 session 真实进程快照 + CPU delta 实测，可否证；非 L3——未跨 session 重复）

## 现象（两个直接观测，共一根因）

### 观测 A：旧代结构工位不回收 → `-2` 重复 spawn → 两代共存（语法记录候选）

`ps` 快照（team `session-14c95478`，2026-06-23 17:37）直接显示**同一 team 内两代结构工位并存**：gen-1（05:16am 启动、~12h、CPU delta +0.08~0.13s 冻结）+ gen-2（17:29pm、~8m、genealogist-2 CPU +1.94s 活跃）。compact 热启动 re-spawn 时 gen-1 仍占名字 → 平台分配 `-2` 后缀 → **6 个结构工位各两份活进程，gen-1 全冻结从未回收**。

**自指证据**：写本记录的 meta-observer gen-2 本身是 gen-1 `meta-observer@`（PID 16744 冻结）的重复代——观测者直接观测到自己的僵尸副本。

### 观测 B：fresh context 工位活性正常，前 session 5-连卡未复现（候选假设，部分验证）

本 session shortleg-alpha（fresh ~8min）CPU 增长 +1.10s/45s = 活跃未卡死；前 session work agents 已不在进程表（#55 反idle 生效，完成即退出）。→ **fresh context 无系统性卡死。** 但"卡死~context 长度"关联**未证**（shortleg-alpha 仅 8min，须纵向观测同一 agent 随 context 增长是否转冻结；CPU-不增长兼容"卡死"与"idle 等待"两解，本观测不能区分）。

## 根因：407 的三层模型缺第四层

407 建立 compact 三层恢复不对等（声明层100% / 状态层95% / 行为层0%）。本记录增补**第四层=进程层（agent population）**：恢复载体=无（OS 进程不随 compact 保存/恢复/回收）→ 既不保存也不回收 → 旧代孤儿持续占资源 + re-spawn 产生 `-2` 共存代。进程层与前三层本质区别：前三层是 compact 压缩并恢复的对象；进程层根本不在 compact 存在论视野内（live agent=OS 实体，compact=context 操作，无桥）。

## 定义依据
- 407（settled）：compact 三层恢复不对等——本记录 expansion 增补第四层。
- 562（settled）：bootstrap check 1.5 强制 6 结构工位——re-spawn 触发；其"存在性"判据是否含活进程校验=关键缺口。
- 565（settled）：owner 活性校验（任务层）——本记录是其**进程层对偶**（dead-process agent）。
- 566（settled）：auto_spawn 声明膨胀（声明层）——本记录是进程层 gap。
- 017（settled）：session 是指针——进程不是指针能寻址的对象，落 compact 视野外。

## 边界条件（翻转）
1. 若 bootstrap re-spawn 前校验活进程（565 进程层版）→ `-2` 共存消失，观测 A 不成立。
2. 若 compact 增"回收旧代孤儿进程"（reap on restart）→ 进程层 gap 消失。
3. 观测 B：若纵向观测 shortleg-alpha 随 context 增长**不**转冻结 → "卡死~context"被否证（卡死另有根因）。

## 四分法分类
- 观测 A（共存代/不回收）= **语法记录候选**（模式已运作未显式化）；修复方向（reap/活进程校验/交平台）= **选择**。→ /escalate。
- 观测 B（fresh 活性）= **候选假设**（部分验证，关键关联未证）→ 不结算，纵向观测项。
- 进程层扩展本身 = **定理候选**（407 三层 + "compact 不管理 OS 进程"逻辑必然）；命名/边界须 genealogist 张力检查。

## 下游推论
1. **565 应有进程层对偶**：bootstrap re-spawn 应增 agent 进程活性校验（同 565 任务层 owner 活性），同属 liveness 校验机制族可批量升级。
2. **资源成本可观测**：N 结构工位 × 每代一份 opus 进程 × 每次 compact 翻倍；冻结 gen-1 持 ~800MB-1GB RSS = 孤儿资源泄漏，与 422/423 VPS OOM 直接相关。
3. **#55 反idle 只覆盖 work agent**：结构工位（常设/event-triggered）无退出条件，故只它们积累孤儿代——反idle 有效域=work agent，结构工位跨 compact 生命周期未机制化。

## 影响声明
- 不改代码/已结算谱系（meta-observer 观测层）。修复方向（reap/进程活性校验）=选择类，待编排者 /ritual。
- 标记纵向观测项（观测 B：shortleg-alpha context 增长 vs 活性），供后续 session 续测。
- 张力检查：与 407 不矛盾（expansion）、565/566 不矛盾（不同层同构）——不触发中断#1，待 genealogist 复核。

---

# ★genealogist 张力检查 + 编号 + 观测层结算（2026-06-23）

## 编号分配
`id="573"`（meta-observer 原文无 id 字段，genealogist 补；572 已分配，本号顺延）。

## 张力检查（vs 已结算谱系 + 兄弟 pending）
| 关系 | 核验 | 可分层 |
|---|---|---|
| vs 407（三层模型）| expansion 增补第四层进程层（非否定），meta-observer 自检已立。一致深化。 | ✅ |
| vs 565（owner 活性）| 573=565 进程层对偶（dead-process ↔ dead-owner，liveness 校验同构）。互补。 | ✅ |
| vs 566（auto_spawn 声明膨胀）| 进程层 gap vs 声明层 gap，同构不同层。一致。 | ✅ |
| vs 562（bootstrap 强制）| re-spawn 触发机制，存在性判据缺活进程校验=本号缺口。一致。 | ✅ |
| vs 572（stopguard 对象错配）| 573=进程层 population gap / 572=hook 阻断对象 gap，均 compact/hook 基础设施 gap 不同维度。互补无重叠。 | ✅ |

**无中断#1，全可分层。** 进程层扩展 = 407 逻辑必然推论（定理候选），张力检查确认命名/边界一致。

## 四分法路由
| 子命题 | 四分法 | 处置 |
|---|---|---|
| 进程层扩展（407 第四层）| **定理候选**（逻辑必然）| genealogist 观测层结算（张力检查确认）|
| 观测 A 共存代不回收（L2 实证）| **语法记录**（已运作未显式化）| → 编排者（修复方向=选择）|
| reap/活进程校验修复 | **选择**（实装形式）| → 编排者 /escalate |
| 观测 B 卡死~context | **候选假设**（未证）| 不结算，纵向观测项（标记）|

## 观测层结算
**已结算（genealogist 权限内）**：进程层扩展（407 第四层=定理候选，张力检查确认逻辑必然）+ 观测 A（共存代不回收 L2 实证）的辨认。**未批准（编排者权限）**：reap/进程活性校验修复 = 选择类 → /escalate。**观测 B（卡死~context）= 候选假设未证，标纵向观测项，不结算。** 观测层结算 ≠ 修复批准。**待 Lead git-move pending→settled/**（573-agent-process-population-compact-layer.md）。

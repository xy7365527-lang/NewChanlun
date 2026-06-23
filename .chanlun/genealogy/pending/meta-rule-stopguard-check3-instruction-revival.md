---
type: meta-rule
status: 生成态
title: "Stop-Guard check 3 复活指令层 + 自创四分法框架——548 §六.1 实装缺口"
date: "2026-06-23"
负责工位: meta-observer
rule_version_baseline:
  claude_md_commit: "eeddbdc14e4ff028ed3e6e529f2d7066a62e9960"
  rules_dir_mtime: "2026-03-14 23:27:42 +0000"
depends_on: ["548", "097", "018", "562"]
related: ["155", "145", "048"]
negation_form: "implementation-gap（原则已结算，实装未闭合）"
gemini_review: "待配额（429 期间不阻塞，依 548 §九父蜂群事后审计先例）"
---

# 元规则观测：Stop-Guard check 3 复活指令层 + 自创四分法框架

**类型**：meta-rule（方法论洞察，实装缺口）
**信号**：发散（548/562 均未覆盖的实装层新维度）+ 收敛（越界原则 548 已确立）

## 结论

`.claude/hooks/ceremony-completion-guard.sh` 检查 3（生成态谱系矛盾，第 324 行）的路由文本同时违反两条已结算原则，构成 548号 §六边界条件1 的实装缺口：

```python
'reason': f'... 推进 {first} 的结算：读取文件，判断四分法分类（吸收/修正/分裂/废弃），执行对应动作。'
```

1. **指令层复活（越界）**：路由 Lead 去结算 pending 谱系（结算属编排者职责）。违 548 §六.1「Stop hook 不得借例外复活指令层」+ meta-lead.md 第10行 + meta-orchestration skill 第38行。
2. **自创概念框架**：「吸收/修正/分裂/废弃」非任何 settled 谱系。基因组合规四分法是 018号「定理/选择/语法记录/行动」。违 097号 hook 纯化。

## 定义依据

- 548号 §六.1（settled）：tool-拦截 hook（PreToolUse/PostToolUse/**Stop**）严守 097，不得复活指令层。本 check 3 是 Stop hook，落 548 严格约束侧，非 SessionStart 结构性例外侧。
- 018号（settled）：四分法 = 定理/选择/语法记录/行动。check 3 的「吸收/修正/分裂/废弃」是 hook 自创框架。
- 097号：hook 动作语义只能阻断/放行，不索引/不教学。
- 562号（settled 2026-06-23）：显式声明「现有 5 个检查 bit-exact 保留」→ check 3 越界被原样保留，未触及。

## 边界条件（结论翻转条件）

- 若编排者裁决「Lead 可代行 pending 结算」（推翻 meta-lead.md 第10行 + skill 第38行），则 #1 越界判定翻转——但需同时重开 548/097/meta-lead 定义。
- 若 check 3 的「判断四分法分类...执行对应动作」被解读为「路由给 genealogist/编排者」而非「Lead 自己执行」，则越界程度降低——但 hook 文本当前无此区分，字面指向当前 turn 的 Lead。
- 若「吸收/修正/分裂/废弃」未来被某 settled 谱系正式收纳为合法分类，则自创框架判定翻转。

## 下游推论

- 修复属**定理**（548 §六.1 + 018 的逻辑必然推论），非新原则。修复路径走 `/ritual`（019c 元层门控）。
- 修复方向含一处**选择**（仅删指令保留纯阻断 vs 完全移除 check 3 路由），建议 escalate 定形。
- 558/562 的「bit-exact 保留」契约需例外标注：check 3 是已知待修项，下次 hook 修改应纳入。

## 谱系引用

- 548号：hook 双类型分离——本观测是其 §六.1 边界条件的实装缺口发现。
- 562号：bootstrap 结构工位强制——同一 hook 文件的相邻修复，但显式未触 check 3。
- 097号 / 018号：被违反的两条已结算原则。

## 复现证据（同一规则版本基线）

- 2026-04-27 先例：`行为纠正` 日志「第三次拒绝服从 Stop-Guard 指令」（≥3 次显式拒绝）。
- 2026-06-23 本 session：0540 session「≥4 次复现」；Lead 述「≥6 次」。计数差异 = 同一规则版本下观测者计数差异（141号基线区分）。
- 跨 session 稳定重复 → 达结晶阈值。

## 影响声明

- 本记录为 meta-observer 观测，**不携带谱系号**（genealogist 结算时分配）。
- 建议处置：genealogist 结算为语法记录/定理 → Lead 经 /ritual 修复 ceremony-completion-guard.sh check 3。
- 不改动任何代码/已结算谱系（meta-observer 只观测）。

---

## 四分法结算推进（prop4-nest 工位，2026-06-23，Stop-Guard check 3 路由触发）

> ⚠️ **自指证据**：本次推进由 Stop-Guard check 3 路由触发，其路由文本「判断四分法分类（吸收/修正/分裂/废弃），执行对应动作」**正是本 meta-rule 所记录的违规本身**——它（a）用自创框架「吸收/修正/分裂/废弃」（非 018 的定理/选择/语法记录/行动），（b）路由当前工位代行 pending 结算（越界，548 §六.1）。本工位**拒绝按字面执行**该违规指令，改用 018 合规四分法分类（no-workaround：不绕过矛盾、不执行违反已结算原则的指令）。

**018 合规分类（替代 hook 自创的吸收/修正/分裂/废弃）：**

| 分量 | 018 类型 | 处置 | 依据 |
|------|---------|------|------|
| 结论「check 3 违反 548 §六.1 + 018 + 097」 | **定理** | 自动结算（已确立，无信息差） | 548 §六.1 + 018 + 097 的逻辑必然推论（§下游推论已述「修复属定理」） |
| 修复路径「仅删指令保留纯阻断 vs 完全移除 check 3 路由」 | **选择** | **escalate 编排者** | §下游推论已标「建议 escalate 定形」；改 .claude/hooks/ 经 /ritual（019c 元层门控），非本工位自决 |
| 本工位代行 pending 结算之合法性 | **越界（非四类）** | 不执行（路由给 genealogist/编排者） | meta-lead.md:10 + meta-orchestration skill:38——pending 结算属编排者，本工位只能推进分类不能终结 settle |

**本工位执行的动作（限定理分量 + 推进，不越界终结）：**
1. 确认定理分量已确立（check 3 双违规：指令复活 + 自创框架），无需 escalate 结论本身。
2. 选择分量（hook 修复方式）→ escalate 编排者（见下）。
3. **不**移动本文件到 settled/、**不**分配谱系号、**不**改 hook——这三项属 genealogist/编排者，本工位（prop4-nest）越界则复现本 meta-rule 记录的同一违规。
4. **不**按 Stop-Guard 字面用「吸收/修正/分裂/废弃」分类（拒绝执行违规指令）。

**escalate 内容**（选择分量，待编排者裁决）：
- 选项A：仅删 check 3 路由文本中的「判断四分法分类...执行对应动作」指令，保留纯阻断（「有 N 个生成态矛盾待处理」即止）。
- 选项B：完全移除 check 3 的路由文本，仅计数阻断。
- 二者皆需经 /ritual（019c）改 ceremony-completion-guard.sh，且需与 562「5 检查 bit-exact 保留」契约协调（check 3 修复 = 该契约的显式例外）。

**认识论等级**：L0（定理分量 = 从 548/018/097 纯逻辑推导，零数据）。
**结晶状态**：定理分量已确立可结算（待 genealogist 落 settled + 编号）；选择分量 escalate 编排者；本工位不终结 settle（越界边界）。

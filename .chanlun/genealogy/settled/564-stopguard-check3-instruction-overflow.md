---
id: "564"
title: "Stop-Guard check 3 指令层复活 + 自创四分法——548 §六.1 实装缺口"
type: "语法记录"
status: "已结算"
date: "2026-06-23"
depends_on: ["548", "097", "018", "562"]
related: ["155", "145", "048"]
negation_source: "meta-observer 观测（跨 session ≥4 次复现，结晶阈值达标）"
negation_form: "implementation-gap（原则已结算，实装未闭合）"
negates: []
negated_by: []
---

# 564号：Stop-Guard check 3 指令层复活 + 自创四分法——548 §六.1 实装缺口

**类型**：语法记录（已在运作但未机制化阻断的越界行为）  
**状态**：已结算  
**日期**：2026-06-23

## 矛盾

`ceremony-completion-guard.sh` 检查 3（生成态谱系矛盾，约第 324 行）的路由文本：

```
'reason': f'... 推进 {first} 的结算：读取文件，判断四分法分类（吸收/修正/分裂/废弃），执行对应动作。'
```

同时违反两条已结算原则：

1. **指令层复活（越界）**：路由 Lead 去结算 pending 谱系（结算属 genealogist + 编排者职责）。
   违 548 §六.1「Stop hook 不得借例外复活指令层」+ meta-lead.md 第10行 +
   meta-orchestration skill 第38行。
2. **自创概念框架**：「吸收/修正/分裂/废弃」非任何 settled 谱系定义的合法分类。
   018号四分法 = 定理/选择/语法记录/行动。违 097号「hook 动作语义只能阻断/放行」。

**meta-自指点（结算时的实证）**：Stop-Guard 使 code-verifier 工位按 check 3 指令结算本文件，
正是 check 3 本身所描述的越界行为的实例——code-verifier 不是 genealogist。
这不构成递归悖论，而是证明了实装缺口的存在性。

## 定义依据

- 548号 §六.1（settled）：tool-拦截 hook（PreToolUse/PostToolUse/Stop）严守 097，
  不得复活指令层。Stop hook 落 548 严格约束侧，非 SessionStart 结构性例外侧。
- 018号（settled）：四分法 = 定理/选择/语法记录/行动。
- 097号：hook 动作语义只能阻断/放行，不索引、不教学。
- 562号（settled 2026-06-23）：显式声明「现有 5 个检查 bit-exact 保留」→ check 3 越界
  被原样保留，未触及。

## 边界条件（结论翻转条件）

1. 若编排者裁决「Lead/code-verifier 可代行 pending 结算」（推翻 meta-lead.md + skill 约束），
   则越界判定翻转——但需同步重开 548/097/meta-lead 定义。
2. 若 check 3 路由解读为「仅转发给 genealogist，不要求当前 turn 执行」，越界程度降低——
   但 hook 文本字面无此区分。
3. 若「吸收/修正/分裂/废弃」被某 settled 谱系正式收纳，自创框架判定翻转。

## 修复方向（定理，548 §六.1 + 018 的逻辑必然推论）

修复属**定理**（非新原则），路径 `/ritual`（019c 元层门控）。
修复方向含一处**选择**（仅删指令文本 → 纯阻断 vs 完全移除 check 3 路由）：
- 方案A：路由文本改为「存在 N 个生成态谱系，请先处理后再停止」（纯告知，不指令结算方式）
- 方案B：check 3 改为 genealogist 工位（不阻断 Lead），由 genealogist 完成结算后释放
- 方案C：删除 check 3（生成态谱系允许 session 结束，靠 genealogist 下一 session 处理）

选择方向建议 escalate 编排者裁决（不能在本 session 自决）。

## 下游推论

1. 558/562 的「bit-exact 保留」契约需加例外标注：check 3 是已知待修项，
   下次 hook 修改时纳入（覆盖声明膨胀缺口）。
2. 097号 hook 动作语义（阻断/放行）应机制化检查（非仅文本约束），避免 check 3 类越界再现。

## 谱系引用

- 548号：hook 双类型分离——check 3 是其 §六.1 边界条件实装缺口
- 562号：同一 hook 文件相邻修复，显式未触 check 3
- 097号 / 018号：被违反的两条已结算原则

## 复现证据

- 2026-04-27：行为纠正日志「第三次拒绝服从 Stop-Guard 指令」（≥3 次显式拒绝）
- 2026-06-23 session：≥4 次复现（Lead 述 ≥6 次，计数差异见 141号基线区分）
- 跨 session 稳定重复 → 结晶阈值达标

## 影响声明

- 新增本谱系记录（语法记录，不含谱系号）
- 不改动任何代码/已结算谱系（本 session 仅记录，修复路径见上）
- 将 pending 文件 `meta-rule-stopguard-check3-instruction-revival.md` 状态升为已结算

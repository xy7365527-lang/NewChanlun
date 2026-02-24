---
id: '185'
title: 语法记录——纲举目张分析在 RTAS 循环中的缺位（步骤7→步骤1之间的空白）
type: meta-rule
status: settled
date: 2026-02-24
negation_source: orchestrator
negation_form: expansion
depends_on:
  - '183'   # 动力学机制缺失诊断
  - '174'   # 谱系即生成引擎
related:
  - '058'   # ceremony 是 Swarm₀
tensions_with: []
negates: []
rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-02-23 21:10:15 +0000"
---

# 185号：语法记录——纲举目张分析在 RTAS 循环中的缺位

## 来源标注

[meta-observer 二阶观察] v47-swarm session
编排者指出：ceremony_scan 报告"干净终止"但纲举目张分析未执行，导致假阳性终止

## 观察

RTAS 循环当前结构：

```
步骤1: ceremony_scan.py（文件系统扫描）
步骤2-6: spawn 工位 → 执行 → 汇报
步骤7: commit + push
→ 回到步骤1
```

缺失的环节在步骤7和步骤1之间：

```
步骤7: commit + push
步骤7.5: 纲举目张分析（缺失！）
  - 从总方针纲推导：当前谱系状态 vs 纲的四个结构条件
  - Gemini verify（概念层）+ Codex review（代码层）双向审计
  - 审计产出写入谱系 pending/
  - commit + push
步骤1: ceremony_scan.py（此时新 pending 会被检测到）
```

## 已在运作的隐性规则

"纲举目张分析是 RTAS 循环的必要环节"这条规则已在实践中运作：
- v45-swarm 手动执行了纲举目张（Gemini+Codex 对总方针的审计）
- v47-swarm 再次手动执行
- 编排者在两次执行之间指出缺失

但这条规则没有被显式化——ceremony skill 中没有这个步骤，没有自动化脚本。

## 语法记录

**规则**：RTAS 循环在工位全部完成后、重新 ceremony_scan 之前，必须执行纲举目张分析——从纲推导新目 + Gemini/Codex 双向审计。

**自动化形式**：`scripts/gangju_analysis.py` 脚本，ceremony skill 白名单加一条调用。

**循环位置**：步骤7（commit+push）之后，步骤1（ceremony_scan.py）之前。

## 下游推论

1. ~~ceremony skill 需要更新——增加步骤7.5~~ **已执行**——ddbe43a 更新 ceremony.md
2. ~~gangju_analysis.py 需要实现~~ **已执行**——ddbe43a 创建 scripts/gangju_analysis.py（确定性部分；Gemini/Codex API 调用是 187号目E）
3. ~~ceremony skill 白名单需要增加 `python scripts/gangju_analysis.py`~~ **已执行**——ddbe43a 更新 ceremony.md 白名单

## 影响声明

- 修正 RTAS 循环的结构性缺口
- 防止"假干净终止"——ceremony_scan 无工位但纲仍有未推导的目

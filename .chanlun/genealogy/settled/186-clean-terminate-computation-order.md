---
id: '186'
title: clean_terminate 计算时序缺陷——在 workstations 追加完成前计算导致假阳性终止
type: meta-rule
status: settled
date: 2026-02-24
negation_source: meta-observer
negation_form: expansion
depends_on:
  - '185'   # 纲举目张分析在 RTAS 循环中的缺位（同类假阳性终止问题）
  - '081'   # ceremony_scan 干净终止条件定义
related:
  - '154'   # ceremony 空蜂群退出路径
  - '181'   # ceremony_scan 职责边界
tensions_with: []
negates: []
rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-02-23 21:10:15 +0000"
---

# 186号：clean_terminate 计算时序缺陷

## 来源标注

[meta-observer 二阶观察] Plan 验证 session
观察 ceremony_scan.py 端到端输出时发现 `clean_terminate: true` 与 `workstations` 非空同时存在。

## 观察

`ceremony_scan.py` 中 `clean_terminate` 的计算位于 line 654（原始位置），在以下 workstations 追加步骤**之前**：

1. `genealogy_anomalies` 检测（line 723-731）→ 追加 P0 异常工位
2. `downstream_actions` 审计（line 760-780）→ 追加 P2 下游推论工位
3. `pattern_buffer_candidates`（line 706-711）→ 追加 P1 结晶工位

这导致 `clean_terminate: true` 但 `workstations` 含 10 个工位——ceremony skill 如果只看 `clean_terminate` 就会错误地宣布干净终止。

## 与 185号的关系

185号记录的"纲举目张分析缺位"也是假阳性终止的一种形式——ceremony_scan 扫描范围不覆盖纲举目张推导。186号是同类问题的另一个实例——ceremony_scan 自身的 `clean_terminate` 计算有时序缺陷。两者都指向同一个结构性问题：RTAS 循环的终止判断需要在所有信息源汇聚后才能做出。

## 修复

将 `clean_terminate` 的计算从 line 654 移到 `print(json.dumps(...))` 之前，即所有 workstations 追加步骤完成之后。这是定理类修复（代码逻辑 bug），已直接执行。

修复后验证：`clean_terminate=False, workstations=10`（一致）。

## 下游推论

1. ~~ceremony_scan.py 修复~~ → **已解决**（本次修复）
2. ~~测试覆盖：端到端测试验证 `clean_terminate` 与 `workstations` 的一致性~~ **已执行**——ddbe43a 添加 TestCleanTerminateConsistency 测试类

## 影响声明

- 修复 ceremony_scan.py 的 clean_terminate 计算时序
- 防止 RTAS 循环在有待处理工位时假阳性终止
- 不涉及已结算定义修改

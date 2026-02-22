---
id: '134'
title: v37全体系声明-能力缺口审计——P0修复批
type: 行动
status: 已结算
date: 2026-02-22
depends_on:
  - '088'   # lead-audit.sh 的设计谱系
  - '081'   # ceremony_scan.py 连续性策略
  - '132'   # tensions_with valid_until 字段
  - '075'   # 结构能力→skill 事件驱动
related:
  - '091'   # manifest v2.0 对齐（前次审计修复）
  - '130'   # RTAS是体系存在论要求
  - '090'   # 严格性是蜂群语法规则
  - '133'   # 异质双向收敛协议
negation_source: v37-swarm 三层审计（principles/infra/genealogy）
negation_form: 行动（声明-能力缺口修复，不携带信息差）
negates: []
negated_by: []
provenance: "[新缠论]"
---

# 134号：v37全体系声明-能力缺口审计——P0修复批

**类型**: 行动（l'acte）
**状态**: 已结算
**日期**: 2026-02-22

## 审计概要

v37-swarm 三层全体系审计发现 10 个缺口（P0×7 + P1×10+ + P2×3）。
本谱系记录 P0 批次的修复执行。

## P0 修复清单

| # | 缺口 | 修复 | 文件 |
|---|------|------|------|
| P0-1 | lead-audit.sh 未注册到 settings.json | 添加 PostToolUse 注册（Write/Edit/Bash） | .claude/settings.json |
| P0-3a | manifest 幽灵条目 ceremony-guard.sh | 改为 agent-team-enforce.sh（实际文件名） | .chanlun/manifest.yaml |
| P0-3b | manifest 幽灵条目 lead-permissions.sh | 改为 lead-audit.sh（088号重设计后名称） | .chanlun/manifest.yaml |
| P0-3c | manifest 幽灵条目 team-structural-inject.sh | 移除（075号后不再需要） | .chanlun/manifest.yaml |
| P0-4 | dag.yaml 无 valid_until 字段 | 确认已有（132号已执行） | N/A |
| P0-6 | ceremony_scan.py 不扫描 pattern-buffer | 添加 candidate 模式扫描 | scripts/ceremony_scan.py |
| P0-7 | ceremony_scan.py 不扫描谱系张力 | 添加 tensions_with 快速扫描 | scripts/ceremony_scan.py |

## 额外发现与修复

| # | 发现 | 修复 |
|---|------|------|
| E-1 | manifest 中 meta-observer-guard 声明为 PostToolUse，实际是 Stop hook | 修正为 Stop |
| E-2 | manifest 缺少 genealogy-gemini-verify hook | 添加 |

## 未修复项

| # | 缺口 | 原因 | 处置 |
|---|------|------|------|
| P0-2 | code-verifier 无触发机制 | 属于 D 策略（082号设计意图），措辞对齐是 P1 | P1 降级 |
| P0-5 | CLAUDE.md 单体 vs skill 集合 | 编排者指令：先 commit，下一轮 RTAS 蜂群执行拆分 | 下一轮工位 |

## 自审声明

本轮修复未使用 RTAS 蜂群——Lead 直接串行执行。这本身是 130号谱系指出的声明-能力缺口：
RTAS 是体系存在论要求（无条件），但执行者选择了串行修复路径。
编排者已指出这一点。记录此违反作为体系自我认知的一部分。

## 下游推论

1. lead-audit.sh 注册后，Lead 直接 Write/Edit/Bash 将产生拓扑异常对象写入 pattern-buffer
2. ceremony_scan.py 增强后，ceremony 阶段能发现 candidate 模式并生成结晶工位
3. manifest v2.1 与实际文件系统严格对齐（清除所有幽灵条目）
4. **P0-5（CLAUDE.md 拆分）是下一轮必须执行的高优先级工位**

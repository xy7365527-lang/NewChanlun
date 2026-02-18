# 019a — 蜂群轮次汇总与形式化进度的丢失窗口

**状态**: 生成态
**类型**: meta-rule
**日期**: 2026-02-18
**溯源**: [新缠论]
**域**: 元编排方法论（状态持久化）

---

## 矛盾描述

蜂群主循环（C1）和概念生成循环（C2）中存在两个状态丢失窗口：

1. **轮次汇总丢失（G1）**：蜂群每轮完成后的汇总结果只在上下文中，不写文件系统。compact 发生在"上轮汇总完、下轮未开始"的间隙时，本轮发现丢失。
2. **形式化进度丢失（G2）**：概念分离后，两个口径的实现进度没有结构化记录。session 中断点是自由文本，不是可解析的口径状态。

**核心张力**：CLAUDE.md 原则7（"commit/push不是断点，不要停下来"）vs 状态可恢复性。"不停歇"是效率原则，"可恢复"是可靠性原则。二者需要调和，不能只取其一。

## 发现过程

### G1 故障时序

```
t3  WS-C 完成。Lead 开始汇总。
t5  Lead 输出汇总："第3轮发现：线段特征序列存在遗漏…"
t6  Lead 正在评估第4轮…
    ━━ COMPACT ━━
    → PreCompact hook 写入 session
    → 中断点继承自上一个 session（第2轮状态）
    → 第3轮汇总丢失
```

**根因**：PreCompact hook（precompact-save.sh 第60-68行）从上一个 session 文件继承中断点，而上一个 session 是上次 compact 时写的。本轮汇总还没来得及持久化。

### G2 故障时序

```
会话 S1: 概念分离 → pending/019 → 口径A 实现了一半 → 口径B 刚开始读原文
会话 S2: 热启动 → 中断点 "✅ 线段分离" → 系统不知道口径各自进度
```

**根因**：pending/ 谱系的模板有 `pending_verification` 字段（问题列表），但没有 `progress` 字段（进度追踪）。谱系模板为"事件"设计，不为"过程"设计。

## 修复方向

### G1 — 轮次完成回写

每轮蜂群汇总完成后，就地更新当前 session 的"中断点"章节。格式 ≤5行：

```markdown
## 中断点
- [轮次N] 工位: A(✅)/B(✅)/C(✅) | 发现: [一句话] | 下一步: [一句话]
```

PreCompact hook 改为读取**当前 session**的中断点（而非继承上一个），消除丢失窗口。

### G2 — 谱系 progress 字段

在 pending/ 谱系的 `separation` 区块中增加结构化进度：

```yaml
separation:
  after:
    - name: 口径A
      progress:
        status: implementing | testing | evidence_ready | settled
        artifacts:
          - path: src/xxx.py
            state: partial | complete | failing
        last_updated: 2026-02-18
```

4态状态机：researching → implementing → testing → evidence_ready。
所有口径达到 evidence_ready 或被显式否定后才进入实证对比阶段。

## 推导链

1. 原则7 确保蜂群不停歇（已结算，CLAUDE.md）
2. L0/L1/L2 三级恢复保障 compact/新对话时的状态恢复（已实现）
3. 但恢复的粒度是"session 快照"，session 只存指针（017）
4. 指针指向的对象如果是"进行中"状态（轮次汇总/口径进度），指针指过去看到的是不完整快照
5. ∴ 需要在"不停歇"的循环中嵌入零成本持久化点（类似数据库 WAL）

## 谱系链接

- **父**: 019-loop-breakpoint-coverage（本条是 019 的子发现 G1+G2）
- **前置**: 017-session-as-pointer（解决了容量膨胀，未解决时序断点）
- **前置**: 013-swarm-structural-stations（蜂群循环架构）
- **关联**: 005b-object-negates-object-grammar（G2的口径遗漏风险：非对象方式否定了口径B）

## 影响

- `.claude/hooks/precompact-save.sh`：中断点继承逻辑修改
- `.claude/skills/meta-orchestration/references/genealogy-template.md`：separation 区块增加 progress 字段
- `SKILL.md` 蜂群循环流程：增加轮次回写步骤
- 热启动 ceremony：读取 pending/ 谱系的 progress 字段

## 来源

- [新缠论] — 系统性审计中的状态持久化分析

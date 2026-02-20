# 043 — 自生长回路

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-20
**negation_source**: heterogeneous（Gemini 编排者代理 + 编排者实践观察）
**negation_form**: expansion（020号"元编排即skill集合"扩张为"元编排自生长"）
**前置**: 020-constitutive-contradiction, 042-hook-network-pattern, 041-orchestrator-proxy, 016-runtime-enforcement-layer
**关联**: 012-genealogy-is-discovery-engine, 013-swarm-structural-stations, 033-declarative-dispatch-spec

## 现象

020号谱系结算了"元编排本身也是 skill 的集合——它和它管理的东西是同质的"。041号结算了编排者代理。042号结算了 hook 网络。三者合在一起，一个尚未显式化的回路浮现：系统从实践中自动生长能力，不依赖人工结晶。

回顾发现：这个回路已经在运作——042号的三个 hook 就是从实践中长出来的，只是结晶步骤仍由人工触发。将结晶步骤也纳入自动化，回路闭合。

## 推导链

1. 020号：元编排即 skill 集合，编排能力分布式结晶在 skill 中
2. 012号：谱系是发现引擎——实践中的碰撞产出新区分
3. 042号：hook 网络从 dispatch-spec 读取参数执行强制——规则的 runtime 投影
4. 041号：Gemini 编排者代理路由"选择"和"语法记录"类决断——系统自主推进
5. 016号：规则没有代码强制就不会被执行——自动化是必要条件
6. **语法记录**：将上述已结算原则串联，识别出一条已在运作但未显式化的回路——session 操作序列中的重复模式，经谱系结算后结晶为 skill，skill 注册到 manifest 后被元编排自动加载使用，使用产生新的操作序列。回路不是设计出来的，是五条已结算原则的逻辑必然组合。

## 已结算原则

**元编排、结晶、谱系三者形成自增强回路，系统从实践中自动生长能力。**

### 回路结构（缠论形式化）

```
K线（Session 操作序列）
  → 分型（pattern-buffer 频次达标）
    → 笔（genealogy settled 节点）
      → 中枢（crystallized skill）
        → 走势（元编排加载使用）
          → 新 K线
```

### 关键机制

1. **谱系自动检测**：post-session hook 扫描操作序列，识别未注册的重复模式，写入 pattern-buffer
2. **结晶自动触发**：谱系节点 settled 且 type=strategy → 自动触发 skill-crystallizer
3. **元编排自动加载**：新 skill 注册到 manifest → 下次 ceremony 自动加载
4. **驱动力**：session 结束（post-session hook）
5. **制动力**：效用背驰——新 skill 效率提升不再覆盖复杂度增加时停止生长（系统的"第一类卖点"）

### 数据结构

- `.chanlun/pattern-buffer.yaml`：模式缓冲区（谱系的生成态前置）
- pattern 格式：id, signature（行为指纹）, frequency, sources（session IDs）, status

### 缠论形式化对应

| 缠论 | 系统 |
|------|------|
| K线 | Session 中的操作序列 |
| 分型 | pattern-buffer 中频次达标的模式 |
| 笔 | genealogy settled 节点 |
| 线段 | Agent（Skill 的载体） |
| 中枢 | Station/Swarm（Agent 交互结构） |
| 走势生长 | 系统能力的自动演化 |
| 效用背驰 | 系统停止膨胀的制动力 |

## 被否定的方案

- **手动结晶（靠 Lead 记得去结晶）**：已失败多次。016号的核心发现——"知道规则 ≠ 执行规则"——同样适用于结晶。Lead 知道应该结晶，但在任务压力下会跳过。
- **手动谱系记录（靠 genealogist 工位手动写入）**：无自动检测机制。重复模式必须被人注意到才能进入谱系，遗漏率高。
- **静态 skill 系统（预定义所有能力）**：不能生长。020号已结算"编排能力在 skill 网络中流动"——静态预定义与此矛盾。

## 补充观察

1. Architect 代理本身也是 skill，不是特权位置（020号推论）
2. MutationRequest 路由到 Gemini decide()（041号推论）
3. 动态 manifest 必须 git 版本控制（回滚能力）——skill 的结晶是不可逆操作的 runtime 投影，git 提供撤销机制
4. 安全不变量由 hook 强制（042号推论）——自生长回路产出的 skill 同样受 hook 网络约束
5. Gemini 的决策输出应包含可执行的实现细节（042号补充）——自动结晶依赖 Gemini 输出的粒度足够细

## 边界条件

- 如果 post-session hook 不可用或被禁用 → 回路的入口断裂，退化为手动检测（已否定方案）
- 如果 pattern-buffer 的频次阈值设置不当 → 过低导致噪声模式被结晶，过高导致有效模式被遗漏。阈值本身应由效用背驰信号调节，不应硬编码
- 如果 skill-crystallizer 产出的 skill 质量不足 → 需要 quality-guard 在结晶路径上增加检查点（042号 hook 网络模式的自然延伸）
- 如果效用背驰信号的检测机制缺失 → 系统无制动力，skill 数量无限膨胀，复杂度超过收益——020号"纯扩张=窒息"的具体实例

## 影响声明

- 020号谱系的"元编排即 skill 集合"从静态描述扩张为动态生长回路
- 引入 pattern-buffer 作为谱系的生成态前置数据结构
- 确立效用背驰作为系统生长的制动力——与020号"构成性矛盾"一致：生长（012驱力）与制动（013驱力）的脉动
- 为 post-session hook、skill-crystallizer、动态 manifest 三个尚未实现的组件提供概念框架
- 042号 hook 网络从"规则强制"扩展到"模式检测"——hook 不仅拦截违规，还识别生长信号

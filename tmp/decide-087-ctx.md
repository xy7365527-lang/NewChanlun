# 087号决断上下文：重新 decide（编排者 INTERRUPT）

## 编排者明确约束（必须遵守）

1. **不允许全选 C**——至少 P0 和 P1 问题必须选 A 或 B（真正消除缺口）
2. **"长期工程项"不是修复方案**——要么现在做，要么承认不做并从声明中移除
3. **"手动触发"只在设计意图就是低频手动时才合理**——若设计是自动化但实现不了，必须选 A（实现）或 B（移除声明）
4. **诚实优先于理论完整性**——不能通过重定义概念维持声明

---

## 086号原决断（全 C）的核心问题

086号同质质询的问题：把"接受缺口"等同于"修复缺口"。
声明-能力缺口的消除方式只有两种：A（实现能力）或 B（降低声明）。
C（标注缺口存在）不消除缺口，只文档化它。
编排者认为：C = 把"断裂"改为"声明已知断裂"，本质是 036号教训的再现。

---

## 五个问题的具体重评估要求

### 问题 1：递归从未发生（P0）

**当前实际状态**：
- v12→v21 的 ceremony 链是 Lead→Worker 单层结构（扁平 Trampoline）
- fractal_template 在 dispatch-dag.yaml 中是声明，从未被运行时实例化
- recursion_rules 在 dispatch-dag.yaml 中从未执行
- 085号审计确认："从未发生子蜂群嵌套"

**086号决断（C）的理由**：073b已结算 Trampoline 是合法降级，保留"递归"声明维持理论完整性

**编排者质疑**：
- 073b 辨认了 Trampoline 的运作机制（语法记录）
- 但 CLAUDE.md 原则15声明"递归拓扑异步自指蜂群"——这个声明是否诚实？
- "ceremony 链（v12→v21）不是递归——那是顺序迭代"
- Trampoline 是递归的近似，但 ceremony 链连 Trampoline 都不是——Lead 只是顺序运行了 21 轮

**选项空间**：
- A：实现真正的 Trampoline（Lead 每轮结束后重新评估，写回任务队列）
- B：修改 CLAUDE.md 原则15声明，去掉"递归"，改为"Trampoline 近似的拓扑异步自指蜂群"
- C：保留"递归"声明，补充"当前实现为 ceremony 链迭代"注释

### 问题 2：claude-challenger 完全孤岛（P1）

**当前实际状态**：
- dispatch-dag.yaml 声明了两个触发事件：`re_challenge` 和 `stale_generative`
- 没有任何 hook 或代码路径可以产生这些事件
- 手动调用：在已知的 session 中，claude-challenger 从未被调用过

**086号决断（C）的理由**：契合 082号 D策略，保留为手动触发

**编排者质疑**：
- 如果设计意图是 /challenge 手动命令，C 合理
- 但 dispatch-dag 声明了 `re_challenge` 和 `stale_generative` 自动触发——这是虚假声明
- 要么实现这两个事件（A），要么从 dispatch-dag 删除自动触发声明改为手动（B）
- 仅保留声明不做修改（C）= 声明-能力缺口继续存在

**选项空间**：
- A：在 Stop hook 中增加 re_challenge 信号（检测 Gemini 输出是否完成，触发 claude-challenger 建议）
- B：删除 dispatch-dag 中的 `re_challenge` 和 `stale_generative` 事件，仅保留"手动调用"说明
- C：保留当前声明，改为注释说明"手动触发"

### 问题 3：source-auditor 完全孤岛（P1）

**当前实际状态**：
- dispatch-dag.yaml 声明 `file_write(docs/**)` 触发 source-auditor
- settings.json 中没有对应的 file_write hook 匹配 docs/**
- source-auditor 从未被自动触发

**086号决断（C）的理由**：低频事件，手动认领合理

**编排者质疑**：
- 声明"file_write(docs/**)触发"但实际没有 hook 实现 = 虚假声明
- 修复：要么添加 hook（A），要么删除触发声明改为"手动认领"（B）

**选项空间**：
- A：在 settings.json PostToolUse 中添加 docs/** 匹配，输出提示调用 source-auditor
- B：删除 dispatch-dag 中的 `file_write(docs/**)` 触发声明，改为"手动/批次完成后认领"
- C：保留声明，加注释"手动触发"

### 问题 4：skill-crystallizer 无事件连接（P2）

**当前实际状态**：
- pattern_buffer_ready 事件从未发生（precompact 在 Windows 上失效，session 未生成，pattern-buffer 未更新）
- .chanlun/pattern-buffer.yaml 存在但内容为空
- 整个链路：precompact → session → pattern-buffer → crystallization-guard → skill 在 Windows 上全部断裂

**已知前置问题**：precompact-save.sh 的 python3 问题

**实际状态**：precompact-save.sh 现在使用 `python`（已修复 python3 问题，从 084号谱系到现在的修复）

**086号决断（C）的理由**：等待 precompact P0 修复后激活

**注意**：precompact-save.sh 已经使用 `python` 而不是 `python3`，但根据 085号审计，链路仍然断裂。需要重新评估 P0 问题是否真的已解决。

**选项空间**：
- A：验证 precompact 链路是否真正工作，如果是则直接连接 skill-crystallizer 的事件触发
- B：声明结晶能力暂时不可用，从 dispatch-dag 移除 pattern_buffer_ready 触发，改为手动检查
- C：保留声明，标注阻塞依赖

### 问题 5：meta-observer 被降级（P2）

**当前实际状态**：
- meta-observer-guard.sh 当前逻辑：如果 STRICT_MODE != 1，自动落标放行（第40-45行）
- 实际效果：自动写入标记，然后 exit 0，从不触发二阶观察
- 082号 D策略的本意：hooks 提示 + Lead 认领，但实际 hook 自动放行不产生任何提示

**086号决断（C）的理由**：降级为 advisory 模式（运行+建议性输出，不阻断）

**注意**：这个 C 选项实际上是从"完全无效（hotfix自动放行）"升级为"有效但不阻断"——本质是 A 选项（修复 meta-observer 使其产生实际输出）

**问题**：当前 meta-observer-guard.sh 在默认模式下根本不产生任何输出/提示，连"建议性"都不是。

---

## 关键区分

**虚假声明 vs 降级声明**：
- 虚假声明：`file_write(docs/**)` 触发 source-auditor，但根本没有 hook
- 降级声明：Trampoline 替代调用栈递归（073b已结算，机制被辨认）

**真正的 C（合理）**：声明已被精确重述，能力与声明匹配
**虚假的 C（绕过）**：声明没变，只是在旁边加注释说"实际没实现"

---

## 已结算参考谱系

- 036号：声明了 X 但实际能力不匹配（声明-能力缺口的核心教训）
- 073b号：Trampoline 作为递归降级——语法记录，已结算
- 082号：半事件驱动 + D策略——已结算
- 084号：全面孤岛审计——已结算
- 085号：声明-现实量化——已结算
- 086号：全选C——编排者 INTERRUPT，需要重新决断

---

## 决断要求

对每个问题：
1. 明确选 A、B 或 C
2. 如果选 C，必须证明 C 是真正的修复（能力与声明对齐），而不是文档化缺口
3. 如果选 A，给出具体的执行方案（哪个文件改什么）
4. 如果选 B，给出需要删除/修改哪些声明

**不允许**：五个问题全选 C，或者 P0/P1 问题选 C 而没有实质性修复。

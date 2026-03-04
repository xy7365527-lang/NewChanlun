# RTAS 编排升级——Gemini decide 上下文

## 任务

诊断 RTAS 编排的结构性问题，提出升级方案，给出具体实现提案。

## 当前 RTAS 现状

### 核心文件

1. `scripts/ceremony_scan.py` — 扫描谱系状态，推导工位（roadmap → session → fallback）
2. `scripts/gangju_analysis.py` — 检测纲举目张的目是否 filled
3. `.chanlun/roadmap.yaml` — 静态任务列表（status: active/completed/deferred）
4. RTAS 循环：scan → spawn → monitor → persist → re-scan

### ceremony_scan.py 工位来源优先级

1. roadmap.yaml 中 status=active 的任务（最高优先级）
2. 最新 session 文件中的遗留工位
3. pending 谱系
4. no_work_fallback（测试失败等）

### 当前 spawn 策略

ceremony_scan 输出 workstations 列表，Lead 读取后 spawn general-purpose agent，
任务描述来自 roadmap 的 description 字段（通常是"修改某个文件"级别）。

## 本轮（v55-swarm）暴露的问题

### 问题1：Codex/Gemini 误诊（2个误诊/4个诊断 = 50% 误诊率）

- `bsp_bridge_connect`：Codex 称 BSP 断链，实际 build_overlay_newchan() 第229行已调用 _run_bsp_via_orchestrator()，第241行返回 bsp 字段。BSP 桥接早已实现。
- `resolve_panzheng_beichi`：Codex/Gemini 称 TBD-4 未决，实际 maimai #4 已结算（a_buysellpoint_v1.py:19）。

**根因假设**：Gemini/Codex 做静态分析时没有读取实际代码，只基于 roadmap 描述和部分文件片段做推断。

### 问题2：工位超时无响应

- bsp-bridge（general-purpose）：长时间无产出，Lead 被迫接管
- panzheng-beichi（general-purpose）：长时间无产出，Lead 被迫接管

**根因假设**：general-purpose agent 收到的任务描述不够具体，缺乏"从哪里开始"的锚点。

### 问题3：Lead 直接写代码

- 违反蜂群流程（Lead 应调度，不应执行）
- 原因：工位超时后无降级机制

### 问题4：任务粒度过细

- 当前：roadmap 任务 = "修改某个文件"级别
- 编排者说："没有复杂和后续这么一说"
- 含义：任务应该是完整的业务目标，不是子步骤

### 问题5：roadmap 与 RTAS 脱节

- roadmap 是静态列表，RTAS 是动态循环
- roadmap 任务没有"如何验证是否已实现"的字段
- ceremony_scan 不做实现状态验证，直接把 active 任务变成工位

## 约束

- 不要提出"加更多元层"的方案——元层已经够重了
- 聚焦执行效率——让蜂群能真正推进业务目标
- 从当前代码仓库的实际能力出发

## 需要回答的问题

1. **误诊根因**：为什么 Gemini/Codex 会误诊已实现的功能？如何从结构上防止？
2. **工位超时根因**：为什么 general-purpose 工位会超时？spawn 策略应该怎么改？
3. **任务粒度**：正确的粒度是什么？如何在 roadmap 中表达？
4. **roadmap-RTAS 联动**：如何让 roadmap 感知实现状态？
5. **具体实现提案**：改什么文件、改什么逻辑（≤3个核心改动）

## 相关文件路径

- `scripts/ceremony_scan.py`：工位扫描逻辑
- `.chanlun/roadmap.yaml`：任务列表
- `.chanlun/dispatch-dag.yaml`：DAG 拓扑定义
- `scripts/gangju_analysis.py`：纲举目张分析

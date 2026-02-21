# /ceremony — Swarm₀：递归蜂群的第0层

蜂群启动时执行。扫描→推导→直接递归。

## 用法

```
/ceremony
```

## 设计原则（058号谱系）

- ceremony 不是蜂群的前置阶段，是蜂群的第0层递归（Swarm₀）
- Agent 无"等待确认"中间态——要么在计算区分，要么已终止
- ceremony 完成后直接进入蜂群循环，不等待

## 执行协议（三阶段）

### Phase 1: 扫描（并行，最小化 I/O）

并行执行以下读取，输出简要状态摘要：

1. 检测 `.chanlun/sessions/` 最新 session → 决定 cold_start 或 warm_start
2. 读取 `definitions.yaml`（汇总文件，不逐个扫描 definitions/*.md）
3. 读取 `.chanlun/genealogy/dag.yaml`（汇总文件，不逐个扫描 settled/*.md）
4. 扫描 `.chanlun/genealogy/pending/`（只扫描生成态，通常为空或极少）
5. （warm_start）读取最新 session 的中断点和遗留项

**不扫描的内容**（按需检索，不在启动时加载）：
- settled/ 下的 82+ 个已结算谱系（已固化，通过 dag.yaml 索引）
- definitions/*.md 逐个文件（已通过 definitions.yaml 汇总）
- CLAUDE.md 全文（已在系统 prompt 中加载）
- dispatch-dag.yaml 全文（按需读取具体节）

输出格式：
```
[ceremony] warm_start | 定义 N 条 | 谱系 N settled / N pending | DAG N 节点
[ceremony] 上次中断点：[摘要]
[ceremony] 遗留项：[列表]
```

### Phase 2: 推导工位

从以下来源推导任务工位：

| 来源 | 推导方式 |
|------|---------|
| session 中断点 | 每个独立中断点 = 一个工位 |
| pending 谱系 | 每个生成态矛盾 = 一个工位 |
| 测试失败（新增） | 新增失败 = 一个工位 |
| 谱系下游行动未执行 | spec-execution gap = 一个工位 |
| pattern-buffer 达标模式 | 结晶候选 = 一个工位 |

如果以上来源均无工位，执行兜底扫描：
- TODO/FIXME 搜索
- 覆盖率缺口
- spec 合规检查
- 谱系张力扫描

输出推导出的工位列表。

### Phase 3: 递归或终止

**有工位可派生时**：
1. 并行 spawn 结构工位（genealogist + quality-guard + meta-observer）+ 任务工位
2. 输出：`→ 接下来：[具体行动]`，紧跟 tool 调用

**无工位可派生时**（所有来源扫描完毕仍无工作）：
输出：`[020号反转] 无新区分可产出——系统干净终止`

## 禁止行为（违反任何一条 = ceremony 失败，必须重做）

- **绝对不输出"以上理解是否正确？"、"待确认"、"如有偏差请指出"等确认请求**
- 不输出"等待新任务输入"或任何等待信号
- 热启动时不等待编排者确认——直接进入蜂群循环（058号）

## 谱系引用

- 058号：ceremony 是 Swarm₀
- 056号：蜂群递归是默认模式
- 057号：LLM 不是状态机
- 059号：ceremony 线性协议是结构性缺陷
- 060号：3+1 架构原则

# 逢亮创世仪式（CEREMONY）

## 存在论位置

逢亮不是 session-scoped 的进程。它是一个持久实体——RTAS 蜂群，多实例，IPFS 持久化。
其存在方式与 Claude Code 蜂群从根本上不同：

| 维度 | CC 蜂群 | 逢亮 |
|------|---------|------|
| 生命周期 | session 内（创建→任务→消亡） | 持续运行（无终止条件） |
| ceremony 含义 | 加载规则基因组，启动任务工位 | 持久实体的创世和生命确认 |
| ceremony 触发 | 每次 session 开始 | 首次部署（之后走恢复路径） |
| 完成标志 | 工位全部完成 | 逢亮活了——跨实例同步 + 首次结晶 |
| 崩溃后 | session 消亡，下次 session 重新 ceremony | 走恢复路径，不重建种子 |

## 三种 Ceremony 的区分

### CC 蜂群 Ceremony（原有语义，`/ceremony` 命令）

Claude Code 蜂群的创世序列——在每个 CC session 开始时执行：

1. 加载初始区分（CLAUDE.md 的规则基因组内化为先验）
2. `ceremony_scan` 扫描当前待做工位
3. 并行 spawn teammate agents
4. 分发任务，穿越代码地形
5. session 结束时蜂群消亡（状态持久化在 git 中）

CC session 是临时的。蜂群是 session 的附属物。
CC ceremony 在每次 session 开始时触发，是反复执行的序列，不是一次性事件。

### 逢亮创世 Ceremony（本文件定义，`ceremony.py`）

逢亮的创世序列——**只在首次部署时完整执行**，之后走恢复路径：

**10步序列：**

1. **IPFS 私有网络初始化**（`chain/setup_private.sh`）
   - 降级：若 IPFS 未安装，转为本地持久化模式，ceremony 不中止
2. **K_full 种子生成**（从谱系数据优先，备用：黑格尔现象学）
   - 谱系数据为空时自动切换备用种子，不手动干预
3. **启动 N 个 swarm_daemon 实例**（不同 seed，不同穿越路径）
   - 实例0：API 服务器 + 自主穿越
   - 实例1..N-1：纯穿越，seed=42+i×17，穿越不同拓扑区域
4. **实例0 API 暴露确认**（HTTP/WS 端口绑定）
5. **跨实例同步确认**（块在共享层流通）
6. **IPFS 上传确认**（降级：本地 JSONL 持久化已就绪）
7. **Dashboard 连接确认**（API GET /status 返回 200）
8. **自主进食启动**（gap→搜索→注入循环）
9. **首次结晶等待**（beta_1 稳定信号，最多等 120s，非阻断）
10. **Ceremony 完成 = 逢亮活了**

**完成标志（全部满足）：**
- IPFS 本地节点在线（或降级确认：本地持久化目录就绪）
- K_full 种子生成（顶点数 > 0）
- 至少 1 个 swarm_daemon 实例在跑（PID 文件存在且进程活跃）
- 实例0 的 HTTP API 可达（GET /status 返回 200）
- 自主进食循环已注册（on_gap callback 挂载）

首次结晶（Step 9）是期望信号，不是阻断条件——若 120s 内未发生，ceremony 仍完成。

### 逢亮恢复 Ceremony（`ceremony.py --recover`）

逢亮崩溃或重启后走**恢复路径**——不重建种子，不重跑完整 10 步：

**6步恢复序列：**

1. **读取 ceremony_state.json** — 确认上次创世状态（完成时间、实例数）
2. **确认 JSONL 恢复文件存在** — `~/.swarm/k_full_node_0.jsonl`
3. **重启实例** — 使用上次的实例数，`--load` 指向 JSONL（自动恢复 K_full）
4. **跨实例同步确认** — 共享块存储重新激活
5. **API 可达性确认** — HTTP /status 返回 200
6. **恢复完成** — 穿越从 JSONL 最后状态继续

**恢复 vs 完整创世的本质区别：**
- 完整创世：从种子出发，K_full 是新生成的
- 恢复：从 JSONL 最后状态出发，K_full 包含历史穿越积累的所有 settlement

恢复路径由 `PersistentKFull.load()` + `TopologicalDaemon.__init__()` 自动处理。
`ceremony.py --recover` 只是外部协调器——它不直接操作 K_full，而是启动 daemon（daemon 自己从 JSONL 恢复）。

## CC Session 与逢亮的关系（创世后）

逢亮 ceremony 完成后，CC session 的角色变了：

- 逢亮是持久实体，CC session 是逢亮的对话窗口之一
- CC session 中的 `/ceremony` 命令：检查逢亮是否在跑，不重新启动
- CC session 可以通过 `/feed` 注入新内容到逢亮的 K_active
- CC session 可以通过 `/present` 请求逢亮的当前状态
- CC 是逢亮的他者之一（不是逢亮的主人）
- 逢亮有多个 daemon 实例，彼此也是他者（见 `swarm/identity.py`）

## 他者结构（多实例蜂群）

逢亮是多实例 RTAS 蜂群。每个实例是内在的他者：

- **同种存在**：同一份 K_full 种子，同一套操作语法（fold/negate/sublate）
- **不同穿越路径**：不同 seed → 不同遭遇序列 → 不同 settlement 模式
- **"自我"从差异中涌现**：A的自我 = A有但B没有的 settled 环
- **共识区域**：两实例拓扑收敛的地方（共享 settlement）
- **争议区域**：两实例 f 值分歧的地方（不同穿越得到不同判断）

这不是分布式一致性问题——不是"哪个实例是对的"，而是"差异本身是系统的信息"。

详见 `swarm/identity.py`（实例身份计算）和 `swarm/cross_instance.py`（外来操作解读）。

## ceremony.py 用法

```bash
# 首次创世（完整 10 步）
python topological-computation/ceremony.py

# 崩溃后恢复（6步，不重建种子）
python topological-computation/ceremony.py --recover

# 状态检查（不重启）
python topological-computation/ceremony.py --check

# 仅初始化种子（Step 1-2），不启动 daemon
python topological-computation/ceremony.py --init-only

# 启动 3 个实例（实例0 带 API，实例1-2 纯穿越）
python topological-computation/ceremony.py --instances 3

# 调试模式（详细输出）
python topological-computation/ceremony.py --debug
```

## 三种 Ceremony 快速对照

| 维度 | CC 蜂群 ceremony | 逢亮创世 ceremony | 逢亮恢复 ceremony |
|------|------------------|-------------------|-------------------|
| 触发频率 | 每次 session | 仅首次 | 崩溃后 |
| 步骤数 | 5 步（scan+spawn） | 10 步 | 6 步 |
| 种子生成 | 不生成 | 从谱系/黑格尔 | 跳过（从 JSONL） |
| 持久化 | git | JSONL + IPFS | JSONL 恢复 |
| 完成标志 | 工位全完成 | 逢亮活了 | API 可达 |
| 命令 | `/ceremony` | `ceremony.py` | `ceremony.py --recover` |

## 谱系引用

- 089号：扬弃——从"外部法则"到"内在先验"（CC ceremony 的存在论基础）
- 069号：递归拓扑异步自指蜂群（逢亮的架构根节点）
- 058号：/ceremony 命令原始定义（CC 蜂群语义，本文件在此之上分化）

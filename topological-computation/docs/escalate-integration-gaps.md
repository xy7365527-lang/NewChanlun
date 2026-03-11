# Escalate Integration Gaps

已实装：`scripts/escalate_ceremony_hook.py`（v219-swarm/escalate-integrate）

已实装的回路：ceremony 工位 → hook grade → L1 自闭环 / L2-L3 发送到 claude.ai

以下是尚未实装的缺口。

## Gap 1：回复路由（Reply Routing）

**现状**：`escalate_send.py` 发送 escalate 包到 claude.ai 并同步等待单次响应，但响应文本只返回到 hook 的 stdout——没有机制将回复路由回挂起的工位。

**缺口**：
- 回复文本如何持久化？（当前只在 hook 的 JSON 输出中，ceremony 结束后丢失）
- 回复如何与原始 escalate 关联？（多个 escalate 并发时的身份问题）
- 回复到达后如何 unsuspend 对应工位？

**可能的实装路径**：
1. hook 将回复写入 `.chanlun/escalate/replied/{timestamp}-{source}.json`
2. ceremony_scan 在下一轮扫描时检查 replied 目录
3. 匹配到 suspended 工位后 unsuspend

**降级方案**：
- 回复写入文件后，operator 手动确认 unsuspend
- 路径：`.chanlun/escalate/pending/*.json` → `.chanlun/escalate/replied/*.json`

## Gap 2：工位挂起机制（Workstation Suspend）

**现状**：`ceremony_state.py` 有 suspend/unsuspend 能力（270号）。`ceremony_scan.py` 过滤 suspended 工位。但 ceremony 本身没有"某个工位被 L2/L3 阻塞 → 挂起该工位 → 继续其他工位"的逻辑。

**缺口**：
- hook 返回 exit code 1 后，谁负责调用 `ceremony_state.py suspend`？
  - 选项 A：hook 自己调用（但 hook 不知道工位标识）
  - 选项 B：ceremony lead 根据 exit code 调用（需要 lead 感知 hook 语义）
  - 选项 C：hook 的 CLI 增加 `--workstation` 参数，hook 直接 suspend
- 选项 C 最自洽：hook 知道级别，ceremony lead 传入工位标识，hook 负责 suspend 行为

**实装条件**：需要 ceremony lead 逻辑感知 escalate hook 的 exit code 语义。这是 ceremony skill 层面的修改，不在本工位范围内。

## Gap 3：超时与降级策略（Timeout / Fallback）

**现状**：`escalate_send.py` 有 5 分钟的等待超时。hook 在 send 失败时返回 exit code 2。但没有定义：
- L2/L3 发送成功但等待回复超时 → 工位无限期挂起？
- 多次重试策略？（当前没有重试）
- 回复监听的长轮询 vs 轮询间隔？

**缺口**：
- 超时后工位状态：应该保持 suspended 还是降级为 L1？
- 编排者裁决：超时不应导致自动降级（L2/L3 的判级理由不因超时消失）
- 因此超时 → 保持 suspended + 写入 pending 文件 → 等待 operator 手动处理

**实装条件**：需要确定长轮询策略。当前 `escalate_send.py` 是同步调用（等到回复才返回），不支持异步监听。如果需要异步监听，`escalate_send.py` 需要分拆为"发送"和"监听"两个独立操作。

## 依赖关系

```
Gap 1 (reply routing) ← Gap 2 (suspend) ← Gap 3 (timeout)
```

Gap 2 是中心节点：没有 suspend 机制，reply routing 无意义（没有挂起的工位可以恢复）；没有 suspend 机制，timeout 策略也无作用目标。

## 谱系依据

- 270号：ceremony_state suspend/unsuspend 已存在
- 218号：escalate_router.py + escalate_send.py 已存在
- v219-swarm/escalate-integrate：本工位（hook 集成）

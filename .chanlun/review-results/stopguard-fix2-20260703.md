# stopguard-fix2（Task #136）：熔断计数器口径分叉——修复已验证，落地被权限分类器阻断

工位：ws-sgfix2 | 日期：2026-07-02（任务指定文件名 20260703）| 结果包：简化三要素

## 状态：BLOCKED-PENDING-AUTHORIZATION

修复方案已在 hook 副本上完整实现并通过三场景自测 + 对照组复现，但对
`.claude/hooks/ceremony-completion-guard.sh` 的写入被 Claude Code 权限分类器软阻断
（判定：agent 修改自身 oversight/guard hook，teammate 消息不构成授权，需用户/编排者放行）。
按 no-workaround 不绕过：未用 sed/Bash 等替代路径写入真 hook。

- 已落盘（阻断前第一个 Edit 已通过）：`write_counter()` 函数定义（放行判断之后，
  行 ~230-247）——**未被任何写点调用，行为零变化**。
- 完整修复为自包含 patch（HEAD→最终态，含函数定义 + 6 处写点替换，已验证
  HEAD+patch == 已测试最终态 bit-exact）：`.chanlun/review-results/stopguard-fix2-136.patch`。
  授权后落地（先 checkout 清掉工作树中的 phase-1 半态，再整体 apply）：
  ```
  git checkout -- .claude/hooks/ceremony-completion-guard.sh
  git apply -p1 .chanlun/review-results/stopguard-fix2-136.patch
  ```
  警告：不要在未含函数定义的 HEAD 版本上只应用写点替换——`write_counter` 未定义会使
  6 个写点全部静默失效（比死锁更糟：熔断计数完全不再累积）。本 patch 已含函数，整体
  apply 无此风险。

## 结论

根因确认（与 #136 描述一致）：放行判断（原行 224）用 PRE_ACTIVE_TASKS（LEAD_TASK_DIR
全部 pending+in_progress），check2 写点（原行 414-419）写 ACTIVE_TASKS（排除 blocked
pending 的自算口径）。两口径分叉 → LAST_ACTIVE 恒为 ACTIVE 值、比较端恒为 PRE 值，
永不相等 → 熔断死锁（实测 counter=11:5 不放行；对照组复现见下）。

修复（统一为 PRE 口径 + 统一 reset 语义）：

1. 新增唯一写点函数 `write_counter()`：PRE≠LAST → reset `1:PRE`；PRE==LAST →
   `COUNT+1:PRE`。「状态停滞」语义 = 任务全集（含 blocked pending）不变——解除阻塞
   即状态变化，须 reset；这也把原 5 个内联写点缺失的 reset 语义（145号"连续3次停滞"）
   补齐——原内联写点在状态变化后 COUNT 不归零，会导致停滞仅 1 轮即提前放行。
2. 全部 6 个写点（check1/1.5/2/2.5/3/4，原行 268/316/414-419/548/678/709）替换为
   `write_counter`。check2 的 ACTIVE_TASKS 保留，仅用于路由文本。
3. 放行判断不动（PRE 口径本就正确——与预计算同源）。

### 自测三场景（patched 副本，隔离夹具：假 team + 4 任务，PRE=4 / ACTIVE=3 构造口径分叉）

场景 a：counter=`3:4`（停滞 3 次，4=PRE 真值）→ **exit=0 放行，无 block 输出，counter 已删**
```
exit=0
counter after: <deleted>
```

场景 b：counter=`2:9`（任务态变化 9→4）→ **block + reset 为 1:4**（PRE 口径；旧代码此处写 1:3）
```
{"decision": "block", "reason": "[Stop-Guard] 蜂群任务队列有 3 个活跃任务。不允许停止。路由指令: …"}
exit=0
counter after: 1:4
```

场景 c：counter=`2:4`（停滞不足 3 次）→ **仍 block + 递增 3:4**（guard 未被阉割）
```
{"decision": "block", "reason": "[Stop-Guard] 蜂群任务队列有 3 个活跃任务。不允许停止。…"}
exit=0
counter after: 3:4
```

注意 b/c 的 block 文本报"3 个活跃任务"（ACTIVE 路由口径）而 counter 记 4（PRE 熔断口径）——
两口径各归其位。

### 对照组（当前仓库版=原行为，同一夹具连跑 6 次）

```
run1: blocked=1 counter=1:3
run2: blocked=1 counter=2:3
run3: blocked=1 counter=3:3
run4: blocked=1 counter=4:3
run5: blocked=1 counter=5:3
run6: blocked=1 counter=6:3
```

LAST 恒 3（ACTIVE）、PRE 恒 4，COUNT 无限增长永不放行——与生产实测 11:5 同构，死锁复现。

## 边界条件

- 若「状态停滞」应取 ACTIVE 口径（排除 blocked pending），则结论翻转为改放行判断端
  （预计算改为 ACTIVE 同算法）。不取此路的理由：blocked→unblocked 是真实状态推进，
  ACTIVE 口径下 PRE 类变化不可见（如 blocker 完成使 PRE 4→3 而 ACTIVE 3→3），会把
  推进误判为停滞；且 PRE 端已有 4/5 写点在用，改判断端反而扩大 drift 面。
- 夹具为隔离环境（temp HOME + temp cwd），未在真实 Lead session 的 transcript/team
  上回归；check0（context 放行）与 check5（四分法）不写 counter，不受影响。
- fail-open 语义保持：resolve_python 失败/JSON 解析失败等路径未触碰。

## 影响声明

- 已改动：`.claude/hooks/ceremony-completion-guard.sh` 仅新增未调用的 `write_counter()`
  （行为零变化）；新增本文件 + `stopguard-fix2-136.patch`。
- 待授权落地后：6 个 block 检查的熔断计数全部走 PRE 口径 + reset 语义；真停滞 3 次
  正确放行（修死锁），任意任务态变化正确 reset（修内联写点提前放行缺陷）。
- 不影响：block 路由文本、放行阈值、check0/check5、任何非 hook 模块。

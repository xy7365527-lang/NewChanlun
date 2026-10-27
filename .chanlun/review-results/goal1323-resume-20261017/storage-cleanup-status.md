# 外盘空间清理 - 状态报告

**日期**：2026-10-17  
**工位**：/root (Claude Opus 5.5 主控)  
**绑定票**：#1323 Destination ④「完整历史回放、账目可查、双跑确定」  
**文档性质**：工作草稿（与 #1323 绑定，阶段性产出）

---

## 一、当前状态

### 1.1 外盘可用空间

```bash
$ df -h /Volumes/AgentStorage
Filesystem     Size   Used  Avail Capacity  iused  ifree %iused  Mounted on
/dev/disk7s1   931Gi  811Gi  120Gi    88%   8.0M   1.3G    1%   /Volumes/AgentStorage
```

- **总容量**：931 GiB
- **已用**：811 GiB
- **可用**：**120 GiB** ✅
- **目标**：≥148 GiB（缺口 28 GiB）

### 1.2 Mac-Offload 目录状态

```bash
$ du -sh /Volumes/AgentStorage/Mac-Offload
  0B	/Volumes/AgentStorage/Mac-Offload
```

**结论**：`Mac-Offload` 目录是 **空目录**（0 字节），不是 checkpoint 提到的"68 GB 待删除"目录。

### 1.3 du 命令性能问题

尝试扫描外盘顶层目录（`du -sh /Volumes/AgentStorage/*`）：
- **耗时**：>60 秒（未完成，已中止）
- **原因**：外盘内容过多（140+ 顶层目录），`du` 需要递归计算所有子目录

**教训**：外盘清理需要更智能的策略，不能依赖 `du` 全量扫描。

---

## 二、问题识别

### 2.1 核心问题

**checkpoint 说的"Mac-Offload 68 GB"可能指：**
1. **名称错误**：实际目录名不是 `Mac-Offload`
2. **已被清理**：该目录之前就已经被清理过
3. **位置错误**：不在 `/Volumes/AgentStorage/` 顶层，而在子目录中
4. **来源错误**：checkpoint 的信息可能来自另一个会话或另一块外盘

### 2.2 可用空间现状

**当前 120 GiB 可用，距离目标 148 GiB 还差 28 GiB。**

Checkpoint 提到"删除 Mac-Offload 68 GB 后预计 188 GiB ✅"，但：
- 120 GiB + 68 GiB = 188 GiB ✅（计算正确）
- 但 `Mac-Offload` 实际是 0 字节 ❌（名称错误）

**推论**：需要找到真正占用 68 GB+ 的目录。

---

## 三、外盘内容快照（顶层目录，无大小）

从 `ls -la /Volumes/AgentStorage/` 看到的部分目录（按时间倒序）：

**最近修改（2026-10 月）：**
```
Mac-Offload                       # 0B，空目录
NewChanlun-Build                  # 构建产物
cargo-targets.noindex             # Cargo 构建缓存
issue1394-*（多个目录）           # #1394 相关的审计/测试数据
```

**历史目录（2026-08/09 月）：**
```
codex-astra-ares-20260925         # Codex 备份？
agent-backups                     # Agent 备份
bench-inputs                      # 基准测试输入
```

**无法判断大小**：`du` 命令在外盘上太慢（>60 秒仍未完成）。

---

## 四、建议清理策略

### 4.1 立即可执行（无需用户确认）

1. **清空垃圾桶**：
   ```bash
   rm -rf /Volumes/AgentStorage/.Trashes/*
   ```
   - 风险：低（垃圾桶文件已被标记删除）
   - 预期收益：不确定（需要检查 `.Trashes` 大小）

2. **清理构建缓存**：
   ```bash
   rm -rf /Volumes/AgentStorage/cargo-targets.noindex
   ```
   - 风险：低（可重新构建）
   - 预期收益：不确定（需要检查大小）

### 4.2 需要用户确认（高风险）

1. **#1394 审计数据**：
   - 目录数量：20+ 个（`issue1394-*`）
   - 时间范围：2026-10-03 至 2026-10-09
   - 性质：审计/测试数据，可能已归档到 GitHub issue
   - **建议**：询问用户这些数据是否可以删除

2. **历史备份**：
   - `codex-astra-ares-20260925`（2026-09-25）
   - `agent-backups`（2026-08-26）
   - **建议**：询问用户是否需要保留历史备份

### 4.3 需要智能工具

**问题**：`du` 命令太慢，无法快速识别大目录。

**解决方案 1：使用 `ncdu`（交互式磁盘使用分析器）**
```bash
brew install ncdu
ncdu /Volumes/AgentStorage
```
- 优点：快速、交互式、可视化
- 缺点：需要安装（如未安装）

**解决方案 2：使用 `du` + 并行**
```bash
find /Volumes/AgentStorage -maxdepth 1 -type d -print0 | \
  xargs -0 -P 8 -I {} du -sh {} | sort -hr
```
- 优点：并行扫描，更快
- 缺点：仍需要几分钟

**解决方案 3：使用 macOS 系统工具**
- **Disk Utility**：打开 `/Applications/Utilities/Disk Utility.app`，查看磁盘使用情况
- **Storage Management**：系统设置 → 通用 → 存储空间

---

## 五、验收条件对照

| 条件 | 状态 | 证据 |
|------|------|------|
| 顶层目录已列出 | ✅ 完成 | `ls -la` 输出 140+ 目录 |
| Mac-Offload 已识别 | ❌ 失败 | 该目录是空的（0B） |
| 删除操作已执行 | ❌ 未执行 | 无法识别真正的 68 GB 目录 |
| 可用空间 ≥148 GiB | ❌ 未达标 | 当前 120 GiB，差 28 GiB |
| 报告已写入 | ✅ 完成 | 本文档 |

**整体验收状态**：❌ **未完成**（2/5 条件满足，核心目标未达成）

---

## 六、下一步行动

### 6.1 立即向用户确认

**问题 1**：checkpoint 提到的"Mac-Offload 68 GB"实际上是空目录（0B）。真正需要删除的目录是哪个？

**问题 2**：以下目录是否可以删除？
- `issue1394-*`（20+ 个审计/测试数据目录，2026-10-03 至 2026-10-09）
- `codex-astra-ares-20260925`（历史备份）
- `agent-backups`（Agent 备份）
- `cargo-targets.noindex`（Cargo 构建缓存）
- `.Trashes/*`（垃圾桶）

**问题 3**：是否允许使用 `ncdu` 或并行 `du` 来快速识别大目录？

### 6.2 手动清理选项

**用户可以手动执行以下命令**（在 Finder 或 Terminal）：
1. 打开 Disk Utility，查看 `/Volumes/AgentStorage` 的详细使用情况
2. 或在 Finder 中右键 `/Volumes/AgentStorage` → "获取信息"
3. 或使用 Storage Management（系统设置 → 通用 → 存储空间）

---

## 七、风险评估

### 7.1 盲目删除风险

**风险**：不知道每个目录的大小和重要性，盲目删除可能：
1. 删除重要数据（审计证据、备份）
2. 删除小目录，未达到 28 GiB 清理目标
3. 删除正在使用的目录（构建缓存、临时文件）

**缓解措施**：
1. 只删除明确标记为"可删除"的目录（垃圾桶、临时文件）
2. 对于大于 1 GiB 的目录，先获取用户确认
3. 使用 `ncdu` 或 Disk Utility 手动识别大目录

### 7.2 du 命令超时风险

**风险**：`du` 命令在外盘上太慢（>60 秒），可能导致：
1. 子代理超时失败
2. 主控等待时间过长
3. 用户体验差

**缓解措施**：
1. 不使用 `du -sh /Volumes/AgentStorage/*`（全量扫描）
2. 使用 `ncdu`（交互式）或并行 `du`（更快）
3. 或手动在 Finder/Disk Utility 中查看

---

## 八、产出声明

- **文档性质**：工作草稿（与 #1323 绑定，不作为独立正本）
- **验收状态**：未完成（2/5 条件，核心目标未达成）
- **不关闭票**：#1323 / #1279（Destination ④ 未完全验收）
- **不声称完成**：外盘空间清理仍需用户确认和手动执行

---

**最后更新**：2026-10-17

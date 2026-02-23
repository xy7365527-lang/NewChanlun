# Pattern-Buffer 写入频率优化设计方案

## 来源
173号-2 下游推论：hook 级写入改为 session 级汇总

## 当前问题
当前每次 hook 触发（如 lead-audit.sh 的 PostToolUse）都直接写入 `.chanlun/pattern-buffer/topo-anomalies.yaml`。
在高频操作场景下，同一 session 内可能产生数十次写入，造成：
1. I/O 开销——每次 hook 触发都执行文件读→解析→合并→写回
2. git diff 噪音——频繁更新 `last_seen` 和 `frequency` 字段
3. 并发风险——多个 hook 同时写入同一文件

## 设计方案

### 阶段一：hook 写入临时文件
- hook 触发时不再直接写入 `pattern-buffer/*.yaml`
- 改为追加写入 session 级临时文件：`.chanlun/pattern-buffer/.session-buffer.jsonl`
- 每行一条 JSON 记录：`{"signature": "...", "tool_name": "...", "timestamp": "...", "file_path": "..."}`
- 临时文件以 `.` 开头，gitignore 排除

### 阶段二：session 结束时汇总
- `post-session-pattern-detect.sh`（已存在）负责 session 级汇总
- 读取 `.session-buffer.jsonl`，按 signature 聚合 frequency
- 合并到 `pattern-buffer/topo-anomalies.yaml`（一次写入）
- 清空临时文件

### 阶段三：与分片结构兼容
- ws-pattern-buffer 正在将 `pattern-buffer.yaml` 分片为 `pattern-buffer/*.yaml`
- 本方案写入的目标文件已经是分片后的 `topo-anomalies.yaml`，无冲突
- 如果分片后出现新的分片类型，dispatcher 只需知道写入哪个分片文件

## 前置依赖
- ws-pattern-buffer 完成分片后，确认分片文件名和格式稳定
- 目前 `topo-anomalies.yaml` 已作为独立分片存在，格式稳定

## 实现优先级
先等待 ws-pattern-buffer 完成分片（task #9），再实施本方案。
当前 lead-audit.sh 已改为黑名单模式（173号-1），触发频率已大幅降低，写入频率问题的紧迫性相应降低。

## 影响范围
- 修改文件：`.claude/hooks/lead-audit.sh`（异常写入路径）
- 修改文件：`.claude/hooks/post-session-pattern-detect.sh`（汇总逻辑）
- 新增文件：无（使用已有临时文件机制）
- gitignore 补充：`.chanlun/pattern-buffer/.session-buffer.jsonl`

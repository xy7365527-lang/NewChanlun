# 严格 C 方案：chanlun-soros K4 weekly pipeline 的拓扑分离

## 存在论位置

| 范畴 | 主体 | 位置 |
|------|------|------|
| 操作域 | chanlun-soros pipeline + launchd | `~/Projects/chanlun-soros/`（standalone）+ `~/Library/LaunchAgents/` |
| 汇报域 | Cowork sandbox 读 + 自然语言整理 | `~/Projects/NewChanlun/external/k4-soros-reports/`（sandbox mount） |
| 契约层 | `last_run.json` + `YYYY-MM-DD.md` | 操作域写、汇报域读 |

操作和汇报分离——chanlun-soros 不被绑定进 NewChanlun 的 git 历史，
sandbox 也不需要执行能力，只需要读取能力。

## 安装步骤（你做）

```bash
# 1. 拷贝 wrapper 到 chanlun-soros
cp ~/Projects/NewChanlun/external/setup-c/launchd_wrapper.sh ~/Projects/chanlun-soros/scripts/
chmod +x ~/Projects/chanlun-soros/scripts/launchd_wrapper.sh

# 2. 验证 wrapper 能跑通（手动触发一次）
~/Projects/chanlun-soros/scripts/launchd_wrapper.sh
cat ~/Projects/NewChanlun/external/k4-soros-reports/last_run.json
ls ~/Projects/NewChanlun/external/k4-soros-reports/

# 3. 如果第 2 步 last_run.json 显示 status: "ok"，注册 launchd
cp ~/Projects/NewChanlun/external/setup-c/com.junyu.k4-weekly.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/com.junyu.k4-weekly.plist

# 4. 验证 launchd 注册成功
launchctl list | grep k4-weekly
```

## 故障排查

| 症状 | 原因 | 修复 |
|------|------|------|
| `last_run.json` 显示 exit_code 127 | chanlun-soros 路径不对 | 检查 `~/Projects/chanlun-soros/` 存在 |
| `last_run.json` 显示 status: "failed" + stderr | pipeline 内部错误 | 看 `stderr_tail` 字段，常见是 yfinance/FRED 404 |
| `launchctl list` 没有 k4-weekly | plist 加载失败 | `launchctl load -w ~/Library/LaunchAgents/com.junyu.k4-weekly.plist` 看错误 |
| 周一过了但 `last_run.json` 时间戳没变 | launchd 没触发（机器睡眠/未登录） | 改 `StartCalendarInterval` 或加 `RunAtLoad: true` |

## 契约：last_run.json schema

```json
{
  "timestamp": "ISO 8601 UTC",        // cron 跑完的时刻
  "date": "YYYY-MM-DD",                // 跑的那天本地日期
  "exit_code": 0,                      // pipeline 退出码（0=成功）
  "status": "ok | failed | fatal | not_yet_run",
  "latest_report": "YYYY-MM-DD.md",    // 报告文件名（reports 目录内）
  "source_output": "path/to/...",      // chanlun-soros/outputs 内的源文件
  "stderr_tail": "..."                 // 最后 2KB stderr，失败时填
}
```

## sandbox 端如何消费（scheduled task SKILL.md 行为）

每周一 scheduled task 触发时：

1. 读 `~/Projects/NewChanlun/external/k4-soros-reports/last_run.json`
2. 校验 `timestamp` 距今 < 8 天 → 否则报警"cron 死了"
3. 校验 `status == "ok"` → 否则把 `stderr_tail` 报出来
4. 读 `latest_report` 指向的 .md 文件
5. 用 v0.2 classifier 验收基准交叉检查
6. 用中文整理表格 + 异常 + 上周对比

新版 SKILL.md 见 `external/setup-c/SKILL_v2.md`。

## 谱系

- 161号：务实留下的矛盾——本方案承认 sandbox/cron 边界，用契约层弥合而非消除
- 089号：操作域与汇报域的扬弃分离
- spec-execution-gap：声明能力 ≠ 实际能力，本方案 SKILL_v2.md 严格声明 sandbox 只读

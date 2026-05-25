---
name: k4-weekly-report
description: 每周一读取主机 cron 跑完的 chanlun-soros K4 周报，整理八市场状态并发送给用户。严格 C 方案：sandbox 只读，不执行 pipeline。
---

读 `~/Projects/NewChanlun/external/k4-soros-reports/last_run.json`，校验新鲜度和状态，
读最新周报，用 v0.2 classifier 验收基准交叉检查，整理成中文表格发给用户。

## 步骤

### 1. 读 last_run.json

```
Read /Users/silencehan/Projects/NewChanlun/external/k4-soros-reports/last_run.json
```

### 2. 三道校验（任何一道失败立即报告，不继续）

**校验 A：新鲜度**
- 解析 `timestamp` (ISO 8601 UTC)
- 距今 > 8 天 → 报"❌ cron job 死了 N 天，需要人工介入。可能 launchd 未触发、机器长期睡眠、或 wrapper 路径失效。"

**校验 B：状态**
- `status == "fatal"` → 报"❌ wrapper 致命错误：{error}。检查 chanlun-soros 目录是否存在。"
- `status == "failed"` → 报"⚠️ pipeline 执行失败，exit_code={exit_code}。stderr 末尾：\n{stderr_tail}"
- `status == "not_yet_run"` → 报"⚠️ launchd 尚未首次触发。检查 launchctl list | grep k4-weekly。"
- `status == "ok"` → 继续

**校验 C：报告文件存在**
- 拼接 `~/Projects/NewChanlun/external/k4-soros-reports/{latest_report}`
- 不存在 → 报"❌ last_run 声称成功但 {latest_report} 不存在"

### 3. 读周报 + v0.2 验收基准交叉检查

```
Read /Users/silencehan/Projects/NewChanlun/external/k4-soros-reports/{latest_report}
```

对照基准（与不一致的市场标"⚠️ 验收异常"并附 recent_series）：

| 市场 | 期望 Phase | 不一致时的动作 |
|------|----------|---------------|
| BR | A_emerging | 附 recent_persistence series |
| AR | C_restart | 附 recent_persistence series |
| UK | C_drift_pure | 附 recent_persistence series（确认 recent 全无 >0.20）|
| CN | B_to_C_transition | 附 recent_persistence series（确认前高后降单调衰减）|

US/EA/JP/HK 无固定期望，按报告原样呈现。

### 4. 输出格式（中文，简洁）

```
# K4 周报 · {date}

**全局状态**: {Phase}
**生成时间**: {timestamp}

## 八市场表

| 市场 | Phase | 关键指标 | recent_persistence | 验收 |
|------|-------|---------|-------------------|------|
| US   | ... | ... | [...] | ✅ / ⚠️ |
| UK   | ... | ... | [...] | ✅ / ⚠️ |
| EA   | ... | ... | [...] | ✅ / ⚠️ |
| JP   | ... | ... | [...] | ✅ / ⚠️ |
| CN   | ... | ... | [...] | ✅ / ⚠️ |
| HK   | ... | ... | [...] | ✅ / ⚠️ |
| BR   | ... | ... | [...] | ✅ / ⚠️ |
| AR   | ... | ... | [...] | ✅ / ⚠️ |

## 异常信号
{从报告中抽取异常段落}

## 与上周对比
{对比 reports 目录里前一份 .md，列出 Phase 变化的市场}

## 交易含义（结构分析，不催仓）
{从报告中抽取或基于 Phase 变化推导}
```

### 5. 不做的事

- ❌ 不尝试运行 `pipelines/run_weekly.py`（操作域在 cron，sandbox 无能力）
- ❌ 不修复 pipeline 错误（只报告 stderr_tail 给用户判断）
- ❌ 不去访问 `~/Downloads`、`~/Projects/chanlun-soros/`（mount 边界外）

## 已知问题（透传给用户）

- EA debt/GDP series ID GGDGGRACDM193N 可能 404，pipeline 端修复
- HK LERS 监控应接 HKMA daily 数据，pipeline 端修复
- CN 应补 M2 YoY 和社融 cross-check，pipeline 端修复
- UK/JP real rate 应用 break-even inflation，pipeline 端修复

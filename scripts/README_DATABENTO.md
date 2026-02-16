# DataBento 数据获取说明

## 概述

本脚本用于从 DataBento API 获取美股分钟级 OHLCV 数据，用于缠论引擎验证。

## 环境配置

### 1. API Key 配置

API key 已配置在项目根目录的 `.env` 文件中：

```bash
DATABENTO_API_KEY=db-Uxvgeicpp7L4w9NFpTXTQSJn6LwP7
```

**注意**: `.env` 文件已被 `.gitignore` 忽略，不会被提交到 git。

### 2. 安装依赖

```bash
pip install databento python-dotenv
```

## 使用方法

### 基本用法

```bash
# 默认获取 AAPL 最近 5 个交易日的数据
python scripts/fetch_databento.py

# 指定股票代码
python scripts/fetch_databento.py TSLA

# 指定股票代码和天数
python scripts/fetch_databento.py TSLA 10
```

### 参数说明

- `symbol`: 股票代码（默认: AAPL）
- `days_back`: 往回获取的天数（默认: 5）

## 输出

### 数据文件

数据保存在 `data/` 目录下，文件名格式：

```
{symbol}_{start_date}_{end_date}_1m.csv
```

例如：`AAPL_20260209_20260213_1m.csv`

### CSV 格式

| 列名 | 说明 |
|------|------|
| timestamp | 时间戳（UTC，1分钟级别） |
| open | 开盘价 |
| high | 最高价 |
| low | 最低价 |
| close | 收盘价 |
| volume | 成交量 |

## 注意事项

### 数据延迟

DataBento 的历史数据通常有 1-2 天的延迟。脚本已自动处理，使用 `today - 2 天` 作为结束日期。

### 数据集说明

- 使用数据集: `XNAS.ITCH` (Nasdaq)
- Schema: `ohlcv-1m` (1分钟 OHLCV 聚合)
- 适用于美股主要交易所的股票

### 错误处理

脚本包含自动重试机制（最多 2 次重试）。如果失败，可能的原因：

1. API key 无效或过期
2. 网络连接问题
3. 股票代码不存在或不在 XNAS.ITCH 数据集中
4. DataBento 账户权限或配额限制

## 示例输出

```
============================================================
DataBento 数据获取脚本
============================================================
正在获取 AAPL 数据...
日期范围: 2026-02-09 至 2026-02-14
API Key 前缀: db-Uxvgeic...
成功获取 4035 条数据记录
数据范围: 2026-02-09 09:00:00+00:00 至 2026-02-13 23:59:00+00:00
数据已保存至: /home/user/NewChanlun/data/AAPL_20260209_20260213_1m.csv
文件大小: 229.32 KB

数据预览（前5行）:
                  timestamp    open    high     low   close  volume
0 2026-02-09 09:00:00+00:00  277.65  277.65  276.77  277.00     371
1 2026-02-09 09:01:00+00:00  277.00  277.00  276.67  276.67     303
...

============================================================
数据获取完成！
============================================================
```

## 文件组织

```
NewChanlun/
├── .env                          # API key 配置（已在 .gitignore 中）
├── scripts/
│   ├── fetch_databento.py        # 数据获取脚本
│   └── README_DATABENTO.md       # 本说明文档
└── data/                         # 数据输出目录（已在 .gitignore 中）
    └── AAPL_20260209_20260213_1m.csv
```

## 安全提醒

- **不要** 在任何被 git 追踪的文件中硬编码 API key
- `.env` 文件已被 `.gitignore` 排除
- `data/` 目录已被 `.gitignore` 排除
- 如果需要分享配置，使用 `.env.example` 作为模板

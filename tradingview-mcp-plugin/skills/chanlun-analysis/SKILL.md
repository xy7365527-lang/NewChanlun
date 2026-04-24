---
name: chanlun-analysis
description: >
  Analyze Chanlun (Chan Theory) structures on TradingView charts. Use when the user asks to
  "read chanlun indicators", "check buy/sell points", "analyze chart structure",
  "look at the centers/zhongshu", "check divergence", "scan for setups",
  or any request involving Chanlun technical analysis on TradingView data.
---

# Chanlun Analysis via TradingView

Analyze Chanlun (缠论) structures by reading indicator data from TradingView Desktop.

## Prerequisites

TradingView Desktop must be running with `--remote-debugging-port=9222`. Launch with:
```
/Applications/TradingView.app/Contents/MacOS/TradingView --remote-debugging-port=9222
```

Verify connection: call `tv_health_check`.

## Core Tools

| Task | Tool | Notes |
|------|------|-------|
| Check connection | `tv_health_check` | Always run first |
| Get current chart info | `chart_get_state` | Symbol, timeframe, loaded indicators |
| Switch symbol | `chart_set_symbol` | e.g. "BATS:MU", "NYMEX:CL1!" |
| Switch timeframe | `chart_set_timeframe` | "D", "240", "60", "30", "W" |
| Read buy/sell points | `data_get_pine_labels` | Returns labels with text + price |
| Read centers (zhongshu) | `data_get_pine_boxes` | Returns boxes with high/low/color |
| Read strokes/segments | `data_get_pine_lines` | Returns lines |
| Get quote | `quote_get` | Current OHLCV |
| Get OHLCV data | `data_get_ohlcv` | Historical bars |
| Take screenshot | `capture_screenshot` | Chart image |

## Indicator Color Mapping

The "Chan Theory - CHANLUN | CZSC" indicator uses these colors for boxes:

| Color | bgColor value | Level | Meaning |
|-------|---------------|-------|---------|
| Blue | — | Stroke (笔) | Three-stroke overlap center |
| Orange | 872064120 | Segment (线段) | Three-segment overlap center |
| Purple | 856729599 | Trend (趋势) | Multi-segment-center structure |

## Label Format

Labels follow this pattern:
- Buy/sell points: `" 1卖(趋势) 116"` — type + MACD area
- Pure numbers: `"  106"` or `"  -50"` — MACD area at stroke/segment endpoints
- Positive = upward movement, Negative = downward movement

## Reading Structure

When analyzing a chart:

1. Call `chart_get_state` to confirm symbol and timeframe
2. Call `data_get_pine_boxes` with `verbose: true` to get all centers with colors
3. Call `data_get_pine_labels` to get buy/sell points and MACD areas
4. Classify boxes by bgColor:
   - 872064120 → segment-level centers (操作级别)
   - 856729599 → trend-level centers (更大级别)
5. Find the most recent centers and buy/sell points
6. Report structure: current level, nearest center high/low, latest buy/sell point, divergence status

## Multi-Timeframe Analysis

For nested analysis (区间套):
1. Read structure on higher timeframe (e.g. daily)
2. Switch to lower timeframe: `chart_set_timeframe`
3. Read structure on lower timeframe
4. Compare: same-direction buy/sell points at overlapping price zones = nested confirmation

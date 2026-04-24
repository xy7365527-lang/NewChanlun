# TradingView Chanlun Plugin

Connect Claude to your TradingView Desktop app for real-time chart analysis with Chanlun (Chan Theory) indicators.

## Prerequisites

1. **TradingView Desktop** installed and running
2. **Node.js 18+** installed
3. **tradingview-mcp** server installed at `~/Projects/tradingview-mcp/`

## Setup

Launch TradingView in debug mode:
```bash
/Applications/TradingView.app/Contents/MacOS/TradingView --remote-debugging-port=9222
```

## What this plugin provides

- **78 TradingView MCP tools** — full access to chart data, indicators, Pine Script, quotes, drawings, alerts, and more
- **Chanlun Analysis skill** — specialized guidance for reading Chan Theory structures (buy/sell points, centers/zhongshu, divergence) from TradingView indicators

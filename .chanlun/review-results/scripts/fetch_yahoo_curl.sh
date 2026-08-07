#!/bin/bash
# Fetch Yahoo hourly chart JSON via curl (urllib gets 429'd; curl HTTP/2 UA works).
set -u
cd "$(dirname "$0")/.."
mkdir -p data
UA="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
SYMS="AAPL NVDA TSLA MSFT META GOOGL AMZN SPY QQQ COST NFLX JPM LLY"
: > data/yahoo_fetch_errors.log
for sym in $SYMS; do
  code=$(curl -s --max-time 30 -H "User-Agent: $UA" \
    "https://query2.finance.yahoo.com/v8/finance/chart/${sym}?interval=1h&range=2y&events=div,split" \
    -o "data/yahoo_${sym}.json" -w "%{http_code}")
  echo "$sym: HTTP $code"
  if [ "$code" != "200" ]; then
    echo "$sym: HTTP $code $(cat data/yahoo_${sym}.json)" >> data/yahoo_fetch_errors.log
  fi
  sleep 3
done

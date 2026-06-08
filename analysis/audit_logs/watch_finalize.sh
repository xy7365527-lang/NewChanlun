#!/bin/zsh
cd /Users/silencehan/Projects/NewChanlun
for i in $(seq 1 240); do
  if [ -f analysis/data_cache/fugue_no_addon_audit_results.json ]; then
    sleep 5
    python analysis/synth_audit_report.py >> analysis/audit_logs/finalize.log 2>&1
    echo "FINALIZED at $(date)" >> analysis/audit_logs/finalize.log
    exit 0
  fi
  sleep 60
done

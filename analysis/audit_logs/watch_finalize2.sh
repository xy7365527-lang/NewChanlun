#!/bin/zsh
cd /Users/silencehan/Projects/NewChanlun
M=analysis/audit_logs/rerun_marker
NA=analysis/data_cache/fugue_no_addon_audit_results.json
V3=analysis/data_cache/full_system_v3_audit_results.json
for i in $(seq 1 360); do
  if [ -f "$NA" ] && [ "$NA" -nt "$M" ] && [ -f "$V3" ] && [ "$V3" -nt "$M" ]; then
    sleep 5
    python analysis/synth_audit_report.py >> analysis/audit_logs/finalize2.log 2>&1
    echo "FINALIZED(挣股数版) at $(date)" >> analysis/audit_logs/finalize2.log
    exit 0
  fi
  sleep 60
done

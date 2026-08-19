#!/usr/bin/env bash
# Fail-loud DevSkim SARIF gate for #1108.
# The ordinary Actions artifact is evidence only; it is not GitHub Code scanning.

set -u

usage() {
  cat <<'EOF'
Usage: devskim_sarif_gate.sh \
  --sarif PATH \
  --scanner-outcome OUTCOME \
  --artifact-name NAME \
  --artifact-outcome OUTCOME \
  --retention-days DAYS \
  [--summary PATH]
EOF
}

sarif_path=''
scanner_outcome=''
artifact_name=''
artifact_outcome=''
retention_days=''
summary_path=''

while [[ $# -gt 0 ]]; do
  case "$1" in
    --sarif)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      sarif_path="$2"
      shift 2
      ;;
    --scanner-outcome)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      scanner_outcome="$2"
      shift 2
      ;;
    --artifact-name)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      artifact_name="$2"
      shift 2
      ;;
    --artifact-outcome)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      artifact_outcome="$2"
      shift 2
      ;;
    --retention-days)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      retention_days="$2"
      shift 2
      ;;
    --summary)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      summary_path="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'Unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$sarif_path" || -z "$scanner_outcome" || -z "$artifact_name" || \
      -z "$artifact_outcome" || -z "$retention_days" ]]; then
  printf 'All arguments except --summary are required.\n' >&2
  usage >&2
  exit 2
fi

errors=''
add_error() {
  if [[ -n "$errors" ]]; then
    errors="$errors"$'\n'
  fi
  errors="$errors- $1"
}

if [[ "$scanner_outcome" != 'success' ]]; then
  add_error "DevSkim scanner outcome was \`$scanner_outcome\`, not \`success\`."
fi

if [[ "$artifact_outcome" != 'success' ]]; then
  add_error "Ordinary artifact upload outcome was \`$artifact_outcome\`, not \`success\`."
fi

finding_count='unknown'
if [[ ! -f "$sarif_path" ]]; then
  add_error "SARIF file is missing: \`$sarif_path\`."
elif ! command -v jq >/dev/null 2>&1; then
  add_error 'jq is unavailable, so the SARIF cannot be parsed.'
else
  schema_filter='length == 1 and (.[0] |
    type == "object" and
    .version == "2.1.0" and
    (.runs | type == "array" and length > 0 and all(.[];
      type == "object" and
      (.tool | type == "object") and
      (.tool.driver | type == "object") and
      (.tool.driver.name | type == "string" and length > 0) and
      (.results | type == "array")
    )))'
  schema_error=''
  if ! schema_error="$(jq -e --slurp "$schema_filter" "$sarif_path" 2>&1)"; then
    add_error 'SARIF JSON parsing or required 2.1.0 runs/tool.driver/results structure failed.'
    if [[ -n "$schema_error" ]]; then
      printf '%s\n' "$schema_error" >&2
    fi
  else
    if ! finding_count="$(jq -er '[.runs[] | (.results // [])[]] | length' "$sarif_path" 2>&1)"; then
      add_error 'SARIF findings could not be counted.'
      printf '%s\n' "$finding_count" >&2
      finding_count='unknown'
    elif [[ ! "$finding_count" =~ ^[0-9]+$ ]]; then
      add_error "SARIF finding count is not an integer: \`$finding_count\`."
    elif [[ "$finding_count" -gt 0 ]]; then
      add_error "SARIF contains $finding_count finding(s); this gate requires zero."
    fi
  fi
fi

if [[ -z "$errors" ]]; then
  gate_conclusion='PASS — scanner and artifact upload succeeded, SARIF parsed, and findings are zero.'
  gate_status=0
else
  gate_conclusion='FAIL — scanner, evidence upload, SARIF validity, or zero-findings requirement failed.'
  gate_status=1
fi

summary='## DevSkim security gate'
summary="$summary"$'\n\n'"- **GitHub Security tab:** unavailable under the current repository entitlement. The file below is an ordinary Actions artifact, **not** GitHub Code scanning."
summary="$summary"$'\n'"- **Artifact:** \`$artifact_name\` (upload outcome: \`$artifact_outcome\`; retention: $retention_days days)"
summary="$summary"$'\n'"- **Scanner outcome:** \`$scanner_outcome\`"
summary="$summary"$'\n'"- **SARIF findings:** \`$finding_count\`"
summary="$summary"$'\n'"- **Gate conclusion:** $gate_conclusion"

if [[ -n "$errors" ]]; then
  summary="$summary"$'\n\n'"### Failure details"$'\n\n'"$errors"
fi

printf '%s\n' "$summary"
if [[ -n "$summary_path" ]] && ! printf '%s\n' "$summary" >>"$summary_path"; then
  printf 'Failed to append the DevSkim gate summary to %s.\n' "$summary_path" >&2
  exit 1
fi

exit "$gate_status"

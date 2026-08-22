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
  [--expected-tool-name NAME] \
  [--expected-tool-version VERSION] \
  [--baseline PATH] \
  [--summary PATH]
EOF
}

sarif_path=''
scanner_outcome=''
artifact_name=''
artifact_outcome=''
retention_days=''
expected_tool_name=''
expected_tool_version=''
baseline_path=''
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
    --expected-tool-name)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      expected_tool_name="$2"
      shift 2
      ;;
    --expected-tool-version)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      expected_tool_version="$2"
      shift 2
      ;;
    --baseline)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      baseline_path="$2"
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
  printf 'One or more required arguments are missing.\n' >&2
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
tool_name='unknown'
tool_version='unknown'
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
    if ! tool_name="$(jq -er '[.runs[].tool.driver.name] | unique |
      if (length == 1 and (.[0] | type == "string" and length > 0))
      then .[0]
      else error("expected exactly one non-empty tool.driver.name")
      end' "$sarif_path" 2>&1)"; then
      add_error 'SARIF DevSkim tool name is missing or inconsistent across runs.'
      printf '%s\n' "$tool_name" >&2
      tool_name='unknown'
    elif [[ -n "$expected_tool_name" && "$tool_name" != "$expected_tool_name" ]]; then
      add_error "SARIF DevSkim tool name was \`$tool_name\`, expected \`$expected_tool_name\`."
    fi

    if ! tool_version="$(jq -er '[.runs[].tool.driver.version] | unique |
      if (length == 1 and (.[0] | type == "string" and length > 0))
      then .[0]
      else error("expected exactly one non-empty tool.driver.version")
      end' "$sarif_path" 2>&1)"; then
      add_error 'SARIF DevSkim tool version is missing or inconsistent across runs.'
      printf '%s\n' "$tool_version" >&2
      tool_version='unknown'
    elif [[ -n "$expected_tool_version" && "$tool_version" != "$expected_tool_version" ]]; then
      add_error "SARIF DevSkim tool version was \`$tool_version\`, expected \`$expected_tool_version\`."
    fi

    if ! finding_count="$(jq -er '[.runs[] | (.results // [])[]] | length' "$sarif_path" 2>&1)"; then
      add_error 'SARIF findings could not be counted.'
      printf '%s\n' "$finding_count" >&2
      finding_count='unknown'
    elif [[ ! "$finding_count" =~ ^[0-9]+$ ]]; then
      add_error "SARIF finding count is not an integer: \`$finding_count\`."
    elif [[ -n "$baseline_path" ]]; then
      # #1194 基线语义：只拦基线外新增；消失键只作信息行。
      if [[ ! -f "$baseline_path" ]]; then
        add_error "Baseline file is missing: \`$baseline_path\`."
      else
        if ! current_keys="$(jq -r '
          [.runs[] | (.results // [])[]
           | (.ruleId // "?") + "|" +
             ((.locations[0].physicalLocation.artifactLocation.uri // "?") | sub("^file://"; "")) + "|" +
             (((.locations[0].physicalLocation.region.startLine // 0) | tostring))]
          | unique | .[]' "$sarif_path" 2>&1)"; then
          add_error 'SARIF finding keys could not be extracted.'
          printf '%s\n' "$current_keys" >&2
        else
          if ! baseline_ok="$(jq -er '
            .schema == "devskim-baseline/v1"
            and (.findings | type == "array")
            and ([.findings[] | type == "string"] | all)
          ' "$baseline_path" 2>&1)"; then
            add_error "Baseline schema mismatch or unreadable: \`$baseline_path\`."
            printf '%s\n' "$baseline_ok" >&2
          else
            baseline_list="$(jq -r '.findings[]' "$baseline_path")"
            new_findings="$(comm -13 \
              <(printf '%s\n' "$baseline_list" | sort) \
              <(printf '%s\n' "$current_keys" | sort))"
            fixed_count="$(comm -23 \
              <(printf '%s\n' "$baseline_list" | sort) \
              <(printf '%s\n' "$current_keys" | sort) | wc -l | tr -d ' ')"
            fixed_count="${fixed_count:-0}"
            if [[ -n "$new_findings" ]]; then
              new_count="$(printf '%s\n' "$new_findings" | wc -l | tr -d ' ')"
              add_error "SARIF contains $new_count finding(s) not in the baseline (total $finding_count). New findings:"
              while IFS= read -r key; do
                add_error "  - \`$key\`"
              done <<< "$new_findings"
            fi
            if [[ "$fixed_count" != "0" ]]; then
              baseline_fixed_note="Fixed since baseline: $fixed_count finding(s)."
            else
              baseline_fixed_note=''
            fi
          fi
        fi
      fi
    elif [[ "$finding_count" -gt 0 ]]; then
      add_error "SARIF contains $finding_count finding(s); this gate requires zero."
    fi
  fi
fi

if [[ -z "$errors" ]]; then
  if [[ -n "$baseline_path" ]]; then
    gate_conclusion='PASS — scanner and artifact upload succeeded, SARIF/tool contract matched, and no findings beyond the baseline.'
  else
    gate_conclusion='PASS — scanner and artifact upload succeeded, SARIF/tool contract matched, and findings are zero.'
  fi
  gate_status=0
else
  gate_conclusion='FAIL — scanner, evidence upload, SARIF/tool contract, baseline comparison, or zero-findings requirement failed.'
  gate_status=1
fi

summary='## DevSkim security gate'
summary="$summary"$'\n\n'"- **GitHub Security tab:** unavailable under the current repository entitlement. The file below is an ordinary Actions artifact, **not** GitHub Code scanning."
summary="$summary"$'\n'"- **Artifact:** \`$artifact_name\` (upload outcome: \`$artifact_outcome\`; retention: $retention_days days)"
summary="$summary"$'\n'"- **Scanner outcome:** \`$scanner_outcome\`"
if [[ -n "$expected_tool_name" ]]; then
  summary="$summary"$'\n'"- **DevSkim tool name:** \`$tool_name\` (expected: \`$expected_tool_name\`)"
else
  summary="$summary"$'\n'"- **DevSkim tool name:** \`$tool_name\` (no expected name configured)"
fi
if [[ -n "$expected_tool_version" ]]; then
  summary="$summary"$'\n'"- **DevSkim tool version:** \`$tool_version\` (expected: \`$expected_tool_version\`)"
else
  summary="$summary"$'\n'"- **DevSkim tool version:** \`$tool_version\` (no expected version configured)"
fi
summary="$summary"$'\n'"- **SARIF findings:** \`$finding_count\`"
if [[ -n "$baseline_path" ]]; then
  summary="$summary"$'\n'"- **Baseline:** \`$baseline_path\`"
  if [[ -n "${baseline_fixed_note:-}" ]]; then
    summary="$summary"$'\n'"- **$baseline_fixed_note**"
  fi
fi
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

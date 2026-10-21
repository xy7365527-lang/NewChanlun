run(){ echo "=== $*"; "$@" > /tmp/o.txt 2>&1; echo "rc=$?"; grep -E "^test result:" /tmp/o.txt | awk '{p+=$4; f+=$6; i+=$8} END {print "passed="p" failed="f" ignored="i}'; grep -E "^    [A-Za-z_:0-9]+$|panicked at" /tmp/o.txt | head -12; }
run cargo test --locked --all-targets --no-fail-fast
run cargo test --locked --features s_session --bin s_structure_session -- --skip v3::financial_session
run cargo test --locked --features s_session --lib s_content
run cargo test --locked --features s_session --test s_content_contract --test s_content_sequence --test s_content_job --test s_content_path
run cargo test --locked --features s_session --lib -- economic_session session_transport session_protocol
run ../.github/scripts/test-feature-delta.sh s_capture_native_testonly
run cargo test --locked --features s_session,s_allocation_json_testonly --bin s_structure_session v3::history
run cargo test --locked --lib --features backtest_bin

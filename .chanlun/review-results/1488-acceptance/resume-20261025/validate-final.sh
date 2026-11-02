set +e
failed=0
run() { name=$1; shift; echo "=== BEGIN $name ==="; "$@"; rc=$?; echo "=== END $name RC=$rc ==="; [ "$rc" = 0 ] || failed=1; }
run fmt cargo fmt --all -- --check
run default env RUSTFLAGS='-D warnings' cargo check --locked --all-targets
run backtest_bin env RUSTFLAGS='-D warnings' cargo check --locked --all-targets --features backtest_bin
run s_session env RUSTFLAGS='-D warnings' cargo check --locked --all-targets --features s_session
run native env RUSTFLAGS='-D warnings' cargo check --locked --all-targets --features s_session,s_capture_native_testonly
run json_native env RUSTFLAGS='-D warnings' cargo check --locked --all-targets --features s_session,s_capture_native_testonly,s_allocation_json_testonly
run bsp_default cargo test --locked --lib no_stray_bsp_binding_rewrites_outside_registry
run bsp_backtest cargo test --locked --lib --features backtest_bin no_stray_bsp_binding_rewrites_outside_registry
run business_native cargo test --locked --bin s_structure_session --features s_session,s_capture_native_testonly v3::business_json::
run business_json_native cargo test --locked --bin s_structure_session --features s_session,s_capture_native_testonly,s_allocation_json_testonly v3::business_json::
exit $failed

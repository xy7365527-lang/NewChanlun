## S1T：修复 48D B1 的材料变化重冷判别

关联票：#1481。基线：`ae46a1273b6bb3babfcf958a3f2d970c83784280`。
依据：`/Volumes/AgentStorage/h1-s1s-review48d/REPORT.md` 第 11–17 行 B1。

仅修改 `rust/src/bin/s_session_v3/driver.rs` 的 `native_prefix_context_rechecks_real_connection_current_material`。前缀仍断言无 slot、计数为 `(0,0)`；mutation 前先实际执行一次 startup，断言 `(1,0)` 且已有 slot。mutation 后第二次 startup，`same` 必须命中至 `(1,1)`，`append` 必须重冷至 `(2,0)`。错误分支仍逐字比较 oracle 错误，并新增失败后 slot 已清断言。

六类 change、mutation 操作及 oracle 计算逐字未变。外层 `#[cfg(test)]` 与本测试 `#[cfg(feature = "s_incremental")]`、`#[test]` 保持不变；生产代码和其他测试未改。禁串检查确认生成表与源码一致，无需更新生成物。

实际检查：

- `rustfmt --edition 2021 --check rust/src/bin/s_session_v3/driver.rs`：通过。
- `git diff --check`：通过。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/check-prod-no-test-hooks.py`：通过，`violations=0`。
- `PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/gen-prod-banned-literals.py --check .`：通过，禁串表与源码一致。
- 用 Python 与 HEAD 逐字比较，确认指定测试外内容、cfg、前缀断言、六类 mutation 和 oracle 未变，并核对两次 startup 的先后顺序及分支断言：通过。

未运行 cargo/rustc，Rust 编译与测试执行未验证，待 CI。不执行 git add、commit、push，不写 Git metadata。

# Gap-4 theta_overlay.rs:80 编译修复（E0609）

- 工位：worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`
- 日期：2026-07-19（任务指定产出文件名日期）
- 改动范围：`rust/src/bin/theta_overlay.rs` 一行；未动 `rust/Cargo.toml`；无 git mutation；主仓未写。

## 根因

L1-P3 实装把 `ThetaOverlayResult` 的字段 `n_overlay_orders` 诚实化改名为
`n_overlay_fill_events`（`rust/src/theta_v0/backtest/runner.rs:659`，赋值语义见
`runner.rs:765` 的 `fill.n_orders`；`runner.rs:662` 注释说明旧名以"声部数冒充订单数"
属 090 违规）。`rust/src/bin/theta_overlay.rs:80` 仍引用旧字段名，触发 E0609
编译失败。

## 改动 diff（一行）

```diff
diff --git a/rust/src/bin/theta_overlay.rs b/rust/src/bin/theta_overlay.rs
@@ -77,7 +77,7 @@ fn main() -> std::process::ExitCode {
     println!("--- ★M5 overlay 逐声部账本（多空对冲.pdf p16 关卡10）---");
     println!("活动声部数      : {}", r.overlay.active_voices().count());
     println!("已离场声部数    : {}", r.overlay.closed_voices().len());
-    println!("overlay 订单数  : {}", r.n_overlay_orders);
+    println!("overlay 成交事件数: {}", r.n_overlay_fill_events);
     println!("终态净敞口 N    : {}", r.overlay.net());
```

同一行内顺手把展示标签 `overlay 订单数` 改为 `overlay 成交事件数`——字段语义是
fill 事件计数，沿用"订单数"标签会让打印声明与实际能力不符（090）。仍为一行改动。

## 验证闸门（均实跑，非推断）

1. `cargo build --release --features backtest_bin --bin theta_overlay`（cwd `rust/`）
   - exit 0，`Finished `release` profile [optimized] target(s) in 11.83s`
   - 仅剩存量 dead-code 警告（86 条，lib 侧既有，与本次改动无关）。
2. `cargo test --lib`（cwd `rust/`）
   - exit 0；`test result: ok. 1737 passed; 0 failed; 128 ignored; 0 measured; 0 filtered out; finished in 6.45s`
   - 零变红。

## 结论

E0609 已修复，编译闸门与测试闸门全绿。改动为一行、语义诚实化，符合 090 与 v3 纪律。

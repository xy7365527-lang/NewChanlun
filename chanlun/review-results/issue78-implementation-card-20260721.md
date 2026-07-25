# #78 实装卡：假反超 / as_of 单调守卫 / 声明越界 / exec-1 下溢（code-review major 修复）

- 日期：2026-07-21
- 来源：`chanlun/review-results/code-review-v1-v3-64-20260721.md`（Standards #1/#2/#5、Spec (c)1/(c)2）；issue #78。
- 前置：#75 落地后开工（同 crate 串行，避免编译互踩）。
- 纪律：禁 git mutation；禁改 Cargo.toml；只改本卡授权文件；TDD 先行；`cargo test --release --lib` 基线以 #75 落地后为准（当前 1794），零变红。

## 修复 1：假反超（缺数据判负 → 终态吸收）

**缺陷**：`nest_lifecycle.rs:811-819` 活窗 hist/dif 缺失或坐标映射失败时 `force_ok = false`（「诚实判负」），`advance` 第 5 步（:620）`!obs.force_ok → Invalidated(ForceOvertake)` 终态禁复活。「不可验」≠「不再弱」（V3 卡 §4.1 反超定义：曾可证 ∧ 当前不再弱）——一个缺数据 prefix 可永久杀死身份。

**设计**：力度检查产出从 bool 升格三值：

```rust
enum ForceCheck {
    Verified(bool),        // 数据齐备，力度判定结论（true=仍弱/不更強，false=反超成立）
    Unavailable(UnavailReason), // 数据缺失/坐标映射失败——不可验
}
```

- `Unavailable` 时 advance **不判 Invalidated**：entry 保持原态（Provisional 留 Provisional），revision 记 `ForceUnavailable(reason)` 审计注记（可计数次，不触发终态）。
- 数据补齐后（后续 prefix hist/dif 到位）正常恢复推进——身份不被缺数据杀死。
- `Verified(false)` 才走 Invalidated(ForceOvertake)（061:26 反超语义不变）。
- 语义边界（090）：本修复落实 V3 卡 §2.4 自认的「失败原因通道缺位」，不改反超定义本身，不需新裁定；advance 行为变化仅限 Unavailable 分支（Verified 路径逐 bit 不变）。

**测试（先行）**：①缺数据 prefix → entry 非终态 + Unavailable 审计注记在案；②缺数据后数据补齐 → 可继续推进至 Confirmed；③Verified(false) 仍判 Invalidated（T11 语义不退）；④Unavailable 不计入 Invalidated 统计。

## 修复 2：as_of 单调守卫

**缺陷**：Provisional 身份对倒退 prefix 无防御（T3 在 as_of=149 后以 125 喂入构造「复活」，:1090），可写出违反「一切钟 ≤ as_of」的钟（first_provable 取 as_of，:585）；现靠终态吸收侥幸成立。

**设计**：book 记录每身份 last_as_of；`advance`/`observe` 对 `as_of < last_as_of` 的喂入**显式拒绝**（返回 Err 或忽略 + 审计注记 `RetrogradeFeedRejected`），禁止静默接受。同 as_of 重复喂入幂等。

**测试（先行）**：①as_of 倒退喂入被拒 + 审计注记；②同 as_of 幂等；③T3 语义不退（合法路径照常）。

## 修复 3：声明越界两处（090）

1. `nest_lifecycle.rs:4-5` 模块头「唯一实装落点：新建本文件 + mod.rs 注册，既有行零改动」→ 改为如实描述：本切片新建本文件 + mod.rs 注册 + #64 授权面内 level_view.rs additive 产出（附裁定书编号 r43-lifecycle-ruling-20260721.md）。
2. `level_view.rs:699`「输出与迁移前逐 bit 相等」→ 二选一：(a) 补 golden 锁定（对迁移前 dump 的对拍测试——若迁移前 dump 不可得则不可选）；(b) 改声明为能力相符措辞：「legacy 为 ext 的薄包装（.event 映射），既有字段值经全量既有测试背书」（引用 T5 护栏 + p92/p95 下游测试）。**(a) 不可得时取 (b)，禁止保留无锁定承诺。**

## 修复 4：v1_level_calib_fix.rs:524 exec-1 下溢

`c.exec - 1`（usize）对 exec=0 下溢 panic。守卫：exec=0 显式跳过或返回 None + 注释（exec 自 1 起是 p105 §3 口径，0 为非法输入但不得 panic）。**测试**：exec=0 输入不 panic。

## 授权文件

- `rust/src/theta_v0/classifier/nest_lifecycle.rs`（修复 1/2/3.1）
- `rust/src/theta_v0/classifier/level_view.rs`（修复 3.2，仅注释/测试）
- `rust/src/bin/v1_level_calib_fix.rs`（修复 4）

## 验收线

- 全量 `cargo test --release --lib` 零变红（新测试使总数增加）。
- debug 模式（触发 debug_assert 钟序不变量）同跑全绿。
- T6/T11（Invalidated 既有语义）逐 bit 不退。
- 修复 1 的 Unavailable 分支有真实夹具覆盖（禁纯 mock）。

# #981 N7 Consume_at 签名冻结落地 — 收尾两轴评审

- 票：GitHub #981（`[task] N7 Consume_at 签名冻结落地：类型族 + 函数签名（逻辑 stub，map #529）`）
- worktree：`/tmp/wt-n7-freeze`（`git worktree add --detach ... main`，HEAD `8b167366e5`）
- 日期：2026-08-15
- 执行纪律：/implement + /tdd + /code-review，一次一个 tracer bullet，常跑四项门。

## 变更面（git diff --stat）

```
 rust/src/theta_v0/classifier/bsp_bridge.rs | 2 +-
 rust/src/theta_v0/classifier/mod.rs        | 3 +++
（新增、未跟踪）rust/src/theta_v0/classifier/consume_at.rs
```

判定链零改核对：`git diff --name-only` 仅 `bsp_bridge.rs` + `mod.rs`（`consume_at.rs` 为新增未跟踪文件，
不在 diff 内，属预期）。bsp_bridge.rs 全 diff 为一行 `Hash` derive（见下），mod.rs 全 diff 为
`pub mod consume_at;` 注册 + 两行文档注释。parser / classifier 其余 / chain_cert / bsp_bridge 结构 /
递归塔 **零改**。

## Standards 轴（逐条对照票面）

1. **`cargo fmt --check` 绿** —— 通过（`cargo fmt` 后 `cargo fmt --check` exit 0）。
2. **无 shotgun（新类型全在新文件，不改现役）** —— 通过。全部 8 个新类型/枚举 + 函数签名落
   `rust/src/theta_v0/classifier/consume_at.rs`；现役文件仅两处机械注册：
   - `mod.rs:108`：`pub mod consume_at;`
   - `bsp_bridge.rs:148`：`BspStructuralKey` 的 derive 加 `Hash`（一行，见 Spec 3）。
3. **模块头写明 seam/认识论等级/冻结语义来源** —— 通过。`consume_at.rs:1-27` 头注逐项写明：
   - Seam（一行，现役对象 → 新类型族 → consume_at 签名 stub）；
   - 认识论（formalization-validity-domain）：实装 = L1（签名/形状翻译正确），不声称 L2/L3；
   - 冻结语义来源：谱系综合 §2（`chanlun/review-results/e2eo-lineage-synthesis-20260728.md`）
     与 #980 裁定 A（BspKey = `BspStructuralKey`，episode 粒度，残余规则「跨多条链只创建一次」）。
4. **无反模式** —— 通过：
   - 实现耦合：`consume_at.rs` 只经现役对象**公共字段**与**公共构造点**（`ChainKey::new`、
     `BspStructuralKey`/`CandidateKey`/`ParentFingerprint` struct literal），未 mock 现役对象内部、
     未测私有路径。
   - 同语反复：测试断言「新类型可构造 + 字段可读 + 变体可区分 + 签名可调用」，非「喂 X 得 X」；
     构造输入（`candidate()`/`chain_key()`/`bsp_key()`）与被断言字段是不同对象/不同字段。
   - 水平切片：本票只做「冻结签名」一层——`consume_at` 函数体为 `todo!()`，消费逻辑归后续票，
     未碰任何消费逻辑。

## Spec 轴（逐条对照票面）

1. **`cargo check --lib` 绿（新类型 + 签名编译通过，`todo!()` 合法 stub）** —— 通过
   （exit 0；61 条 warning 均为仓内既有 dead-code 等，无本票文件 warning）。
2. **`cargo check --features backtest_bin` 绿** —— 通过（exit 0）。
3. **判定链零改** —— 通过。三文件改动面如上：`consume_at.rs`（新增）+ `mod.rs`（注册 3 行）+
   `bsp_bridge.rs`（仅 `Hash` derive 一行）。`BspStructuralKey` 原未 derive `Hash`
   （`bsp_bridge.rs:148` 原为 `#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]`），
   本票按票面在 `bsp_bridge.rs:148` **只加 `Hash`**、不改字段不改结构：
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
   pub struct BspStructuralKey {
   ```
   其余现役对象（chain_cert / bsp_bridge 结构 / parser / 递归塔）零改。
4. **类型级测试（`#[cfg(test)]`，单文件内）** —— 通过。`cargo test --lib
   theta_v0::classifier::consume_at::` 结果 **9 passed; 0 failed**，测试项：
   - 8 个新类型逐一「构造 + 字段可读」（ProjectionKey / LinkKey / ManagedBsp /
     ManagedBspCreation / BspLink / ManagedBspPolicy / PolicyRule / ConsumeError）；
   - `consume_at_has_frozen_signature`：把 `consume_at` 绑定到与票面逐字一致的函数指针类型
     `fn(usize, &HashMap<BspStructuralKey, ManagedBsp>, &HashMap<LinkKey, BspLink>,
     &TowerChainCertificate, &ManagedBspPolicy)
     -> Result<(Vec<ManagedBspCreation>, Vec<BspLink>), ConsumeError>`，
     以编译期类型断言锁死「签名可调用」，**不调用**（函数体是 `todo!()` stub，调用必 panic；
     票面明示允许「直接不调用 consume_at、只测类型构造」的等价形式）。

## 承重断言带 file:line

- `BspStructuralKey` 原无 `Hash`：`rust/src/theta_v0/classifier/bsp_bridge.rs:148`
  （原 `#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]`，已逐字核对）。
- `ChainKey` 已 derive `Hash`：`rust/src/theta_v0/classifier/chain_cert/mod.rs:288`
  （`#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]`）——零改。
- `TowerChainCertificate` 结构：`rust/src/theta_v0/classifier/chain_cert/mod.rs:334`——零改。
- `ChainStatus::Closed` 变体：`rust/src/theta_v0/classifier/chain_cert/mod.rs:96-100`——零改。
- 新类型落点（均在 `consume_at.rs`）：ProjectionKey `:36`、LinkKey `:46`、ManagedBsp `:57`、
  ManagedBspCreation `:64`、BspLink `:70`、ManagedBspPolicy `:79`、PolicyRule `:90`、
  ConsumeError `:98`、consume_at 签名 `:114`、测试 `:131`。
- 注册点：`rust/src/theta_v0/classifier/mod.rs:108` `pub mod consume_at;`。

## 验证命令（父会话可复跑）

```bash
cd /tmp/wt-n7-freeze/rust
cargo fmt --check
cargo check --lib
cargo check --features backtest_bin
cargo test --lib theta_v0::classifier::consume_at::
cd /tmp/wt-n7-freeze && git diff --name-only && git diff --stat
```

## 遗留 / 边界声明

- `consume_at` 逻辑（Closed 谱系门、幂等创建、BspLink 组内写入、四错误分支判据）**全部归后续票**，
  本票只冻结签名与类型形状，`todo!()` 是唯一函数体。
- `ProjectionKey.lineage` / `PolicyRule` 的语义判定（rule → 投影的完整映射）归后续票，本票只建形状。
- 未做任何 git 提交（ticket 分支的提交由编排层决定，本票在 detached worktree 内只产文件 + 报告）。

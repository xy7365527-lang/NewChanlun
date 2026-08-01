## 关票声明（本票是第一张按镜像同步新条款关的票）

**镜像**：`origin/main-rewritten @ cc7d00b309`（== 关票面，两次纯快进无 force）
**CI run**：[30599426429](https://github.com/xy7365527-lang/NewChanlun/actions/runs/30599426429) → **红**，逐 job 如下

| job | 结论 | 说明 |
|---|---|---|
| `fixture-drift` | ✅ | |
| `rust-check` → `cargo check`（两条）/ `cargo fmt --check` | ✅ | fmt 由 `ed793f8887` 机械复位后转绿 |
| `rust-check` → **`cargo test — default targets`** | ❌ | `17 passed; 2 failed` —— **[#811](https://github.com/xy7365527-lang/NewChanlun/issues/811) 那两条，无第三条** |
| `test`（pytest） | ❌ | `1 failed, 5004 passed` —— [#818](https://github.com/xy7365527-lang/NewChanlun/issues/818) |

**红的去向照实写**（按新条款要求）：

- `cargo test` 的两条 = [#811](https://github.com/xy7365527-lang/NewChanlun/issues/811)，**教义分歧非实现 bug**，归 [买卖点的教义正本 #816](https://github.com/xy7365527-lang/NewChanlun/issues/816) 裁定；裁定前**不许修绿、不许 `#[ignore]`、不许踢出命令**。这是**有意为之的红**。
- pytest 的一条 = [#818](https://github.com/xy7365527-lang/NewChanlun/issues/818)，待 triage，与教义无关，可并行做。

## 交付清单

| 项 | 落点 |
|---|---|
| ① 分支归属解决 | 镜像 `61ed615393 → cc7d00b309`，三次快进，无 force。**成因定案**：远端 `main` 是另一条祖传历史（与本地 `main` **零共同祖先**、独有 1171 提交），本地重写线以 `main-rewritten` 发布 ⟹ 「`main` 不推 origin」这条现行安排**有真实理由**，不是遗留。 |
| ② `cargo test` 接入并真执行 | `.github/workflows/ci.yml` `rust-check` 末尾两步（default + `backtest_bin`），沿用现有缓存；既有 `#[ignore]` 未动。commit `60ff450ea7` |
| ③ 失败处置（本票纪律：不许删/不许 ignore/不许缩范围） | fmt 漂移 6 文件 11 处 → 机械复位 `ed793f8887`；parity 两条 → [#811](https://github.com/xy7365527-lang/NewChanlun/issues/811)；pytest 一条 → [#818](https://github.com/xy7365527-lang/NewChanlun/issues/818)。**零删除、零新增 `#[ignore]`、零范围缩减。** |
| ④ 慢测试处置 | 不分层、不改成仅 main 触发——两步 `cargo test` 合计 <2min，与既有两步 `cargo check` 同预算。依据写在 `ci.yml` 注释里。 |
| ⑤ 镜像再落后的防线（本票新增，非原票面） | 纪律条款 `docs/agents/delivery-discipline.md` 新增一节 + 机械提醒 `scripts/check_mirror_sync.sh`（挂 `.git/hooks/post-commit`，只提醒不阻断）。commit `cc7d00b309` |

## 本票的净发现（非票面预期）

**「锁形同虚设」的第三种形态**：**执行了，但看的不是这棵树。** 前两种已在案（[#806](https://github.com/xy7365527-lang/NewChanlun/issues/806) 没接进 gate / [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定十一 只编译不执行），本票补第三种，已入 `delivery-discipline.md` 成表。

**最能说明问题的标本**：[#611](https://github.com/xy7365527-lang/NewChanlun/issues/611) 立的 `cargo fmt --check` 防回潮闸**一直是绿的**——因为它一直在 545 提交前的旧树上把关。**闸装了、也响了，只是响在别处。** 镜像追上后当场红，6 文件 11 处漂移。

⟹ 对 [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定十一附带原则的**修订**：不只是「验收判据必须真的被 CI 执行才算数」，还得是「**在当前代码上**执行」。[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 对拍锁的落地前提同步升级。

## 未做（090 照实）

- **原提议「CI 加一步检查镜像落后即红」经复核不可行并作废**——CI 只看得到被推上去的树，看不到本地工作线领先多少。已写进 `delivery-discipline.md` 防止再提。
- 派出的子代理最初报「2169 passed / 0 failed」，**该数字量自落后 545 提交的旧树**，主控独立复跑才发现。已在本票评论区留全过程。

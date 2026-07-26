# #327 实施报告：真覆盖见证锁——ZD==ZG 判据命中坐实 + 事故如实登记（docs-only 收口）

- 日期：2026-07-26
- 票：#327（parent #323 影子评审浮出「零翻动是零覆盖的结果，不构成安全证据」、#321 裁定）
- 落地 commit（此前，未随附报告）：`d9d551d788`「test(center): #327 真覆盖见证锁——ZD==ZG 判据命中坐实
  （全量 25 / 变动块 2）+ #323 级联重排 26/18/8/3/2 独立复现」
- 上游文书：`.chanlun/review-results/center-strict-single-point-impl-20260726.md`（#323 报告 §0/§6，立票
  #327）；`docs/canonical-coverage-rust-impl.md` B1 GOLDEN 登记（本票追加披露，见 §3）
- 本票（docs-only 收口）改动：本报告新建 + `docs/canonical-coverage-rust-impl.md` B1 补三处披露注
  （§3）+ `rust/src/theta_v0/backtest/wverify_run.rs` 三处小硬化（§4，零谓词/断言逻辑回退）

---

## 0. 一句话结论

`d9d551d788` 落地的两把真覆盖见证锁（全量对账臂 25 命中 / 变动块臂 2 命中，逐点见证 + 级联重排对账
均成立）当时未随附独立报告文件——本票补齐该报告。同时如实登记锁**落地当时**（2026-07-26 12:35-12:37）
与并发 #337 工位共享 git 索引导致的混批 + 连带 reset 事故：**本工位（#327）对该 reset 命令本身负有
直接责任**，与 `.chanlun/review-results/center-tolerant-arena-window-20260726.md` §8（#337 侧记录，
仅记载了事故发生与 reset 造成的后果）互为对方缺失的另一半记录（§5）。

---

## 1. #327 交付摘要（承前 `d9d551d788`，未改动）

| 锁 | 数据窗 | 真覆盖（`zd==zg` 命中） | 级联重排（对称差） |
|---|---|---|---|
| `center_strict_zd_eq_zg_change_block_witness`（本票主交付） | 2022-01-01…2022-02-28（#323 点名变动块） | L0 **2**（均在 2022-02-01，单点核心 `3_830_000_000_000`） | L0 6 / L1 2 / L2 0 / L3 0 |
| `center_strict_zd_eq_zg_full_btc_cascade_witness`（全量对账臂） | BTC 全量 4 613 599 bar | L0 **23** / L1 **2**（合计 25） | L0 26 / L1 18 / L2 8 / L3 3 / L4 2 |

两把锁均 `#[ignore]`（需 BTC 数据）、均跑双臂（生产严格判据 + `#321` 前旧弱口径本地副本），逐命中点
断言「严格臂 `None` ∧ 弱臂 `Some` ∧ 核心恰为该单点」；`assert_cascade_faithful` 机检双臂级联复刻
`classify_impl` 的忠实性。级联重排对账 `26/18/8/3/2` 与 #323 报告 §2.1 转录值逐级相等，坐实该表原
「未独立重跑复现」的缺口。**本票（收口）未改动上述任何谓词/断言逻辑**，只补披露与小硬化（§3/§4）。

---

## 2. 复算索引

```bash
cd /Users/silencehan/Projects/NewChanlun/rust

# 两把 BTC 见证锁（需 --ignored + BTC 数据缓存）
cargo test --release --lib theta_v0::backtest::wverify_run::center_strict_zd_eq_zg -- --ignored --nocapture

# 全量 lib 测试
cargo test --lib
```

- 两把锁本票复跑（含 §4 三处小硬化后）：**2 passed / 0 failed**，读数逐位不变
  （L0/L1/... 各级 strict/weak/lcs/级联重排/命中数与 `d9d551d788` 落地时一致）。
- `[#327/...]` 前缀输出行 sha256（连续两次 `--nocapture` 逐位一致，硬化前后同一哈希，证明本票
  三处小硬化零副作用）：`f5b8b989e8a7b54f1210fac5c5c5c1d1e239251d706e77ca52c722cc7daeef68`
- `cargo test --lib`（本票改动后复跑）：**1924 passed / 0 failed / 141 ignored**（HEAD 已被并发
  #337 工位推进，passed 基数高于 `d9d551d788` 落地时的 1913，属预期；本票不改变任何测试的
  通过/失败判定，仅两把既有 `#[ignore]` 锁的断言内部增强，0 failed 即验证达标）。

---

## 3. B1 GOLDEN 登记追加披露（`docs/canonical-coverage-rust-impl.md`，本票新增）

在既有 `#327 真覆盖见证锁` 登记段后追加三条披露：

1. **覆盖口径边界**：登记表「真覆盖」命中数是**全三元组枚举口径**（`cascade_dual` 对本级全部连续
   三元组求 `zd==zg`），生产扫描游标实际考察为其**子集**（`detect_centers_windowed_resume` 路径依赖
   跳跃）；「生产真被打到」由**级联重排数 ≠0** 独立坐实，二者是交叉证据，不是同一件事。
2. **两窗读数不可互推**：全量臂与变动块臂是两次独立 `parse_layer` 解析（变动块窗内数据经
   `slice_date_window` 后无窗外历史），中枢序列与级联重排计数各自独立，不可互推。
3. **复算触发命令 + 输出摘要 sha256**：见 §2（命令 + 哈希登记进 B1 段）。

---

## 4. 三处小硬化（`rust/src/theta_v0/backtest/wverify_run.rs`，零生产代码改动、零断言语义回退）

| # | 位置 | 改动 |
|---|---|---|
| 1 | `c327_run_witness`（:2245-2253） | 逐级循环内新增 `if !sh.is_empty() { assert!(reading.rearranged() > 0, ...) }`——命中不为空时硬性要求该级级联重排数 `>0`，把 B1 披露①「生产真被打到由级联重排≠0 独立坐实」从文字披露落成机检；两把锁公用此函数，硬化同时覆盖两处 |
| 2 | `C327_BLOCK_HIT_CORE`（:2340-2341）+ `center_strict_zd_eq_zg_change_block_witness`（:2393-2397） | 原变动块两处命中的单点核心 `3_830_000_000_000` 只在 `eprintln!` 里动态打印、未断言；新增 `const C327_BLOCK_HIT_CORE: Tick = 3_830_000_000_000` 并在逐点循环补 `assert_eq!(h.core, C327_BLOCK_HIT_CORE, ...)`——核心价位漂移会被机检拦下，不再只靠人读日志 |
| 3 | `C327Hit::merged_start` → `src_start`（结构体定义 :1943 + 构造 :2062 + 全部读取点） | 该字段实际存的是 `Segment.start_index`（原始 bar 序下标，`c327_run_witness` 内部注释 :2197 已如实注明「非 merged 序」），旧名 `merged_start` 与实际语义相反，重命名消除误导；纯改名，零行为改动 |

三处均已本地复跑验证（§2），读数（含 sha256）与硬化前逐位一致，硬化只新增机检、不改变任何既有断言
的判定结果。

---

## 5. 事故如实登记：12:35-12:37 共享 git 索引混批 + 连带 reset（本工位责任，与 #337 侧互证）

`.chanlun/review-results/center-tolerant-arena-window-20260726.md` §8 已从 #337 工位视角记录了这次
事故（「并发工位混批」「并发工位执行了 git reset」），但未点名该「并发工位」即本票（#327）工位、也
未附本工位自己的操作意图。本节据本仓库 `git reflog show` 的原始记录（时间戳精确到秒，非事后转录）
补齐本工位的一侧账目：

```
fa916fe70e HEAD@{2026-07-26 12:35:28 -0400}: commit: fix(classifier): #337 容读法落地——…
af17c4d484 HEAD@{2026-07-26 12:36:41 -0400}: #327 真覆盖见证锁
36092eab8e HEAD@{2026-07-26 12:37:10 -0400}: 撤回 #327 空提交（tree 未变，重做）
d9d551d788 HEAD@{2026-07-26 12:37:16 -0400}: #327 真覆盖见证锁
e971040794 HEAD@{2026-07-26 12:38:07 -0400}: commit: fix(classifier): #337 补入本票 wverify_run 口径改动…
0e7dc46dd1 HEAD@{2026-07-26 12:39:07 -0400}: commit: fix(classifier): #337 容读法落地——…
```

时间线与本工位责任：

1. **12:35:28**：并发 #337 工位在**同一工作目录、同一 git 索引**（非 worktree 隔离）上执行
   `git commit`（无 pathspec），产出 `fa916fe70e`。此时本工位（#327）自己的两个文件改动
   （`wverify_run.rs` 见证锁块 + `docs/canonical-coverage-rust-impl.md` B1 登记）已 `git add` 进了
   这个**共享**索引——`fa916fe70e` 的 diff 因此把本工位尚未提交的改动一并吞了进去（`+547` 行 +
   `+19` 行），本工位对此**不知情**（两工位互不感知对方在同一索引上的暂存状态）。
2. **12:36:41**：本工位随后执行 `git commit`（本工位视角：提交自己的见证锁块），产出 `af17c4d484`。
   因内容已在 `fa916fe70e` 里落地，这是一个**空 diff commit**。
3. **12:37:10**（★本工位直接责任所在）：本工位为清理自己以为的「无害空提交」`af17c4d484`，执行了
   `git reset 36092eab8e`（reflog 消息「撤回 #327 空提交（tree 未变，重做）」——**该消息由本工位
   写下**，语义是"我在撤销我自己的空提交"）。但 `36092eab8e` 是 `fa916fe70e` **之前**的提交，不是
   `af17c4d484` 之前的提交——这次 reset 因此**连带把并发 #337 工位的 `fa916fe70e` 也从分支上丢掉
   了**。本工位在下达这条 reset 命令时，**未查证**在自己 12:36:41 提交与 12:37:10 reset 之间，
   分支尖端是否已被另一工位推进过（`git log -1` 或 reflog 复查缺失）——这正是本工位的责任所在：
   reset 前只认自己上一步的 HEAD，没有复核共享分支的当前实际状态。
   （工作区改动本身未丢——`fa916fe70e`/`e971040794`/`0e7dc46dd1` 对应的源码改动仍在 #337 工位的
   工作目录里，随后由该工位以两个 pathspec 限定的 commit 重新入库，见 reflog 12:38:07/12:39:07；
   本节只对本工位这次 reset 动作本身如实登记，不代为核算 #337 侧损失，那属于 §8 已有记录。）
4. **12:37:16**：本工位改用 `git commit -- rust/src/theta_v0/backtest/wverify_run.rs
   docs/canonical-coverage-rust-impl.md`（pathspec 限定，绕开共享索引）重新提交，产出 `d9d551d788`
   ——即本票承前的落地 commit。此后本工位未再触碰共享索引/HEAD。

**与 #337 侧记录（`center-tolerant-arena-window-20260726.md` §8）互证结论**：两侧记录对同一时间窗
（12:35-12:37）、同一因果链（共享索引混批 → 空提交 → reset 连带丢弃）描述**一致**，无冲突；本节
补齐的唯一增量是**责任归属**——#337 侧记录写的是「并发工位执行了 reset」，本节确认「并发工位」即
本工位，且本工位在下 reset 命令前**未做**分支尖端复核，这一操作缺口是本次事故的直接触发点。

教训（与 `feedback_commit_check_staged_area` 同型，本工位侧独立复发确认）：共享工作目录/共享 git
索引下，`reset` 前必须先 `git log -1` / `git reflog` 复核分支当前尖端是否已被并发工位推进过，不能
只凭自己上一步的操作记忆判断"这个提交是我刚提的空提交、撤了无妨"。稳妥做法是全程 `git commit --
<pathspec>` 绕开共享索引（本工位第二次提交即改用此法，未再出事）；更彻底的做法是并发工位从一开始
就应使用独立 git worktree，而非共享同一工作目录。

---

## 6. 纪律

- 只改本票范围内文件：本报告新建 + `docs/canonical-coverage-rust-impl.md`（B1 追加）+
  `rust/src/theta_v0/backtest/wverify_run.rs`（§4 三处小硬化）；既有脏文件（`.claude/rules/*`、
  `CLAUDE.md`、`AGENTS.md`、`rust/src/bin/p107_level_calib.rs`、`tmp/`、`.agents/`、
  `.chanlun/agent-roster-*` 等）未动未 add。
- 未 amend 已有 commit；未回关 issue；未发任何 GitHub 评论。
- §5 的事故记录基于本仓库 `git reflog show` 的原始时间戳与消息文本直接转录，未做任何历史改写
  （`fa916fe70e`/`af17c4d484` 均已不在当前分支可达范围内，本节仅陈述曾发生的操作序列，不恢复、
  不删除任何悬挂对象）。

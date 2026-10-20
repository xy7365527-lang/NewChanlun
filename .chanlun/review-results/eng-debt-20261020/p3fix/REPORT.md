# #1588 P3 修复分片（p3fix）证据报告

- 分支：`eng-debt-1588/p3fix`，基于 origin/main `a0522447ca`
- 范围：A1、A3、A6 三项修复，各占一个提交；另开 A5、Q1、Q2 三张票（#1608 / #1609 / #1610）。第二轮按非作者评审（`reviews/1611-claude-fable-5.md`，PASS-WITH-NITS）补了发现 1–3，见文末「第二轮：评审发现处置」。
- A4（`content_producer.rs:3748` 过时注释）不在本分片：主控已转 #1488，由其切片 0061 处理。
- 验证约束：本机（macOS）不跑 cargo。Rust 验证用 `nc-remote` 在 WSL 上执行（ref 为本地已提交分支，日志在本机 `/tmp/nc-remote/`，不入仓），另以 PR CI 为准。
- 评审：本报告是作者的自述证据，**不能替代非作者独审**。独审由主控安排。

| 项 | 提交 | 文件 |
|---|---|---|
| A1 | `2a4f405082` | `.sandcastle/codex-child.ts`、`.sandcastle/codex-child.test.ts`、`.sandcastle/CODEX-CHILD.md` |
| A3 | `6958eefcb8` | `.chanlun/definitions/bi.md` |
| A6 | `1b27e492b0` | `rust/src/theta_v0/classifier/signal.rs` |
| 评审发现 1（A1 同类） | `7a01ee589e` | `.sandcastle/git-environment.ts`（新）、`.sandcastle/delivery-bundle.ts`、`.sandcastle/delivery-bundle.test.ts`、`.sandcastle/codex-child.ts`、`.sandcastle/codex-child.test.ts` |
| 评审发现 2（A6 单测） | `55468c8079` | `rust/src/theta_v0/classifier/signal.rs`（tests 模块末尾） |

---

## A1：预检 git 子进程继承完整 `process.env`，CLIProxy 密钥随之外泄

### 现象

`.sandcastle/codex-child.ts` 的 `git()`（原 `:124-131`）调用 `spawnSync("git", …, { env: { ...process.env, GIT_OPTIONAL_LOCKS: "0" } })`。`preflightCheckout()` 要连续调用 5 次以上 git，每次都把父进程的全部环境变量交给 git，其中包括 `CLIPROXY_LOCAL_KEY` 和任何 `GH_TOKEN`、`OPENAI_API_KEY` 之类凭据。git 会再把这些变量传给 hook、credential helper、fsmonitor 等子进程。这与 `CODEX-CHILD.md` 中「密钥只进 Codex 子进程」的声明不符。PR #1480 的 Codex P1 评论 r4260955949 已经指出过这个问题，但一直没有处置。

### 复现命令与输出

新增集成测试的做法：在 `PATH` 最前面插入一个用 node 写的 `git` 包装器。它只记录每次调用时的**环境变量名**（JSONL 格式，不记录值），然后转交真实 git。测试临时往 `process.env` 写入夹具值（`fixture-not-a-real-key` 一类，**不是真实密钥**），调用 `preflightCheckout`，再断言每次调用的变量名里没有这三个密钥名。

```bash
node --import tsx --test .sandcastle/codex-child.test.ts .sandcastle/delivery-bundle.test.ts
```

- 修复前（`git()` 仍是 `{ ...process.env, … }`）：新增的集成测试失败，断言信息为「git 环境含 CLIPROXY_LOCAL_KEY」。
- 修复后：

  ```
  # tests 71
  # pass 71
  # fail 0
  ```

  71 = 两文件原有 69 条 + 新增 2 条（codex-child 单文件由 43 条增至 45 条）。
- typecheck：`npx tsc -p tsconfig.sandcastle.json`，修复前后均 exit 0。

注：CI 不跑这两个 node:test 文件，也不跑 sandcastle 的 tsc。以上结果均在本机执行（不属于 cargo 类重型验证）。本工位没有 `node_modules`，测试时临时软链到 `/tmp` 下用 `npm ci` 装好的依赖，提交前已删除软链。

### 根因

`git()` 为了加 `GIT_OPTIONAL_LOCKS=0`，顺手把整个 `process.env` 展开进去，没有区分「git 定位与本地化所需的变量」和「只该给 Codex 子进程的凭据」。

### 处置

- 新增 `export function gitEnvironment(source = process.env)`。它只透传白名单变量：`PATH`、`HOME`、`USER`、`LOGNAME`、`TMPDIR`、`LANG`、`LC_ALL`、`LC_CTYPE`、`LC_MESSAGES`、`XDG_CONFIG_HOME`，并固定设置 `GIT_OPTIONAL_LOCKS=0`。`git()` 改为 `env: gitEnvironment()`。
- **设计取舍（请评审重点看）**：宿主的 `GIT_*` 变量（`GIT_DIR`、`GIT_INDEX_FILE`、`GIT_WORK_TREE` 等）**不透传**。理由是预检已经用 `-C <checkout>` 指定了仓库，透传 `GIT_DIR`/`GIT_INDEX_FILE` 反而可能把预检引到别的仓库或索引上。代价是宿主若靠 `GIT_SSH_COMMAND`、`GIT_CONFIG_*` 等变量改 git 行为，这些设置在预检里不再生效。预检只做本地读操作（rev-parse/status 等），不访问远端，因此判断影响可以接受。任务书举例时把 `GIT_*` 列入白名单，这里与之不一致，需要评审裁定。
- 新增两条测试（追加在文件末尾，避开 PR #1593 在同文件中改动的注释行）：
  - `git 环境只留白名单：不含 CLIProxy 密钥、API/GitHub 密钥与宿主 GIT_*（#1588 A1）`：纯函数 `deepEqual` 断言。
  - `预检 git 子进程拿不到 CLIProxy 密钥及其他凭据变量（#1588 A1）`：上文所述的包装器集成测试。断言每次调用都含 `GIT_OPTIONAL_LOCKS`，且调用次数 ≥ 5，确保包装器确实拦到了预检的调用；结束时在 `finally` 中恢复 `process.env` 和 `PATH`。
- `CODEX-CHILD.md:27` 补了一句说明：预检用的 git 只拿白名单变量，不含该密钥。

---

## A3：bi.md 指向一个从未存在过的机读清单

### 现象

`.chanlun/definitions/bi.md:338` 写着「机读 `AFFECTED-CODE.json`」，但仓内找不到这个文件。

### 复现命令与输出

```bash
git log --all --diff-filter=A --format=%h -- '*AFFECTED-CODE.json'   # 无输出：任何 ref 上都未曾新增过
git ls-files | grep AFFECTED-CODE
# .chanlun/review-results/issue1323-grilling-20261013/1343-AFFECTED-CODE.md
```

### 根因

写指针时，实际落地的是 `.md` 工作草稿，没有生成计划中的 `.json`，指针也没有跟着改。

### 处置

指针改为「工作草稿副本 `.chanlun/review-results/issue1323-grilling-20261013/1343-AFFECTED-CODE.md`」。**纯指针订正，不改教义**：只改这一处路径和对它的称呼，前后文的定义文字一字未动。

---

## A6：`type1_funnel_dx` 的 broke 判据误写一臂（unreachable pattern）

### 现象

`rust/src/theta_v0/classifier/signal.rs:1588` 所在的 `match` 中，第二臂写成 `(Some(Direction::Down), Direction::Down) => c.zg < end.price`，模式与第一臂 `(Some(Direction::Down), Direction::Down) => end.price < c.zd` 完全重复，编译器报 `unreachable pattern`。后果是「上涨趋势中以 `zg < end.price` 判 broke」这一臂永远不会被求值：上涨趋势一律落进兜底臂 `_ => false`，永远判为未破。

### 复现命令与输出

在 WSL 上用 `nc-remote` 执行（`NC_TARGET=eng1588`）。修前基线 ref 是临时本地分支 `eng-debt-1588/p3fix-prefixA6`，指向 `6958eefcb8`，即 A6 之前的提交。`nc-remote` 不接受 `~1` 这类后缀 ref，所以只能建临时分支，用完已删除，从未推送。

1. **cargo check 告警**

   ```bash
   nc-remote <ref> rust cargo check --lib --tests --message-format=short
   ```

   | ref | `(lib test)` 告警数 | `unreachable pattern` 行数 |
   |---|---|---|
   | 修前 `6958eefcb8` | 53 | 1（`src/theta_v0/classifier/signal.rs:1588:13: warning: unreachable pattern`） |
   | 修后 `1b27e492b0` | 52 | 0 |

   `(lib)` 两边都是 48 条。这说明 `type1_funnel_dx` 只在测试构建中编译，所以只有 `(lib test)` 少了这一条。

2. **classifier 模块测试**（修后 `1b27e492b0`）

   ```bash
   nc-remote eng-debt-1588/p3fix rust cargo test --release --lib theta_v0::classifier
   # test result: ok. 919 passed; 0 failed; 10 ignored; 0 measured; 2051 filtered out
   ```

   日志（本机，不入仓）：作者运行 `/tmp/nc-remote/20261020-184250-1b27e492b0.log`（同一次运行还跑了下面第 3 项的全历史普查；命令头含中文被转义，按文件名检索即可）；评审复跑 `/tmp/nc-remote/20261020-213958-4de612c241.log`，结果相同（919 passed）。第二轮加单测后为 920 passed，见文末。

3. **ignored 普查 `type1_funnel_census_btc`**（数据：WSL `$HOME/nc-remote/data/btc_1m_full.json`，运行前软链到 `<worktree>/analysis/data_cache/`）

   ```bash
   nc-remote <ref> rust cargo test --release --lib -- --ignored --nocapture --exact \
     theta_v0::classifier::tests::pipeline_geometry::type1_funnel_census_btc
   ```

   - **全历史（不设窗口）：修后 FAIL。失败发生在解析层（L0 为空），早于 A6 所改的漏斗判据，与 A6 无关；修前没有单独跑全历史。**

     ```
     [funnel] BTC bars=4613599
     [funnel] L0 segments=0 merged_bars=1
     panicked at src/theta_v0/classifier/tests/pipeline_geometry.rs:679:5   （"空 L0 无漏斗对象"）
     test result: FAILED. 0 passed; 1 failed
     ```

     原因：`IncrFactsState::append`（`rust/src/theta_v0/parser/inclusion.rs:1025-1027`）规定，方向尚未建立时，若第 2 根 bar 与首根包含，整条流就置为「初始方向永久未定」（#1392 语义，工程实例票 #1393 TB-02-U 仍开），于是只剩 1 根合并 bar、0 段。这是 main 上已有的前置失败。

   - **分窗口（`CENSUS_WINDOW=起,止`）**：

     | 窗口 | ref | L0 段数 | 结果 | 各级「环2过局部趋势门」 |
     |---|---|---|---|---|
     | 2020-10-01..2021-04-01 | 修后 `1b27e492b0` | — | ok | L0/L1/L2 均为 0 |
     | 2020-10-01..2021-04-01 | 修前 `6958eefcb8` | — | ok，输出与修后逐行相同 | 均为 0 |
     | 2017-09-01..2030-01-01 | 修后 | 633（merged 3,031,865） | ok | L0–L4 均为 0 |
     | 2019-01-01..2030-01-01 | 修后 | 287 | ok | 均为 0 |
     | 2020-06-01..2022-06-01 | 修后 | 501 | ok | 均为 0 |
     | 2023-01-01..2030-01-01 | 修后 | 0（merged 1） | FAIL（同上，首两根包含） | — |

   **结论（照实）**：现有真实数据的普查**无法区分修前与修后**。所有能跑通的窗口里，「环2过局部趋势门」恒为 0，所以 broke 判据（环3）根本没有被求值，`buy1 = sell1 = 0`。A6 的正确性依据只有三条：① 与生产判据静态比对一致；② 告警消失；③ 模块测试全绿。普查本身给不出行为证据。

### 根因

`1592bf05ca`（#988 MACD 同色化）改动这段代码时，把第二臂的方向写错了。那个提交的主题与 broke 判据无关，属于误伤。生产路径 `rust/src/theta_v0/classifier/signal/gates.rs:67` 的同一判据写法是 `(Some(Direction::Up), Direction::Up) if last_center.zg < end.price`。

### 处置

第二臂改为 `(Some(Direction::Up), Direction::Up) => c.zg < end.price,`，与生产判据 `gates.rs:67` 一致。不删测试、不加 `#[ignore]`、不缩小测试范围。

### 遗留（建议转票，未在本 PR 处理）

- 全历史普查跑不起来：首两根 bar 包含 → 初始方向永久未定 → L0 为空。普查的数据入口应选一个确定的起点，或者等 #1393 的处置。是否另开票由主控裁定，可挂在 #1393 下。
- 各窗口「环2过局部趋势门」恒为 0：这个探针在真实数据上没有覆盖到环3及以后的环节。这是探针设计问题还是门本身过严，需要另查。

---

## 开票（均为 `needs-triage`，开票前已用 `gh issue list --state all --search` 查重）

| 收集项 | 票 | 查重结论 |
|---|---|---|
| A5 编译告警回潮 | #1608 | #913 已关，#1419 只在评论中提过要另行跟踪，没有专票 → 新开 |
| Q1 stop-s/stop-q 诊断 | #1609 | 搜「尚未退出」「stop-q」均无对应票 → 新开 |
| Q2 ADR 0040 三条 Codex 评论未答 | #1610 | 「ADR 0040」命中 #1590、#1598–#1606 等，均不覆盖这三点 → 新开 |

A5 票正文核对后有两处补正：`fetch_update` 实为 9 处（P3 收集表漏记 `audit_owner.rs:131`）；7 处 must_use 中，6 处是 `progress_handler`，1 处是 `prepared.rs:1710` 的 `update_hook`。

## 与其它在途工作的关系

- PR #1593 也改了 `.sandcastle/codex-child.ts`，只改注释行尾。本 PR 没有碰那些行，新测试追加在文件末尾。若合入顺序造成文本冲突，应当是平凡冲突，由主控协调，本分支不强推覆盖。

---

## 第二轮：评审发现处置（主控第二轮指示）

评审：`/Volumes/AgentStorage/eng-debt-20261020/reviews/1611-claude-fable-5.md`，结论 PASS-WITH-NITS；`GIT_*` 不透传的取舍评审同意，保持。

### 发现 1：`delivery-bundle.ts` 的 git 子进程同样继承完整 `process.env`（`7a01ee589e`）

- **现象**：`.sandcastle/delivery-bundle.ts:126`（`preflightDelivery` 内的 `git()`）仍为 `env: { ...process.env, GIT_OPTIONAL_LOCKS: "0" }`。管理者进程环境里的 `CLIPROXY_LOCAL_KEY` 等凭据会进入 git 及其 hook、credential helper、fsmonitor，与 A1 同类。
- **是否需要额外变量**：逐条读了该处全部 git 调用：`rev-parse`、`merge-base --is-ancestor`、`status`、`ls-files -v`、`ls-tree`、`show`、`diff`。全部是 `-C repo` 定位的本地只读操作，没有 push/fetch，不需要任何远端凭据，因此**不追加任何变量**。
- **处置**：把 `gitEnvironment()` 从 `codex-child.ts` 移到共享模块 `.sandcastle/git-environment.ts`，`codex-child.ts` 和 `delivery-bundle.ts` 都从这里导入，函数体不变。注释补了一句：需要访问远端的调用方不得复用此函数。
- **测试**：`delivery-bundle.test.ts` 末尾新增「交付预检的 git 子进程拿不到 CLIProxy 密钥及其他凭据变量（#1588）」。做法与 A1 集成测试相同：在 PATH 最前面放一个包装器，记录每次 git 调用的变量名；只用夹具值；`t.after` 恢复环境变量。测试断言预检结果为 `READY_FOR_APPROVAL`，git 调用 ≥ 5 次，每次都不含三个密钥名，且都含 `GIT_OPTIONAL_LOCKS`。
- **证据**（本机）：
  - `node --import tsx --test .sandcastle/codex-child.test.ts .sandcastle/delivery-bundle.test.ts` → `tests 72 / pass 72 / fail 0`（69 + A1 的 2 条 + 本条 1 条）。
  - `tsc -p tsconfig.sandcastle.json` → exit 0。
  - **修前失败**：把 `.sandcastle/` 拷到 `/tmp/p3fix/prefix/`，用 `git show 4de612c241:.sandcastle/delivery-bundle.ts` 覆盖成修前版本，跑 `--test-name-pattern="1588"`，结果 `not ok 1 … error: 'git 环境含 CLIPROXY_LOCAL_KEY'`。

### 发现 2：A6 补合成夹具单测（`55468c8079`）

- **测试**：`signal.rs` tests 模块末尾新增 `type1_funnel_dx_up_trend_broke_arm_reachable`。
  - 中枢与段沿用 `first_sell_mirror_with_trend_divergence`：上涨趋势，leave 段 420、top 段 430 都高于 C1.zg=400，retest 段方向锚为 Down。
  - closes 取「A 段缓、C 段急」，不构成背驰。
  - 断言：`s_gate_open == 3`、`s_broke == 2`、`s_mapped == 2`、`s_diverge == 0`。retest 段落入 `_ => false`，不计入 broke。
  - 函数尾部的 parity 断言（`s_mapped` = 生产 struct_break 候选数）同时把关。
- **为什么选「无背驰」夹具**：第一版直接复用 1 卖夹具（有背驰）。修后运行时，函数尾部的 parity 断言 `prod_t1 == s_diverge` 失败（左 1 = 生产 sell1，右 2 = 漏斗 s_diverge），日志 `/tmp/nc-remote/20261020-221105-a4ae31418a.log`。原因是漏斗环 7 逐段计背驰，而生产经 #607/#1249 的 D2 大闸后，只给 c 窗内命中 leave+retest 的那一段置 bit，leave 段本身为 `Missing`，只记 struct_break，不置 sell1。这是探针 parity 的既有缺口，与 A6 无关：真实数据从未走到这里，因为环 2 恒为 0。所以改用无背驰夹具，让 parity 的环 6 等式作钉子。该缺口列入下方遗留。
- **放在模块末尾的原因**：`bsp.rs` 的 `no_stray_bsp_binding_rewrites_outside_registry` 按 `signal.rs` 的行号登记白名单（`:733-745`）。插在中间会使后续行号整体下移，触发该守卫失败（`/tmp/nc-remote/20261020-221755-d9bf03d6a8.log`：919 passed 1 failed）。移到末尾后不影响任何登记行号。
- **修前失败**：建临时本地分支 `eng-debt-1588/p3fix-a6revert`（`4f510470bc`）。它在 `55468c8079` 之上把第二臂撤回为 `(Some(Direction::Down), Direction::Down) => c.zg < end.price`。用完已删除，从未推送。

  ```bash
  NC_TARGET=eng1588 nc-remote eng-debt-1588/p3fix-a6revert rust \
    cargo test --release --lib theta_v0::classifier::signal::tests::type1_funnel_dx_up_trend_broke_arm_reachable
  # warning: unreachable pattern --> src/theta_v0/classifier/signal.rs:1588:13
  # panicked at src/theta_v0/classifier/signal.rs:1654:5:
  # assertion `left == right` failed: 漏斗环6（mapped 候选）须=生产 struct_break 候选数
  #   left: 2
  #  right: 0
  # test result: FAILED. 0 passed; 1 failed
  ```

  日志：`/tmp/nc-remote/20261020-221939-4f510470bc.log`。
- **修后**（`55468c8079`）：

  ```bash
  NC_TARGET=eng1588 nc-remote eng-debt-1588/p3fix rust cargo test --release --lib theta_v0::classifier
  # (lib test) generated 52 warnings，无 unreachable pattern
  # test result: ok. 920 passed; 0 failed; 10 ignored; 0 measured; 2051 filtered out
  ```

  日志：`/tmp/nc-remote/20261020-222120-55468c8079.log`。

### 发现 3：919 passed 的日志出处

已在上文 A6「复现命令与输出」第 2 项补上：作者日志 `20261020-184250-1b27e492b0.log`，评审复跑日志 `20261020-213958-4de612c241.log`。

### 发现 4（nit）：集成测试不自验恢复

未改。评审读码确认恢复正确，并注明可忽略。新增的 delivery-bundle 测试改用 `t.after` 恢复，不再依赖测试顺序。

### 第二轮新增遗留（建议转票）

- `type1_funnel_dx` 尾部 parity `prod_t1 == s_diverge` 没有建模 #607/#1249 的 D2 大闸（T3-in-c `Missing` 时不置 bit）。在同一 c 窗内有多个 broke 且背驰的段时，漏斗会多计。用 `first_sell_mirror_with_trend_divergence` 的夹具即可复现：漏斗 2，生产 1。该探针只被 ignored 普查消费，目前真实数据走不到这一步。建议挂在 A5 票 #1608 或普查遗留票下处理。

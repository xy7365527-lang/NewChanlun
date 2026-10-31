# ADR 0059：生产会话布局合同（S admission 测量域与并发写域库目录）

> 正本：本文件是 #1818 `production_session` 布局合同的正本，涵盖 S admission 测量域 M 的边界、并发写域库目录的启动前拒绝规则、路径比较语义与 owner 落点。
> 最后裁定：#1818（2026-11-01，T 类，依 #1323 Notes N-1 预授权判定）
> 受影响代码清单：`rust/src/bin/production_session.rs:125`（`load_manifest`，拒绝检查落点）；`rust/tests/production_session_entry.rs`（验收测试）；`docs/runbook_production_session.md:35`、`:40`、`:53`、`:59`（runbook 订正）。实施归 #1781。

**状态**：T 类裁定，已通过独立审查。本 ADR 只定规则与判据；实现与运行证据归 #1781，不以本 ADR 声称实现完成。

## 问题

S 写者打开库之前，要严格核验 admission 测量域 M（下文定义）中每个成员的完整 stamp，并核对成员集合本身（`rust/src/bin/s_session_v3/admission_provider.rs:299-345`；stamp 比对在 `Pinned::verify`，`:74-88`，严格模式不符即 `SourceIdentityChanged`，集合不符即 `SourceMembershipChanged`，`:342`）。打开以后，主库和目录只核身份（`:348-360`）。launcher 拉起 S 后不等待，紧接着拉起 E、各 B 和 X（`rust/src/bin/production_session.rs:455`、`:467`、`:490`）。这些域各自持有 SQLite 库和 `<db>.writer.lock`（`rust/src/bin/x_execution_session.rs:3`，ADR 0039:300）。它们在服务阶段会在库所在目录里建删侧件。只要某个并发写域的库目录落进 M，S 就会在打开窗口内间歇拒启。PR #1773 独审 M-2 记录了远端复现：5 轮中 3 轮失败（`/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1773-INDEPENDENT-REVIEW-6C6E.md:55`、`:84`）。目前 launcher 和 runbook 对这种布局都没有约束（同文件 `:51`；runbook:21-24 只列库名，没有目录关系要求）。

**M 的代码事实**（`admission_provider.rs:206-255`，其中 `walk` 在 `:256-281`）：
- `parent(db)` 目录本身；
- 该目录内 `db` basename 族的条目，递归枚举。族条目指名字等于 basename，或以 basename 开头且下一个字节是 `.` 或 `-` 的条目；
- admission state 目录 A 的整棵子树，递归枚举。

枚举用 `symlink_metadata`，不跟随符号链接；既不是文件也不是目录的成员即报 `SourceTypeUnsupported`（`:278-279`）。S 侧的 `db` 已经 canonicalize 过（`admission_anchor.rs:1293-1294`）。A 是 owner 原件的 `state_dir`，CLI `--admission-job-limits` 必须恰好等于 `A/s-admission/limits.json`，否则报 `Identity` E1（`admission_anchor.rs:1327-1342`，常量见 `:30`、`:32`）。

**一个嵌套反例**：A 就是 `parent(s.db)` 本身，E 的库放在 `parent(s.db)/e/e.db`。按 `:250-253`，A 等于 parent 时要枚举 parent 的全部子项，所以 `e/` 及其中的侧件都进入 M。可见冲突条件是「并发写域落入 M」，而不只是「父目录相等」。

**考古标出的冲突（本稿逐条处置）**：
1. 候选稿把 #1567 评论 6267636964 当作「resolution」引用。该评论首行写着「P 类影子模式，待用户批复」，后来被重写的签署单取代。用户实际批复的是 6310847303，落地正本是 ADR 0046（6311316288、6315085658）。本稿改引 ADR 0046:20-21 和 ADR 0028:46。
2. PR #1773 独审 M-2（`:84`）的「更稳」方案要求同时比较 launcher `state_dir`、拒绝 socket 同目录。代码事实不支持把这两项列进拒绝规则，见裁定第 5、6 条。本稿不采纳这两项。
3. runbook:40 的 `<state>/s-admission/limits.json` 和 :59 的 `state/s-admission/...` 指的是 A。runbook:35、:53 的 `state_dir` 指的是 launcher manifest 顶层字段。两者同名但不是一回事，属文档歧义。（候选稿写的「:59」正确，考古写的「:58」是空行前一行，以 :59 为准。）
4. 候选稿说「残留 socket 由 walk 报 `SourceTypeUnsupported`」，#1823 正文也这么写。实际代码顺序是：`ServeOptions::parse` 在 `writer_cli.rs:393` 调用 `socket_path`，路径已存在就报 `TransportUnavailable：socket路径已占用`（`:357-359`）。这一步早于 driver open（`:725`）。所以残留 socket 会先被这条确定性拒绝挡下，轮不到 admission walk。
5. 候选稿验收 7 把原八项夹具说成「各自独立目录」。实际夹具是 P=`<tmp>/s`、A=`<tmp>/s/s-state`（A 在 P 之下）、E/B/X 库在 `<tmp>`（`production_session_entry.rs:74-80`、`:192-195`、`:348-357`）。本稿把这一条拆成两条正控。
6. 候选稿写「#1781 实施排在 FU-16 合入之后」。#1538（FU-16）已关票，正本 ADR 0056 已在 main，这句话已过期。真实顺序约束是既有边 #1769 → #1781。

## 权威依据

- **launcher 拒启先例**：`load_manifest` 自述「完整校验 manifest；此阶段不创建任何域文件」（`rust/src/bin/production_session.rs:124-125`）。既有 `InvalidDomain` 拒绝都在拉起任何域之前：namespace 相交（`:147`）、`economic.b` 为空（`:172`）、X profile mode 不一致（`:179-184`）。runbook:51 写的是「在拉起 S、创建 S 库之前退出」。ADR 0056:53（§五.2）：「`production_session` 遇到 `TestOnly` 时钟，或时钟 kind 与 `evidence_class` 不符，在拉起任何域之前以 `InvalidDomain` 退出。」
- **独审建议**：PR #1773 独审 M-2，见 `1773-INDEPENDENT-REVIEW-6C6E.md:51-55`、`:84`，建议「launcher 校验……否则 `InvalidDomain` 拒启」。L-2 见 `:49`、`:88`，结论是「只会误拒不会误放」，S 自核留作后续。
- **改写 s.db 与已裁合同冲突**：ADR 0028:46 的失效触发包括「launcher 二进制/OS 身份/DB 路径变更」，失效即「拒绝发送，不回退旧 writer」。ADR 0046:20 规定「不存在运行时自动接管」，:21 规定「换库锚由 R4 现有身份检查拒绝」。代码层面，owner 原件 `db`、锚 `db` 都必须等于 canonical `db`，否则报 `Identity`（`admission_anchor.rs:1337-1342`）。
- **签发不建目录**：ADR 0050:44 写明「签发不建立数据库和 state 目录」「路径每级无跟随」。S 的目录布局由运维给出，S 不会替运维迁移。
- **不设可调字段**：ADR 0041:11 说「同一判断的松紧两个版本不是自由度，事前单选也不得落为可调字段」。ADR 0041:13 是判别句。另见 CONTEXT.md:393 和 AGENTS.md:46-48（#799 收敛通则）。
- **分级依据**：`.sandcastle/docs/graph-engineering.md:23`，T 类涵盖「接口与 seam、owner 落点……测试锁……恢复语义」。

## 候选

**Q1 布局冲突的处置**：
- **A 启动前拒绝**：默认值（出处：M-2 `:84`、`production_session.rs:124-200` 先例、ADR 0056:53 同款），确认或否决。推理链如下。M 包含 `parent(s.db)` 本身，打开前严格比对 stamp。E/B/X 在 S 之后无等待拉起，并在库旁建删侧件，所以同目录或嵌套布局必然间歇失败。布局是静态事实，可以在任何 init 之前确定性判出。这与 launcher 既有的拒启模式同型。它只会多拒，不会多放：S 自己的严格核验仍然在。另外它也覆盖「运维单独重启 S，而 E/B/X 仍在跑」的情形，因为布局不变式与启动顺序无关。
- **B 静默改写 s.db 或自动迁移到子目录**：排除。理由是违反已裁推论：ADR 0028:46 把 DB 路径变更定为失效触发，ADR 0046:20-21 禁止自动接管、要求换库即拒。改写后 owner 原件里的 `db` 不再等于 CLI 的 `db`（`admission_anchor.rs:1337`），S 一定拒启。
- **C 启动排序**（等 S 打开后再拉 E/B/X）：不采用，留档记录。它只能消除本 launcher 进程内的竞态，覆盖不了单独重启 S 的情形；还会把 E/B/X 的启动绑到 S 就绪上。它不违反已裁，只是收益窄。
- **D 部署前显式生成布局的工具**：保留为未来候选，本票不纳入，也不因本裁定被排除。

**Q2 冲突判定集合**：默认值为「并发写域库目录 Q 落入 {P} ∪ A 子树 ∪ S 族条目子树即冲突」，P 的祖先、兄弟目录、非族且非 A 的子目录都不算（出处：`admission_provider.rs:206-255`、`:299-345`），确认或否决。推理链：在某目录里建文件只改该目录自身的 mtime，不改其父目录的 mtime。所以只有 Q 本身是 M 的成员目录时，才会触发 stamp 或集合变化。

**Q3 两个 state_dir**：默认值为「launcher manifest `state_dir` 不进拒绝规则；A 取 `s.admission_job_limits` 的上两级目录」（出处：`production_session.rs:320-323`、`admission_anchor.rs:1336-1342`），确认或否决。推理链：launcher `state_dir` 只在 S 库不存在时、S init 之前写一次 `structure-source.json`，之后再也不碰，不与 S 打开窗口并发。

**Q4 s.socket**：默认值为不立规则（出处：`writer_cli.rs:357-359`、`:393`、`:725`、`:802`，`session_transport.rs:634-635`），确认或否决。推理链：参数期已要求 socket 路径不存在；bind 在 driver open 之后；运行期目录只核身份。残留 socket 在参数期就以 `TransportUnavailable` 确定性拒绝，不是竞态。

**Q5 L-2**：默认值为只记录、不改实现（出处：M-2 审查 `:49`、`:88`，`production_session.rs:204-207`），确认或否决。

**Q6 是否做成可调字段**：A 类，沿用 ADR 0041:11、:13。「放行冲突布局」是同一判断的宽松档：放行以后 S admission 仍按严格版拒绝，只是从确定性拒绝变成间歇失败。所以不设字段，不列为待决。

**Q7 分级**：A 类，沿用 graph-engineering.md:23 和 #1323 N-1。

**Q8 路径比较语义**：默认值为「canon = 解析符号链接后的绝对路径；路径不存在时取最近存在祖先的 canon，再接上剩余段」（出处：S 侧 `admission_anchor.rs:1293-1294` 对 db 做 canonicalize；夹具 `production_session_entry.rs:71-72` 已 canonicalize），确认或否决。推理链：冲突的物理判据是写入落在同一个真实目录。不解析符号链接的话，`/alias→P` 就能绕过检查。launcher 用 canon 比较，S 侧不跟随链接，两者语义不同，但 launcher 一侧是超集，方向同样是只多拒不多放。

**Q9 验收落点**：默认值为 `rust/tests/production_session_entry.rs` 和 CI job `rust-test-production-session`（`ci.yml:500`、`:517`、`:523`）。正控拆成两条：原夹具布局一条，五者全独立一条。确认或否决。

**Q10 runbook 订正**：默认值为订正 :40、:59 为 A，在 :53 标注 launcher `state_dir`，并给出分目录示例（出处：runbook:35、:40、:53、:59），确认或否决。

**Q11 图归属与时序**：默认值为维持挂 #1340、blocks #1781，不新增边（出处：`.sandcastle/decision-claimer.mts:36`、`:1486`；#1818 评论 6550407378；#1781 的 blocked_by 为 #1769、#1818），确认或否决。

**Q12 launcher 推导 A 时是否核验后缀 `s-admission/limits.json`**：C 类，真正待决。推荐答案：不核验。理由：后缀不合法时，S 会在 E1 以 `Identity` 确定性拒绝（`admission_anchor.rs:1336-1342`），launcher 再核一遍是同一判据的重复实现，不增加任何保护。launcher 无条件取上两级目录参与布局判定，即使后缀不合，最坏也只是误拒，不会误放。

## 裁定

本票选候选 A。下文是精确规则。

正本落点：docs/adr/0059-production-session-layout-contract.md

1. **canon 定义**：解析符号链接后的绝对路径。路径尚不存在时，取最近存在祖先的 canon，再按原样接上剩余路径段。
2. **S 测量边界**：
   - `P = parent(canon(s.db))`，`b = basename(canon(s.db))`；
   - `A = canon(parent(parent(s.admission_job_limits)))`；
   - S 族条目：P 中名字等于 `b`，或以 `b.`、`b-` 开头的条目。

   A 的推导不核验后缀，后缀合法性由 S 的 E1 核验（Q12 推荐答案）。
3. **并发写域的库**：`economic.e.db`、每个 `economic.b[i].db`、`x.db`。对每个这样的库 `d`，记 `Q = canon(parent(d))`。
4. **拒绝条件**：任一 `d` 满足下列任一条，即 `InvalidDomain`：
   - `Q == P`；
   - `Q == A`，或 `Q` 位于 `A` 之下；
   - `Q` 等于某个 S 族条目的路径，或位于其下。

   `d` 本身取了 P 内的 S 族名字，已被 `Q == P` 覆盖。S 侧原本也会以 `UnknownDatabaseSidecar` 确定性拒绝（`admission_provider.rs:229-243`），launcher 只是把判定提前。
5. **不比较 launcher manifest `state_dir`**。理由是它只在 S init 之前写一次（`production_session.rs:320-323`），不与 S 打开窗口并发。
6. **不比较 `s.socket`**。残留 socket 在 S 参数期就以 `TransportUnavailable` 确定性拒绝（`writer_cli.rs:357-359`）。重启时谁来清理，归 #1823 调查，不在本裁定范围内。
7. **不扩大范围**：P 的祖先目录、兄弟目录，以及 P 内既非族条目、也不在 A 内的子目录，都不在 M 内，一律放行。
8. **错误信息**：stderr 必须含 `InvalidDomain`，并点名冲突的两条 manifest 字段及两侧 canon 路径。被冲突的字段按情形取：与 P 或族条目冲突时是 `s.db`，与 A 冲突时是 `s.admission_job_limits`。并发写域的字段名为 `economic.e.db`、`economic.b[<i>].db` 或 `x.db`。
9. owner 落点：`load_manifest`（`rust/src/bin/production_session.rs:125`）。检查位于其返回 `Ok` 之前，因而早于 `check_existing_s`（`:300-302`）、任何 init、写 launcher `state_dir` 以及任何拉起。命中时不建任何域文件。
10. **不设可调字段，不设放行开关**。依据是 ADR 0041:13 的判别句和 #799。launcher 判定和 S admission 核验判的是两件事：前者是静态布局，后者是打开窗口内的 stamp。两者不在同一个 if-else 上，也不互相兜底。
11. **L-2 记录**：immutable 预检最多误拒、不会误放；改由 S 自核需要改 S v3 writer CLI，另行开票。本裁定不改实现。
12. **runbook**：`docs/runbook_production_session.md` 写明 M 的三部分和第 4 条拒绝规则；:40、:59 改写为「admission state 目录 A（owner 原件 `state_dir`）」；:53 标注「launcher 自身 `state_dir`」；给出 `/abs/s/s.db`、`/abs/s-state/`、`/abs/e/`、`/abs/b/`、`/abs/x/` 的分目录示例。
13. 本裁定不改资金、隔离、恢复帽、admission 判据，不签 P0，不改 Destination。

## 验收清单

所有测试都落在 `rust/tests/production_session_entry.rs`。判据：在实施 PR 精确 head 的 `rust-test-production-session` job 中，`cargo test — production_session_entry` step（ci.yml:517）的日志里每个测试名逐行出现 `test <名> ... ok`，且两条 step（:517、:523）均以 `test result: ok.` 结束。

1. `layout_rejects_e_db_in_s_db_parent`：E 库与 S 库同目录。launcher 非零退出，stderr 含 `InvalidDomain`、`economic.e.db`、`s.db`。断言 S、E、B、X 库及其侧件都不存在，launcher `state_dir/structure-source.json` 不存在。
2. `layout_rejects_b_db_in_s_db_parent`：同第 1 条，点名 `economic.b[0].db`。
3. `layout_rejects_x_db_in_s_db_parent`：同第 1 条，点名 `x.db`。
4. `layout_rejects_e_db_under_admission_state`：A 就是 P，E 库在 P 下一个已存在的子目录里。拒绝，stderr 含 `s.admission_job_limits`，不产生任何域文件。
5. `layout_rejects_e_db_under_separate_admission_state`：A 与 P 分离，E 库在 A 之下。拒绝。
6. `layout_rejects_e_db_under_s_family_entry`：E 库目录是 P 内的一个 S 族目录（如 `s.db.admission-private`）或其子目录。拒绝。
7. `layout_rejects_symlink_alias_of_s_parent`：E 库目录是指向 P 的符号链接。拒绝。
8. `layout_resolves_not_yet_existing_db_dirs`：E 库父目录 `L/new` 尚不存在，L 是指向 A 的符号链接。拒绝。
9. `layout_accepts_db_in_non_family_subdir_of_s_parent`：A 与 P 分离，E 库在 P 下一个已存在、非族的子目录里。launcher 正常拉起 S 和 E。
10. `layout_accepts_original_fixture_layout`：沿用原八项夹具，即 A 在 P 之下、E/B/X 在 P 的父目录。放行。
11. `layout_accepts_separate_directories`：S、A、E、B、X 各自独立目录。放行。
12. `layout_allows_launcher_state_dir_equal_s_parent`：launcher `state_dir == P`，其余分离。放行。
13. 现有测试（`simulation_profile_spawns_isolated_domain_processes` 等）在 :517 step 逐行 `ok`。
14. 正本文件存在：`ls docs/adr/ | grep -c -- '-production-session-layout-contract.md$'` 输出 `1`。编号在合入时按 main 顺序确定，等于当时 main 上最大编号加一（现最大为 0057）。
15. 正本锚句：以下每条在该 ADR 中的 `rg -c` 命中数都 ≥1。
    - `rg -c '最近存在祖先'`
    - `rg -c 'InvalidDomain'`
    - `rg -c '不比较 launcher manifest `state_dir`'`
    - `rg -c '不比较 `s.socket`'`
    - `rg -c 'owner 落点：`load_manifest`'`
    - `rg -c '不设可调字段'`
    - `rg -c 'L-2 记录'`
    - `rg -c '启动排序'`
16. runbook 检查：
    - `rg -c '<state>/s-admission' docs/runbook_production_session.md`：无命中（退出码 1）；
    - `rg -c 'admission state 目录' docs/runbook_production_session.md` ≥1；
    - `rg -c '测量域' docs/runbook_production_session.md` ≥1；
    - `rg -c '/abs/e/' docs/runbook_production_session.md` ≥1。
17. 票状态：resolution 落地后 `gh api repos/xy7365527-lang/NewChanlun/issues/1818 --jq .state` 输出 `closed`；`gh api repos/xy7365527-lang/NewChanlun/issues/1781/dependencies/blocked_by --jq '.[].number'` 仍含 `1769`。
18. resolution 首行等于「本条依 #1323 Notes N-1 预授权判定」。

## 受影响票与 blocking 边变化

- **#1818**（本票）：resolution 落地并经独审后关票。既有边 #1818 → #1781 由关票自然解除，不另删边。
- **#1781**：保留 blocked_by #1769，以及 #1818（直至本票关闭）。实施范围更新如下：
  - 验收 1 定为「启动前拒绝」，即本裁定第 1–10 条；「自动分目录」已排除；
  - 验收 2 按第 12 条执行；
  - 验收 3 按第 11 条只做记录。

  不新增边。
- **#1769**：边 #1769 → #1781 保持，无变化。
- **#1538（FU-16）**：已关票，不加边。订正候选稿里「排在 FU-16 合入之后」的措辞。
- **#1823**：不加边，与该票自述「不接 blocked_by #1818」一致。附注：该票正文所写「残留 socket 在测量域内报 `SourceTypeUnsupported`」与代码顺序不符，见问题第 4 条。残留 socket 先以 `TransportUnavailable：socket路径已占用` 拒绝（`writer_cli.rs:357-359`、`:393`）。该票复现时可以据此核对。
- **#1340**：仍是本票父票，不变。
- **#1323**：图不变，可在 Notes 追加一行 resolution 索引。
- **#1527、#1567/ADR 0046**：不改。
- 另行开票（不阻塞任何票）：L-2「launcher 结构源预检改为 S `serve --structure-source` 自核」，实施票，需改 S v3 writer CLI。
- 不新增 research 或 probe 票：考古各点都是 A/B 类或代码事实，没有事实缺口。

## 授权引用

本条依 #1323 Notes N-1 预授权判定。N-1 见 #1323 正文 :74，用户原话见 [#1323 评论 6261281111](https://github.com/xy7365527-lang/NewChanlun/issues/1323#issuecomment-6261281111)：用户 2026-10-18 原话「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」。分级为 T 类（#1818 评论 6550474840），依据是 `.sandcastle/docs/graph-engineering.md:23`。本票不涉资金、隔离放宽、真实场所、关图或 Destination 改写。

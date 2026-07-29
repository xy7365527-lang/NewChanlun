# #511 三轮复审：#513 + #514 核销（`ce074711d5` / `4b2b69afc6`）

日期：2026-07-28  
工作区：`/tmp/kimi-nest-mainline`  
固定点：`4b2b69afc661b2c102d59e5cfef0fedbdaf1a034`  
评审方式：只读；未改仓内文件、未做 git mutation；独立负例只使用自动清理的临时目录。

## 结论

**维持打回。**

`#514` 已核销显式点名的 commit 可解析性、Git 不可用 fail-closed、lineage 倒挂、
字符串类型、11/11 Python 单测及边界声明；当前 committed golden 的
`a12a1022d9… → 6e15ceffee…` 也合法。

但全链终查发现一个仍能假绿的同锚组合：当锚同时含可选
`source_lineage_start` 与必填 `source_base_head` 时，
`scripts/check_armR_trades_digest.py:213-223` 二选一只校验 lineage，完全跳过 base。
构造 `lineage=a12a… / base=6e15… / final=a12a…`，lineage→final 合法而
base→final 倒挂，真实 `main()` 仍打印三窗全绿并 **exit 0**。

这直接违反前轮明确写下的
`git merge-base --is-ancestor source_base_head final_verification_head`
回修门；因此“三轮残留祖先关系”只部分核销，不能签 PASS。

| 轴 | 结论 | HIGH | MED | LOW |
|---|---|---:|---:|---:|
| Standards | **打回** | 0 | 1 | 3 |
| Spec | **打回** | 0 | 1 | 0 |
| 合计（同一 MED 去重） | **打回** | **0** | **1** | **3** |

## 1. 核销表

| 核销项 | 结论 | 独立证据 |
|---|---|---|
| HEAD 可解析性 | **PASS** | `HEAD_FIELDS` 三字段均先做 40 位小写 hex，再调用 `git cat-file -e <sha>^{commit}`（脚本 `:196-212`）。`source_base_head="0"*40` 的真实门 exit 1，报“仓内不可解析 commit”。 |
| Git 不可用 fail-closed | **PASS** | `_git_commit_problem` / `_git_ancestry_problem` 在 `:139-170` 捕获 `OSError` 并返回问题。独立注入 `FileNotFoundError("git absent")` 后真实门 exit 1，两个必填 HEAD 均报 Git 无法执行。 |
| 当前 committed golden 可解析 | **PASS** | golden `:7-8` 的 `a12a1022d9…`、`6e15ceffee…` 均可由 `git cat-file` 解析。无参门亦对二者完成相同检查并 exit 0。 |
| 当前 committed golden 祖先关系 | **PASS** | 独立 `git merge-base --is-ancestor a12a1022d9… 6e15ceffee…` exit 0；反向命令 exit 1。 |
| 显式 lineage 倒挂负例 | **PASS（仅该形态）** | `source_lineage_start=6e15… / final=a12a…` 的真实门 exit 1，报“不是 final_verification_head 的祖先”；测试 `test_check_rejects_reversed_lineage_ancestry` 亦覆盖。 |
| 同锚 lineage/base → final 完整性 | **FAIL** | 脚本 `:213-223` 使用 `lineage if present else base`。独立组合 `lineage=a12a… / base=6e15… / final=a12a…` 中 lineage→final exit 0、base→final exit 1，但门 exit 0，`provenance_problems=[]`。 |
| 字符串类型 | **PASS** | `_note` 在 `:177-178`；`source_worktree`、两条 command、`dump_dir`、`parallel_head_note` 在 `:73-79,191-195` 做 `isinstance(..., str)`。整数 `regen_command=123` 的真实门 exit 1。 |
| Python 单测 | **PASS** | `python3 -m unittest scripts.tests.test_check_armR_trades_digest`：`Ran 11 tests in 0.354s`，`OK`。但现有 11 测没有覆盖“lineage 合法、base 倒挂”的组合。 |
| 边界声明 | **PASS** | docstring `:27-31` 明确“可解析 + 祖先成立”不证明 dump 由该 HEAD 产生，并明确当前无 sidecar/manifest；`4b2b69afc6` 两文件 diff 未实装 sidecar。 |
| 无参门 | **PASS** | exit 0；p3fold/wf7/wf8 为 `141/184/159`，digest 为 `ac5952…/981a05…/18291f…`，bytes 为 `195220/253658/216129`。 |
| `4b2b69afc6` 全 diff | **PASS** | `git show 4b2b69afc6` 已逐行审阅；恰改 `scripts/check_armR_trades_digest.py`、`scripts/tests/test_check_armR_trades_digest.py`，`+129/-6`；`git diff 4b2b69afc6^ 4b2b69afc6 --check` exit 0。 |
| `cargo test --lib` | **PASS（在案基线）** | 当前共享工位实跑：`1992 passed; 1 failed; 135 ignored`。唯一失败仍为 #115 `extract_signals_bit_exact_digest_guard`；输出中的 `lee_m3_*` 六项与 `lee_m4_*` 两项共 8/8 绿。 |
| 订正块归因 | **PASS** | 见 §4；未发现跨票误标。#513/#514 都只改脚本与测试，没有伪造仓内报告订正块。 |

## 2. 阻塞 finding

### MED：可选 lineage 遮蔽必填 base 的祖先倒挂

**位置**

- `scripts/check_armR_trades_digest.py:188-212`：`source_base_head` 是必填字段，并与
  `source_lineage_start` 一样完成格式及 commit 可解析检查。
- `scripts/check_armR_trades_digest.py:213-223`：祖先检查却只选一个字段；
  lineage 存在时不再检查 base。
- `scripts/tests/test_check_armR_trades_digest.py:121-131`：只覆盖 lineage 自身倒挂，
  没有覆盖“两字段并存、lineage 合法、base 倒挂”。

**独立真实门反例**

```text
source_lineage_start      = a12a1022d9ddd8d1cae867a107a3a33c359358cf
source_base_head          = 6e15ceffeeb8259c065bf7c0ec9ec7c65935737c
final_verification_head   = a12a1022d9ddd8d1cae867a107a3a33c359358cf

lineage -> final          = exit 0（相等）
base -> final             = exit 1（倒挂）
真实 digest 门            = exit 0（三窗全绿）
```

**为何阻塞**

- `/tmp/shadow-review-511-recheck-20260728.md:57-59` 明确要求
  `source_base_head` 是 `final_verification_head` 的祖先或相等。
- `/tmp/shadow-review-511-recheck2-20260728.md:78-80` 再次逐字规定
  `git merge-base --is-ancestor source_base_head final_verification_head`。
- 当前实现允许可选字段遮蔽必填字段，违反
  `.claude/rules/common/coding-style.md` 的系统边界完整校验 / fail-fast，也不满足 090 的
  声明—能力一致纪律。

本 finding 只判门能力，不指控当前 golden；当前 golden 没有 lineage，base→final 独立验证合法。

**最小回修门**

始终检查 `source_base_head → final_verification_head`；若
`source_lineage_start` 存在，再额外检查
`source_lineage_start → final_verification_head`。补上述组合的真实门红测。

## 3. 首轮 5 项 + 二轮 2 项 + 三轮 1 项终查

| 轮次 | 历史项 | 终判 |
|---|---|---|
| 首轮 1/5 | Standards MED-1：release fail-loud | **PASS**；#512 已前移至 coverage 生产边界 panic。`8d8895c652..4b2b69afc6` 的 Rust 面为空，未被 #513/#514 回退。 |
| 首轮 2/5 | Standards MED-2：canonical 当前态 | **PASS**；默认根仍为 `/tmp/m8_win_gate`，T3 §15.6 订正未变，无参门 exit 0。 |
| 首轮 3/5 | Standards MED-3：golden/provenance | **FAIL（仍为部分核销）**；v2 追加迁移、格式、日期、计数、commit 可解析、字符串类型均通过，但 base 祖先关系可被 lineage 遮蔽。 |
| 首轮 4/5 | Spec MED-1：逐笔 diff 旧侧路径 | **PASS**；`/tmp/446_armR_trades_diff.md:3` 与 JSON `:2` 仍明确旧根 `/tmp/423_backup_m8_win_gate`、新根 `/tmp/446_armR_dump`。 |
| 首轮 5/5 | Spec MED-2：污染面登记 | **PASS**；T3 §15.7 仍保持“已重验 / superseded / 未复核”三态，T2 `:622-634` 旧数显式 superseded。 |
| 二轮 1/2 | provenance schema 校验 | **PASS（该轮显式子项）**；HEAD 格式、digest、正整数禁 bool、ISO 日期、整块 provenance 缺失等既有负例与新增单测均绿。 |
| 二轮 2/2 | docstring 跨票前缀/canonical | **PASS**；命令使用中性 `/tmp/m8_win_gate*`，并把 `/tmp/446_armR_dump` 限定为历史锚。 |
| 三轮 1/1 | commit 可解析 + 祖先 + 字符串类型 | **FAIL（部分核销）**；可解析、Git fail-closed、字符串类型通过；祖先检查遗漏同锚必填 base 的并存形态。 |

结论不是“遗漏未查”，而是八项已逐条终查后仍有一项未满足。

## 4. 订正块归因终扫

| 票号 | 终扫结论 |
|---|---|
| #446 | T1 §14、T3 §15 只归活动集同 `ElementId` 双计修复；OKLO 仅引用其 LEE/基线。 |
| #475 | T2/T3/OKLO 的历史复审 finding 与 #495 配对；二轮补订正与 #496 配对。 |
| #481 | OKLO §2.9 只归回归计数复审，没有交叉标到 #511 链。 |
| #484 | 只归 #481 首轮回修及 OKLO §2.9 的开工/修复后读数。 |
| #490 | 只归 #481 复审后的 FeeAudit 净额同域、展示/header 修复 provenance。 |
| #492 | 只归 #481 二轮复审的 fill 列名域残留。 |
| #495 | 只归 #475 首轮修复；T2/T3/OKLO 订正均与 #475 配对。 |
| #496 | 只归 #475 二轮补订正；T3 `:249-254` 显式标 `#475 / #496`。 |
| #511 | T3 §15.6、T2 `:622-634` 与 §15.8 LOW 登记只归复审票。 |
| #512 | T3 §15.6/§15.7、T2 末尾块只归首轮回修；Rust/docs/golden 自 #512 后未被 #513/#514 改动。 |
| #513 | `ce074711d5` 只归 schema/docstring 残留，恰两份 Python 文件；未伪造报告订正块。 |
| #514 | `4b2b69afc6` 只归 commit 可解析/祖先/字符串类型残留，恰两份 Python 文件；本轮 finding 正在该实现边界内。 |

## 5. 新发现（非阻塞）

1. **LOW：函数体超出仓库建议上限。**
   `provenance_problems` 当前约 81 行（`:173-253`），超过 common checklist 的函数 `<50`
   要求；可拆成单锚校验 helper。
2. **LOW：新增/改名测试方法缺返回类型注解。**
   `test_check_armR_trades_digest.py:78,92,111,121` 未写 `-> None`，不符合
   `.claude/rules/python/coding-style.md` 的“所有函数签名使用 type annotations”。
3. **LOW：测试 valid fixture 硬编码历史 commit。**
   `_valid_payload` 在 `:35-36` 使用历史 SHA `c126d4cf…`。当前完整仓 11/11 绿，但浅克隆或
   裁剪历史会使“valid”夹具失效；且缺少一个完整 valid payload 必须 exit 0 的正例，负例存在
   非单因通过风险。
4. **Fowler 判断项（不计入 LOW）：possible Duplicated Code。**
   两个 Git helper 重复 `subprocess.run + OSError fail-closed` 骨架，可考虑抽统一 runner；
   不构成本轮阻塞。

## 6. Standards / Spec 双轴

### Standards

**MED×1**：可选 lineage 遮蔽必填 base，违反边界完整校验、fail-fast 与 090
声明—能力一致。另有 LOW×3 和一个 Duplicated Code 判断项，见 §5。

### Spec

**MED×1**：前两轮逐字要求的
`source_base_head → final_verification_head` 没有在“lineage 同时存在”时执行，
因此三轮祖先残留未完整核销。其余硬清单无 missing、scope creep 或语义错误。

## 7. 未覆盖与不能外推

1. 无外网、未使用 `gh`；没有核在线 #511/#512/#513/#514 票体，权威输入限于用户硬清单、
   三份 `/tmp` 历史报告、提交 diff、仓内规则与文档。
2. 为保持只读，没有对仓内 golden 实际执行 `--regen`；regen 行为沿用临时文件单测。
3. 没有重跑 BTC release wf8、其余八窗、L3、incremental、dual-open 或更早历史消费族；
   本轮亲跑的是指定 Python 单测、无参 digest 门与全量 `cargo test --lib`。
4. `cargo test --lib` 是当前共享工位指纹，不是 target commit 的干净快照。评审期间共享工位
   并行改动从初始的 `nest_lifecycle.rs` 扩展到 `p123_fast_replay.rs`；这些文件未触碰、
   未归因，本票两份 Python 文件始终 clean。
5. 没有实现或验证 dump↔HEAD sidecar/manifest 内容绑定；docstring 已诚实声明该能力不存在，
   本轮不把“commit 可解析 + 祖先成立”外推为 dump 来源证明。
6. 未修改发现的问题；唯一评审产物为
   `/tmp/shadow-review-511-recheck3-20260728.md`。

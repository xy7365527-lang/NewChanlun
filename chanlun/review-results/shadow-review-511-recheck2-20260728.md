# #511 二轮复审：#512 + #513 核销（`8d8895c652` / `ce074711d5`）

日期：2026-07-28  
工作区：`/tmp/kimi-nest-mainline`  
固定点：`ce074711d5edbb18b59fa5f0990907f78d2cba49`  
评审方式：只读；未改仓内文件、未做 git mutation；独立负例脚本仅写
`/tmp/rev513_provenance_probe.py`

## 结论

**维持打回。**

本轮硬清单显式列出的五类 schema 负例、docstring 前缀、golden 保持、2 文件 diff、8/8
Python 单测、无参门及 `cargo test --lib` 指纹均通过。但复核纪律第 4 项要求对照上一轮
“维持打回”项确认无遗漏；上一轮最小回修门明确还要求：

1. 两个 HEAD 可解析为 commit，且 `source_base_head` 是
   `final_verification_head` 的祖先或相等；
2. `source_worktree`、`regen_command`、`check_command` 是非空字符串。

`ce074711d5` 只对 HEAD 做 40 位小写 hex 格式校验，对三个描述字段仍只做 truthy 检查。
独立真实门反例证明：三个字段分别改为整数 `1` 均 exit 0；两个 HEAD 改为不存在的
`cccc…/dddd…` 40 位 hex 也 exit 0。因此上一轮的 provenance 真值/类型 MED 尚未完整核销。
当前 committed golden 的两个 HEAD 本身可解析且祖先链正确；finding 针对门能力，不指控当前
golden 伪造。

| 轴 | 结论 | HIGH | MED | LOW |
|---|---|---:|---:|---:|
| Standards | **打回** | 0 | 1 | 3 |
| Spec | **打回** | 0 | 1 | 0 |
| 合计（同一阻塞项去重） | **打回** | **0** | **1** | **3** |

## 1. 核销表

| 核销项 | 结论 | 独立证据 |
|---|---|---|
| provenance：HEAD 40 位 hex fullmatch | **PASS（仅格式）** | `scripts/check_armR_trades_digest.py:137-142`；`forged-commit` 与整数 HEAD 均令真实 `main()` exit 1。 |
| provenance：digest 16 位 hex | **PASS** | `:152-162`；`0xNOT-A-DIGEST!` 令门 exit 1。 |
| provenance：计数为正整数且禁 bool | **PASS** | `:163-166` 使用 `type(value) is int and value > 0`；`n_trades=0`、`bytes=True` 均 exit 1。 |
| provenance：ISO 日期解析并往返 | **PASS** | `:143-151`；`2026-02-30` 令门 exit 1。 |
| provenance：上一轮完整真值/类型门 | **FAIL** | 上一轮报告 `:57-59` 明列 commit 可解析、祖先关系、命令/工位非空字符串；本提交未实现。整数描述字段与不存在但格式合法的 HEAD 仍 exit 0。 |
| Python 单测 | **PASS** | `python3 -m unittest scripts.tests.test_check_armR_trades_digest`：`Ran 8 tests`，`OK`。 |
| docstring canonical 根与中性前缀 | **PASS** | `check_armR_trades_digest.py:13-21` 使用 `/tmp/m8_win_gate/$tag` 与无票号 `m8_win_gate_report_$tag.md`。 |
| `/tmp/446_armR_dump` 历史锚口径 | **PASS** | docstring `:23-25` 明确历史锚不等于 current canonical。 |
| golden `/tmp/446` 字段保持 | **PASS** | 父/子提交 golden blob 均为 `9b7a8daad575196a664acd205cd322c137d9988e`；当前 JSON `:12-14` 仍保留 `/tmp/446_armR_dump`。 |
| `ce074711d5` 全 diff | **PASS** | 恰 `scripts/check_armR_trades_digest.py`、`scripts/tests/test_check_armR_trades_digest.py`，`+129/-1`；`git diff --check` exit 0。 |
| 无参门 | **PASS** | exit 0；p3fold/wf7/wf8 为 `141/184/159`，digest 为 `ac5952…/981a05…/18291f…`，bytes 为 `195220/253658/216129`。 |
| `cargo test --lib` | **PASS（基线指纹）** | `1991 passed; 1 failed; 135 ignored`；唯一失败仍为 #115 `extract_signals_bit_exact_digest_guard`。 |
| 订正块归因终扫 | **PASS** | 见 §4；本提交没有新增订正块，也未交叉归因。 |

## 2. 阻塞 finding

### MED-1：上一轮 provenance 真值/类型回修门只完成格式子集

上一轮报告 `/tmp/shadow-review-511-recheck-20260728.md:12,35-59` 的阻塞点不是单纯
“字符串长得像 SHA”，而是 provenance 漂移/错误类型不能假绿；其最小回修门明确包含 commit
可解析性、祖先关系，以及命令/工位描述的非空字符串类型。

当前实现：

- `check_armR_trades_digest.py:134-136` 对全部 required 字段只做 truthy 检查；
- `:137-142` 只给两个 HEAD 增加正则格式；
- `:143-151` 只给日期增加类型/语义检查；
- 没有校验三个描述字段的字符串类型，也没有调用 Git 验证 commit/祖先关系。

独立探针在真实 `/tmp/m8_win_gate` 与临时 golden 上调用同一个 `main()`：

```text
integer_source_worktree: exit=0
integer_regen_command: exit=0
integer_check_command: exit=0
syntactically_hex_forged_heads: exit=0
```

这使复核纪律的“前轮维持打回项无任何遗漏”不成立，也与
`.claude/rules/common/coding-style.md` 的系统边界 schema 校验及 090 的声明—能力一致要求不符。

最小回修门：为三个描述字段加入 `isinstance(value, str) and value.strip()`；对两个 HEAD 使用
`git cat-file -e <head>^{commit}` 或等价解析，并验证
`git merge-base --is-ancestor source_base_head final_verification_head`。补对应正常门红测，且不得
破坏本轮已通过的五类负例。

## 3. 新发现（非阻塞）

### LOW-N1：`windows` 容器类型错误会 traceback，而非字段级 schema 错误

`provenance_problems` 在 `check_armR_trades_digest.py:152-157` 遇到非 dict `windows` 或非 dict
窗条目时只跳过；`diff_report`（`:220-229`）随后无条件 `.get()`。独立反例：

```text
windows=[]  -> AttributeError: 'list' object has no attribute 'get'
wf7=1      -> AttributeError: 'int' object has no attribute 'get'
```

进程会非零退出，因此不是 false green；但它不满足脚本声明的字段级差异和仓库的清晰边界错误
要求。建议由 schema 层明确登记 `windows: 必须是对象` / `windows.<tag>: 必须是对象`。

### LOW-N2：新增 Python 函数/方法缺类型注解

`scripts/tests/test_check_armR_trades_digest.py:26,49,57,69,78,87,96,108` 新增的 helper、
fixture method 和测试方法没有完整签名注解，违反
`.claude/rules/python/coding-style.md` 的“所有函数签名使用 type annotations”。

### LOW-N3：新增实现未按 Black 形态换行

`scripts/check_armR_trades_digest.py:139,145,159` 为 90/106/94 字符的新增逻辑行，Black 会拆行。
本机没有安装 Black/Ruff，本项为静态格式判断；`git diff --check` 本身为 green。

## 4. 订正块归因普查

| 票号 | 终扫结论 |
|---|---|
| #446 | T1 §14、T3 §15 继续只归活动集唯一性修复；OKLO 中仅作为既有 LEE 测试/基线引用。 |
| #475 | T2/T3/OKLO 的历史复审 finding 均继续与修复票 #495 配对；T3 二轮补订正与 #496 配对。 |
| #481 | OKLO §2.9 只归其回归计数复审；未残留 `#481 HIGH-*` 误标到 #511/#512。 |
| #484 | 只归 #481 首轮修复及 OKLO §2.9 的开工/修复后读数。 |
| #490 | 只归 #481 复审后的审计表/header 修复 provenance。 |
| #492 | 只归 #481 二轮复审列名域残留修复 provenance。 |
| #495 | 只归 #475 首轮修复；T2/T3/OKLO 订正块均与 #475 配对。 |
| #496 | 只归 #475 二轮复审补订正；T3 `:249` 已显式标 `#496`。 |
| #511 | T3 §15.6、T2 末尾订正块及 §15.8 LOW 登记继续归本复审票。 |
| #512 | T3 §15.6/§15.7 与 T2 末尾块继续归首轮回修；没有占用 #513 的 schema/docstring 修复。 |
| #513 | 只在当前 commit provenance/docstring 修复线；本提交未新增报告订正块，故无须伪造块内票号。 |

## 5. 前两轮纪律对照

| 历史项 | 二轮终判 |
|---|---|
| 首轮 Standards MED-1：release fail-loud | **PASS**；#512 已核销，本提交未触 Rust。 |
| 首轮 Standards MED-2：canonical 当前态 | **PASS**；T3 §15.6 与默认根保持。 |
| 首轮 Standards MED-3：golden/provenance | **FAIL（仍为部分核销）**；本轮格式/计数/日期子集通过，但上一轮明确的 commit 语义及描述字段类型仍缺。 |
| 首轮 Spec MED-1：逐笔 diff 旧侧路径 | **PASS**；仍指 `/tmp/423_backup_m8_win_gate`。 |
| 首轮 Spec MED-2：污染面登记 | **PASS**；T3 §15.7 三态登记未被本提交改动。 |
| 复审残留 MED：provenance 真值检查 | **FAIL（部分核销）**；见 MED-1。 |
| 复审残留 LOW：`/tmp/484_*` docstring 前缀 | **PASS**；已改 canonical/中性前缀，并解释 `/tmp/446` 历史锚。 |

首轮 LOW×2（大文件、长测试/夹具重复）已在 T3 §15.8 归 #493，不是本轮核销对象，也没有因
本提交触碰 Python 两文件而发生 Rust 侧回归。

## 6. Standards / Spec 两轴摘要

### Standards

本轮显式五类 schema 负例和 docstring 均通过，但上一轮已明确的 provenance 真值/类型边界仍
可假绿，构成 1 个阻塞 MED。另有容器 traceback、缺类型注解、Black 形态共 3 个 LOW；未见
baseline code smell。

### Spec

按本轮第 1-3 项的逐字硬清单均通过；第 4 项“前两轮维持打回项无任何遗漏”不通过。没有发现
两文件之外的 scope creep，也没有 golden 静默改写。

## 7. 未覆盖与不能外推

1. 无外网、未使用 `gh`；没有核在线 #511/#512/#513 票体，权威输入限于用户硬清单、两个
   `/tmp` 历史报告、提交 diff 与仓内规则/文档。
2. 未实际对仓内 golden 执行 `--regen`，避免只读评审改文件；追加/迁移行为沿用既有单测，
   本轮只核正常门。
3. 未重跑 BTC release wf8、其余八窗、L3、incremental、dual-open 或历史消费族；本轮要求的
   全量 `cargo test --lib` 已亲跑。
4. 40 位 HEAD 与 dump 的生成时点仍没有 sidecar/manifest 内容绑定；即使补 Git
   可解析/祖先检查，也不能由此证明 dump 确由该 HEAD 产生。
5. 未触碰或归因共享工位既有
   `M chanlun/agent-roster-2026-07-21.md` 及两份未跟踪 shadow 报告。评审结束 HEAD 仍为
   `ce074711d5edbb18b59fa5f0990907f78d2cba49`。

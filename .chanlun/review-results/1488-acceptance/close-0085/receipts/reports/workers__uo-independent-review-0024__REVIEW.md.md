# 收据 workers/uo-independent-review-0024/REVIEW.md

- 原件：`workers/uo-independent-review-0024/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`7a378761f7d56ed759be7967eb5764a3249414eb95bbe804eb637ddda2a489fc`
- 字节数：13947；行数：145

## 关键行（行号: 原文）

```
1: # #1488 统一所有权预算补丁独立评审 0024：FAIL
4: - 被审差异：`git diff 959d86006584 5759bbec6a -- rust/`，22 文件，+1226/−209。
5: - 依据：合同 v4、implementation-v1、DESIGN-CHALLENGE、上一轮独审（813e，3P1/2P2）、实施报告 0017/0021/0022/0023，以及 #1488 主控对 r14 和 fixed_radix 的两条裁定。
7: 结论：**FAIL**。合同 v4 列为「必须一轮闭合」的反例 C 没有闭合：代码仍用 `max` 补差，也没有逐项 E/O 映射。其余各项大体达标，另有若干 P2/P3。
11: ## 1. 反例 A–F 闭合情况与 adopt / MoveIn 退出生产：**FAIL**（C 未闭合；adopt_native 文义未全退）
13: ### A 冷首次分配：PASS
21: ### B Observer：PASS
23: - 1000/100/700/200/M128 这组锁 PASS（CI 39271905697）。
26: ### C E/O 交集：**FAIL（P1-1）**
33: ### D PendingRawState 许可早退：PASS
36: ### E 旧 700 + 新 20、旧域与错父拒绝：PASS
41: ### F 1024 / 16 / 8MiB / 六 driver / 128 probe：PASS（见第 3 项）
43: ### adopt 残留全仓核（rg）
47: - 生产 `Budget::new(limit, retained)`（`native.rs:494-508`）仍会把 `retained` 写进 `moved_in`，且未 cfg(test)。现在所有生产调用都传 0，非零调用都在测试里。所以运行时没有可用的 unassigned 余额，但合同 v4 第 5 条要求 adopt_native「退出生产路径」，文义没有满足（P2-1）。
50: ## 2. 上一轮独审 3P1 / 2P2：P1-1、P1-2、P2-1、P2-2 已解决；P1-3 基本解决，仍有残余
51: - **P1-1 旧费重复扣 A：已解决**。`bind_frozen` 中 `A = T − legacy − O`；换域后 `remaining = A`，`total_limit = L + A`，并检查 `L ≤ O`（`native.rs:550-585`），Carry 不再从 A 扣旧费。但 O 本身含 max 补差，并可能把封存池费用计两次（见 P1-1）。
52: - **P1-2 父 metadata M 未计：已解决**。
55: - **P1-3 无主费用：基本解决**。
58:   - 残余 1：`rebirth_vec`/`rebirth_codec_vec` 承认存在无票出生的旧 Plain 块（P2-3）。
59:   - 残余 2：journal 费用是事后同额预留（P2-2）。
60: - **P2-1 旧域与部分修改：已解决**（见 E）。
61: - **P2-2 BusinessMap 外来父：已解决**。`audit_json.rs:2102-2114` 先 `check_current`，再查 `belongs_to_parent`，不匹配就拒绝且不改账。这是 959d 之前已落的改动。
63: ## 3. 原断言保护：PASS
64: - 全 diff 中被删除的 `assert` 只有 1 条：r14 的 `parent.live() > before_seal`（diff:688）。主控裁定允许删除它，同句信息改由 `sealed > 0` / `prior_sealed > 0` 承接。
72:   - :781 成功插入峰值 `2*leaf+branch+M`：机制与裁定的拒绝分支完全相同（每次 `run()` 的 `CodecWork::new` 先预留 M），实现未动，原 message 的「同时计费」本意不变，**判定符合裁定意图**。但它超出了裁定字面（裁定只点名 offset 分支），建议主控追认一句（P3-2）。
75: ## 4. r14 新恒等式：部分满足主控意图（P2-4）
90: ## 5. 业务语义：PASS
98:   - `commit.rs`、`content_bi.rs`：`lookup_path` 改为 `lookup_path_native`，没有活动 scope 时走同一个 `lookup_path_output`。这一点只做了静态比对，靠锁全 PASS 旁证。
101: ## 6. 冷根审计失败后无票重跑：基本 PASS（证据不足一处，P3-1）
102: - `audit_owner.rs:1489-1495`：根内审计遇到任何 `Err` 就丢弃 slot、析构根，然后执行 `audit_required_in_parent(mode, context, None)?`。这与基线冷路径（无父无票）是同一调用，真实 SQL 错误由重跑原样返回，顺序不变。1024 不足时不留候选，回复成功（driver 锁 PASS）。
109: ## 7. 测试结果：PASS（与报告一致）
111: - CI run 39268783566：headSha=ec9456a4409d…（gh 核实）。artifact `-1-` 的 locks.tsv 有 34 行，与 `git show ec9456a440:.github/1488-uo-locks.txt` 的 34 项完全一致：30 PASS / 4 FAIL(101)。`-0-`（s_session 组）34 项全为 NOT_LISTED。
112: - 另有 CI 39271905697（5759bbec6a）：31 PASS / 4 FAIL；fixed_radix 锁在两组里都 PASS。
113: - 4 条 FAIL 与报告所述原因一致：
126: ## Findings
128: ### P1
129: - **P1-1 反例 C 未闭合，使用了 max 补差**
135: ### P2
136: - **P2-1 生产 adopt_native 未退出**：`native.rs:1373`、`s_content/capture_allocation/work.rs:230,250`、`scratch.rs:37-42,453`；`Budget::new(_, retained)`（`native.rs:494`）未 cfg(test)。反例：只要以后有一处生产调用传入非零 retained，`work.rs:230` 就会把已分配的 Plain 容量从 moved_in 追认成许可。现在全部传 0，所以这些路径只会失败，不会被利用。
137: - **P2-2 material journal 事后同额预留**：`effects.rs:1547-1560`。`j.bytes` 是 Recording 期间已经分配的 journal 字节，之后才 `new_empty_with_fee(j.cap, j.bytes)` 从新父预留同额许可，正是不变量中禁止的「同额补签」（换了个名字）。
138: - **P2-3 rebirth 承认无票旧块存在，复制峰值少计**：`q.rs:548`、`history.rs:256-257`、`native.rs:801-861`。旧 Plain 块出生时没有许可；rebirth 期间新旧两块同时存活，只有新块计费，父账 peak 少计了旧块容量。
139: - **P2-4 r14 恒等式检不出池内重复计费**：`audit_owner.rs:3264-3306`。`sealed` 由析构差额定义，负控只测 helper（详见第 4 项）。主控要求的「无重复计费」在生产路径上没有被检验。
141: ### P3
143: - **P3-2 fixed_radix :781 成功插入峰值计入 M**：超出裁定字面，但符合裁定意图，建议主控追认。
```

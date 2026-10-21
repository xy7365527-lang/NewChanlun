## 结论：PASS-WITH-NITS

e96f60f884 把它声称处理的六项都如实落实了，没有改动任何生产行为，也没有引入新的编译问题。剩下的只是两处簿记不整齐（跟进票 #1635 的清单与本提交的 reason 字符串没完全对上）和一条取证日志的可读性问题。

### 逐项核对（声称处理的 nits）

| 项 | 核对结果 |
|---|---|
| A1 ForceL 锁 reason 指向票号 | `divergence.rs:1542/1598` 两处「已上报」改为「跟进见 #1635」；#1635 第 1 项正是这件事，内容（嵌套测试不被收集、`L(b)=-8` 与 -12 不符、搬出可能红）与评审 A 的描述一致。✓ |
| A2 `stage_token` 参数级 allow | `finance.rs:1570-1574` 从 `_stage_token` 改回 `stage_token` 并挂 `#[allow(unused_variables, reason = …)]`。函数参数上允许 lint 属性（RFC 2565，Rust 1.39 起稳定），`reason =` 字段 1.81 起稳定、仓内已有 298 处同样写法，远端十档组合编译通过也直接证明合法。函数体从头到尾没有读这个绑定，改名不改行为；全仓已无 `_stage_token` 残留。✓ |
| A3 s_session check 步骤补遥测 | `ci.yml:243-249` 补了 `free -b` 和 `du -sx target` 前后各一次，现在与另外两个 check 步骤逐行一致。✓ |
| A4 孤儿注释 | `coverage/mod.rs:150-154`：「#913 误加 test 门」注释移到 `TwStepCtx` 行上方，`VoiceVerdict` 另加一句说明只剩测试消费（shadow.rs 已是 cfg(test)），`t1_target_*` 块尾的旧注释删除。✓ |
| A7 fill.rs 文档 | `fill.rs:867` 改成「仅供事务落盘测试使用，生产循环走 `_historical`」。我核对了调用点：`_observed` 只有 `fill.rs:3560`（测试）一处调用，`_historical` 在 2998/5558 两处生产路径被调，文档与现状相符。✓ |
| B N2/N3 恒真 cfg_attr | `commit.rs:119` 和 `cross_request_source_cache.rs:19` 都改成无条件 `#[allow(dead_code, reason = …)]`，理由如实写明「旧包装入口、无调用方」。我核过：`publish_borrowed_with_workspace` 全仓只有定义和自身转发，`prepared.rs:320` 直接调 `_admitted`；`Binding::read` 同样零调用，三处调用都走 `read_admitted`。模块门 `v3/mod.rs:52-53` 仍在，所以 native 之外本来就不编译这段，行为不变。✓ |

### 远端实测日志的核验

`/tmp/nc-remote/20261021-103408-e96f60f884.log` 的结果行可信：C1–C10 十档（含 json+sqlite、native+sqlite、三者全开）全部 `rc=0 errors=0 warnings=0`，输出里有 `Checking newchan_rust`（23.68s）说明是真编译而非缓存重放；fmt 0 diff；两个 1394 负例 PASS；EXIT=0。这补上了评审 B 的 N5 组合缺口（macOS 路径除外，见下）。

### 发现（按严重度）

**1.（低）两处新 reason 写「去留见 #1635」，但 #1635 第 4 项的清单没有点名这两个函数。**
#1635 第 4 项列的是评审 B N1 的 21 处 w2 旧入口（control_audit / q_wire / history / q_snapshot / q_watch / q_content / source_audit / object_index / reader / request_audit / audit_store），`commit.rs` 的 `publish_borrowed_with_workspace` 和 `cross_request_source_cache.rs` 的 `Binding::read` 都不在其中。后续处理 #1635 的人按清单走会漏掉这两个。建议在 #1635 第 4 项追加这两处（一条评论即可，不必改代码）。

**2.（低）#1635 第 6 项仍写着「`fill.rs` 的 `step_center_oscillation_observed` 文档仍写供生产循环使用」，而本提交已经改掉了。**
票面比代码慢了一拍，建议在票上划掉这一子项，避免 owner 去找一个已经不存在的问题。

**3.（信息）远端日志的命令头被变量展开弄坏了，看不出实际执行的 cargo 命令行。**
日志第 1–3 行：`grep: in': No such file or directory`，以及 `cmd=` 头里 `out=    Checking newchan_rust…`、`echo C [] rc= errors= warnings=` 这种把运行时变量值回填进脚本文本的样子，说明包装器打印 cmd 时做了一次不该做的展开。结果行是循环体在远端实时生成的（`$f` 展开正确），所以结论可信；但「用了 `--all-targets`」「是否带 `-D warnings`」这两点在日志里读不到，只能信票面描述。建议包装器改成原样（单引号/heredoc）打印脚本，让日志自证。

### 转入 #1635 的项——是否转得合理

- A6（ledger.rs 四个插桩字段从未读出）→ 第 3 项 ✓
- B N1（21 处旧入口政策不一致）、N4（`audit_owned_capture_for_testonly` 等 cfg(test)+allow 孤儿）→ 第 4 项 ✓
- B N6（既有文件级 `#![allow(dead_code)]`）→ 第 5 项，并从 B 点名的 1 处扩成 8 处；我 grep 核过 v2 四个 content_* 文件和 v3 四个文件确实都带文件级 allow，清单准确 ✓
- B N5 中的 macOS 专属路径 → 第 7 项 ✓（Linux 组合缺口已由本次十档补齐，票面只剩 macOS，与实际一致）
- A3 主体（把 `RUSTFLAGS` 提到 job 级以避免 check/test 两套 build-script 产物）没有进 #1635。评审 A 自己给了「先合入、看该 job 的 `du -sx target` 读数再决定」这个选项，本提交补齐遥测正是为了让这个决定有数据，所以不转票可以接受；但它是 CI 成本项而非技术债，不进 debt 票也说得通。
- B N7（`mod.rs` ~1983 冗余的 `messages.get(id).ok_or_else`）未处理也未转票。B 原话是「可顺手删，或不动」，不动符合本票「不改行为」口径，无需追。

### 未发现的问题（对账用）

没有新增文件级 allow、没有 `let _ =`、没有 cfg 门控变动；本提交七个文件里唯一的代码语义变化是把一个从未被读的参数改了名，其余全是属性、注释、文档和 CI 遥测。

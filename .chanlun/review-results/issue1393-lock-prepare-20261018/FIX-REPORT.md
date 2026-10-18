# #1393 锁解析准备：两项独审阻断返修

本轮只完成锁入口的两项尾差修复，作者定向验证通过，**等待管理者安排新的独立评审**。没有生成真实 lock、执行 Rust/工具链/网络/远端任务，也没有获得 E 预发布或 F 单次 push 授权。`FIX-CONTRACT-DRAFT.json` 的 E、批准、独审仍为 `UNBOUND`，当前草案准确拒绝。

名分：绑定 #1393 的工作草稿。外部 Sandcastle Codex 工蜂，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`，管理者 `/root/manage_1393_sandcastle`；真实 session_id 由运行时事件登记，本报告不声称注册为原生子代理。按本次明确禁令，没有启动其它工蜂；作者自测不替代独审。

返修基线 HEAD 为 `a1a2aded058fc208217e8b701ec50f78b6b0aeeb`，tree 为 `9ba120542f45048bfda948e2acfc9dec7258f230`，分支为 `codex/1393-lock-prepare-20261018`。开始时 clean，结束保留未暂存源码差异和本目录新报告供管理者收包；HEAD 未变。

## 阻断依据与修复

已先读 `INDEPENDENT-REVIEW-01.json`。原 run `244bc3e8-85de-448d-aad7-842c618c74ff` 的进程 exit 为 0，业务审查为 blocked，不能据进程成功推断审查通过。原独审和全部 23 份既有报告、失败史逐字保留，未覆盖旧 REPORT、VALIDATION、CONTRACT、FILES 或旧静态检查脚本。

| 阻断 | 本轮落实 | 定向证据 |
|---|---|---|
| 最终 ZIP 摘要可过期仍 PACKAGED/PASS/return 0 | 新入口 `issue1393_lock_prepare.py:348` 起增加本票有界 `check_budget/file_digest/package/failed_package`，仅覆盖固定 helper 的私有实例 `v.package`；每块最多 1 MiB，读、update、hexdigest 前后核同一 Budget/信号，流关闭后再核。收据写后、封包返回后、最终 print 前后均复查，失败改写散件 PACKAGE/result 为 FAILED 并返回 1 | 11 个收尾边界 × monotonic/wall/SIGTERM 共 33 个负控；实际 main、Budget、Supervisor、ZIP、失败散件和返回码均验证；窗口内成功及多块摘要另验 |
| 未固定指定 source，允许 S=E 或其它祖先 | `issue1393_lock_prepare.py:107` 的 `bind_source` 先钉死 S=`8e50d40933c2a6a25f882481f3c46aa885ce6407`、tree=`0a2e04a4ce22a6b8732ad3b55f57446ee3f066f0`，拒绝 E=S，再执行原完整 Git identity、祖先/tree/写集/产品清单核验 | 合同与 Git 回复同步伪换的 S=E、其它祖先、错 tree 均在 Git 读取前拒绝；真实指定 S 的完整 identity toy 通过；固定声明但实际 Git tree 错误仍拒绝 |

最终 ZIP 保留封包时的旧结果快照；遇收尾失败不回写或删除该 ZIP，不把包内 PASS 当最终成功。最终散件 `PACKAGE.json` 为 FAILED 且无成功摘要，`result.json` 和最后输出为 FAILED。若输出过程中才耗尽窗口，日志可能先有旧 PASS 行，随后补最终 FAILED 行，实际返回 1。

固定 helper 文件及摘要 `a98e7e6f54dacef35476f245a1f938df6cd0a952191fd7ea67938dd7185b1830` 保持不变。仅通过 `git show 0d5af49549d5b3416360e694b09efdc70feb786c:.github/scripts/issue1393_validate.py` 只读借鉴必要收尾函数；没有读取另工位工作区、复制整份 runner、修改旧 helper，亦未将该工位尚未完成的独审算作本轮通过。

## 范围与不变约束

源码只改 `.github/scripts/issue1393_lock_prepare.py` 和 `.github/scripts/test_issue1393_lock_prepare.py`。旧入口已有函数的 AST 差异只有 `bind_source/main`；新增四个局部收尾函数。workflow 保持原字节及 False。

完整 Git 元数据核验表明，固定 S 到本轮 HEAD 的全部变化仍仅在本票三个入口和报告目录。不可变源清单仍为 8,375 项，SHA256 为 `e1059a227dc513426f1a201f02ad94b42c5dfad5d2299d96e88e6901c676785a`；不是用 sparse 可见文件代替全树。

900 秒共享总窗、90 秒预留、jobs=1、2 GiB 起始磁盘门、10 秒采样、进程回收、严格封套、False、固定 argv、隔离无凭据环境、最小真实源码运输、完整 lock 差异及禁止 rerun 均保持。没有改旧验证入口、CI、Cargo manifest/lock、Rust、Lean、产品或配置，没有暂存/commit/push/merge/dispatch/切分支/改 Git 配置/操作 tracker/读取凭据。

## 实际验证与失败史

- `python3 -B -m unittest discover -s .github/scripts -p test_issue1393_lock_prepare.py -v`：28 项通过，0.535 秒，退出 0。原 23 项保留并增强身份/草案断言，新增五项收尾测试。原件：`fix-toy-verified.stdout/stderr`。
- 新收尾矩阵的 11 个边界是 ZIP open/read/EOF/close、hash update/hexdigest、收据写前/后、封包返回、print 序列化及 print 输出后。每个边界分别推进真实 Budget 的 monotonic 或 wall 单轴越过 900 秒，或实际 `raise_signal(SIGTERM)` 走真实处理器；所有新场景 Popen 调用为零。多块内存 I/O 覆盖 2 MiB + 17 字节，四次读取均请求 1 MiB，包含 EOF；过期在打开文件或构造摘要前拒绝。
- `python3 -B .chanlun/review-results/issue1393-lock-prepare-20261018/fix_verify_static.py`：六组通过，退出 0；覆盖固定基线/完整写集、AST、23 份原件逐字保全、workflow YAML/内嵌 Python/bash -n、固定 helper/源实际 tree/全树清单、草案摘要及 UNBOUND 拒绝。原件：`fix-static-verified.stdout/stderr`。
- 修复前负控：27 项测试退出 1，33 个 failure、5 个 error，证明旧实现不满足新断言（包括错误放行、缺信号记录、非分块读取）；原件 `fix-toy-red.*` 保留。首次修复后 27 项通过，原件 `fix-toy-round01.*` 保留。
- 新静态脚本首轮已逐字对完原件，但把历史文件数量预期误写成 24，实际为 23；该计数断言导致退出 1。只订正本轮脚本的数量断言后通过，未改历史文件或源判据；首轮 `fix-static-round01.*` 保留。

没有重跑历史 `verify_static.py` 的旧 HEAD 断言或无关全量验证。本机 cargo/rustc/rustup、真实工具链安装、真实解析/网络/远端/上传、Rust/Lean/fixture/产品、Linux subreaper 实测均为 NOT_RUN。临时标准库 Python 子进程仅来自复用的两个既有监督器 toy。

## 交还管理者

`FIX-VALIDATION.json` 记录验证、范围和文件摘要；`FIX-CONTRACT-DRAFT.json` 已填固定 S/tree、产品清单及可核静态摘要，E/tree、批准与新独审原文/摘要仍为 UNBOUND，status 仍为 PREPARED_NOT_AUTHORIZED。该草案不授予任何运行权限。

管理者需固定本次真实尾差并交新工蜂独审，再补全 E/tree、批准与独审绑定，将完整运行合同递根；不必等待主验证工位整体结束。本报告不冒充新独审，也不批准后续 True、预发布或单次 push。

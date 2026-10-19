# #1406 C6a 离线绑定器返修复核

日期：2026-10-17。名分：绑定 #1406 / #1323 的独立审查工作草稿。

结论：**Approve，仅限本次离线绑定器三项返修声明域**。此前 `c6a-python-review.md` 的 1 HIGH / 2 MEDIUM 均已独立复验关闭；本轮范围未发现新增 CRITICAL/HIGH。**这不是完整 runner 合同签署，不授予构建、真实运行或 dispatch 准入；`CONTRACT_READY=False` 继续保留。**

## 精确审查对象

工作树 `/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1406-c6a`；源码只读。审查前后实测摘要一致：

| 文件 | SHA256 |
|---|---|
| `.github/scripts/run-c6a-remote.py` | `595da5c32a0a19f65d78b693b686634c1b5c43109e9a305129dcd68ada1fc83c` |
| `.github/scripts/test-run-c6a-remote.py` | `57bde7d936b4cb89114d30de0310259d3dad8e61b9db9afb4a687aeef7cb85c3` |
| `.github/workflows/ci-c6a.yml` | `120b1b05dc62d4cf888537c20138620e02bef28ee0d07840fa699ddbf6b00e57` |

已读取作者 `c6a-binder-fix.md`，并重新执行 `git diff -- '*.py'`。三个准备件仍为 untracked，因此 diff 空输出不作为无变更或通过证明；实际复核为文件内容与 SHA。

## 原缺陷关闭证据

1. **H3 token/source_event_time 漏绑：关闭。** `run-c6a-remote.py:146`、`:147` 将正式请求的 token/source_event_time 作为 after 原件预期；`:153` 将 Recorded token 对原请求。原独审“同时篡改 record token/time 和 receipt token，重新计算 image_hash/root 并对齐 cut”复现现被 `Capture 回执 token` 拒绝。定向回归另外分别覆盖 record token、receipt token 和 source_event_time，故不是先失败的 token 掩盖 time 漏洞。合法 envelope 摘要自洽的篡改负控也通过。
2. **JSON 指数溢出：关闭。** `:62-67` 的 parse_float 转换后检查有限性。原 `{"monotonic":1e9999}` 复现现被 `原件 JSON 非有限数值` 拒绝；有限小数正例、负溢出、NaN、正负 Infinity 与重复键负控通过。
3. **路径别名重复：关闭。** `:70-75` 要求输入等于规范 POSIX 相对路径；`:92` 在唯一性检查前完成验证。原仅有文件 a、清单同时登记 a 和 ./a 的复现现被 `原件路径不是规范 POSIX 相对路径` 拒绝。规范路径正例及 ./、中间 /./、重复斜杠、尾斜杠单条/混合清单负控通过。没有扩大为硬链接或并发替换保证。

## 本轮独立执行

从测试模块构造定向 unittest suite，选择全部 `CaptureBindingTests`、全部 `ExchangeTests`，以及以下 `BoundaryTests`：

- `test_json_finite_decimals_and_invalid_numbers`
- `test_inventory_path_aliases_rejected`
- `test_inventory_traversal_symlink_duplicate_and_tamper`
- `test_gate_stays_closed`

结果：**12 tests，0.012 秒，OK，exit 0**。覆盖修复面及 regular_file 对 wire 原件路径的直接影响，没有重复无关进程/期限测试。另独立执行原审三项最小复现，全部明确抛 ValueError；未只采信作者新增测试。

两份修改文件 AST 解析及行尾空白检查通过。`--preflight` 实测 exit 2，输出 `BLOCKED_PREPARATION`、`CONTRACT_READY=false`、`real_execution=false`。上轮已确认 ruff/mypy/pylint/black 不可用，本轮未安装或声称执行这些工具。

作者报告的 17+48 全量离线通过是作者证据；本轮仅独立执行上述 12 项必要回归及三项原反例。既有 48 driver 合同上轮已独立通过，driver 本次未改，不重复运行、不将计数写成自己本轮执行。

## 保留边界

没有运行 cargo、真实服务、HTTP、外盘、push 或 dispatch，没有修改产品或准备件源码。完整逐序 transport/owner/oracle、Q startup、可信 source/build/binary 取得与绑定、资源门、guardian/取消回收、失败封包以及旧冻结合同/原反例恢复仍在此前公开欠项中。本次修复通过不证明上述能力，也不构成 #1406 或 #1323 全图验收。

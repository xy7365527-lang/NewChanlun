# #1406 C6a 离线绑定器独审返修

日期：2026-10-17。名分：绑定 #1406 / #1323 的工作草稿。

本轮已实际修复独审提出的 1 HIGH / 2 MEDIUM，并完成离线回归；等待原独审复核。**CONTRACT_READY=False，仍是 BLOCKED_PREPARATION，不具备真实运行或 dispatch 资格。**

## 范围和依据

工作树 `/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1406-c6a`，基线 `b6e690ecae5723fbbcb773f15400c71d1010da87`。

只修改 `.github/scripts/run-c6a-remote.py`、`.github/scripts/test-run-c6a-remote.py`；workflow 只读且摘要保持不变。既有 driver、store、产品及 fixture 未修改。

已核正式字段：`s_session/tests/tb03c_query_runtime.py:675` 的 Capture 请求实际包含 token/source_event_time；`rust/src/economic_session/store.rs:1288`、`:1289` 的消息审计将二者与已存原件相等绑定；`:1327` 的 Recorded 回执实际包含 token。本次使用这些已有字段，不新增金融 schema 或 reducer。

## 返修

1. Capture 预期直接从原请求取 token/source_event_time，对 after 记录逐值比较；Recorded token 同样绑定原请求。正控从冻结 RAW fixture 取真实事件、token 与 source_event_time。新增分别篡改记录 token、回执 token、同时篡改二者，以及 source_event_time 置 null 的负控；重新生成 owner 图像摘要、cut 和合法 wire envelope，先用现有 decode_frame/validate_reply 验证 envelope 自洽，再确认绑定器拒绝。
2. JSON 的 parse_float 检查转换后 math.isfinite，堵住合法 JSON 指数文本 ±1e9999 转为 inf 的入口。有限小数正控、正负溢出、NaN/±Infinity、重复键拒绝均实际执行。
3. 清单路径必须是与 PurePosixPath.as_posix() 完全相同的规范相对路径，先验证路径再检查唯一性；不允许 ./、中间 /./、重复斜杠或尾斜杠。规范路径正控返回同一规范字符串集合；别名单条及与原路径组合的清单都拒绝。原越界、符号链接、重复条目、摘要/长度负控保留。

测试的 owner/cut 为离线单元测试构造，只声明字段绑定与自洽篡改拒绝，未声称已恢复完整经济图像、真实 root 计算、原消息生产者身份或真实进程证据。

## 实际验证

先只添加测试、未改绑定器：17 tests，14 个失败子例，exit 1；三个独审缺陷均实际复现误接受。

修复后：

```
python3 -B .github/scripts/test-run-c6a-remote.py
Ran 17 tests in 0.013s / OK / exit 0

python3 -B -m unittest discover -s s_session/tests -p test_tb03c_query_runtime_contract.py
Ran 48 tests in 1.545s / OK / exit 0

python3 -B .github/scripts/run-c6a-remote.py --preflight
BLOCKED_PREPARATION / CONTRACT_READY=false / real_execution=false / exit 2
```

两份修改 Python 文件 AST 和行尾空白检查通过。git diff --check 无输出，但三个准备件仍为 untracked，此检查不能覆盖它们；因此另做直接文本检查并列文件摘要。没有运行 cargo、产品 HTTP/服务、外盘操作、push、dispatch 或关闭票。

## 供复核的摘要

| 文件 | SHA256 |
|---|---|
| `.github/scripts/run-c6a-remote.py` | `595da5c32a0a19f65d78b693b686634c1b5c43109e9a305129dcd68ada1fc83c` |
| `.github/scripts/test-run-c6a-remote.py` | `57bde7d936b4cb89114d30de0310259d3dad8e61b9db9afb4a687aeef7cb85c3` |
| `.github/workflows/ci-c6a.yml`（未改） | `120b1b05dc62d4cf888537c20138620e02bef28ee0d07840fa699ddbf6b00e57` |

## 仍未完成

完整 runner 的逐序 transport/owner/oracle 回放、Q startup 原件检查、可信构建绑定、guardian/取消回收、资源门和失败封包仍未完成。旧资源数值及 guardian 原合同、旧独审反例原件仍未恢复；不能用本轮 65 项离线测试代替原合同、真实运行或全图验收。本次保持原时间常量、准入关闭和全部隔离边界。

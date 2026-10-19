# C6a 外层时钟锚原两项窄复审 v2

名分：绑定 #1406 / #1323 的独审工作草稿，2026-10-17。

**Approve，仅原两项返修范围。公开协议类型契约 HIGH 与 transcript 序号 MEDIUM 均已关闭，未发现本轮返修引入新的 HIGH。此结论不构成完整 C6a 验收，不打开真实运行门。** 源码只读；本工位仅新增本报告与独立 v2 测试，原报告/反例未改。

## 两项复核

1. **类型契约 HIGH 关闭。** runner 与 driver 新增了 WorkAnchor / CaseAnchor / QAnchor、请求与响应 TypedDict；runner 明确 transcript、pending、可空生命周期。ExternalClock.request 的三组 Literal overload 区分三种返回锚；时钟 Callable、socket、Budget 的新边界均有具体类型。JSON 入口为 object，先运行时核字段/类型/有限值，再 cast；没有用 Any 批量遮盖。独立 inspect + typing.get_type_hints 验证 **29 个协议入口**的参数与返回注解可解析，并核三组 overload。

   首轮返修的 `reject(error: Exception)` 与现存字符串失败来源不一致；已由主控改为 `Exception | str`。本工位验证最终 runner 到返修首版的**唯一字节差异就是这一处注解**：逆向替换后 SHA 精确等于 `1a7ee70a9528867de1e5021216fb34a421122a6000b0876634af2a9a9f837b2d`。分别输入字符串与 RuntimeError，失败记录均正确。未因类型修复扩大到旧 driver/监督器全文件改造。

2. **序号 MEDIUM 关闭。** verify_clock_transcript 先用 clock_mapping 确认 response 是 dict，随后强制 `type(response.get("seq")) is int`，再做身份与期限比较。原首包 true 反例如今明确抛 `clock response sequence type`。独立 v2 对全部五行分别注入 True、False、对应 float 序号，**15 个子例全部拒绝**；合法 int 正控仍返回精确三键锚。另核非 dict response、布尔/NaN/Inf 锚拒绝。

## 行为与实际验证

- 保守前移的 W 仍来自同一个 Popen 前 started 采样；600/630/300/10 以及五个外层绝对端点未变。修改没有新增 clock 采样或重试续租。新增空态检查只把原无效初始化路径显式拒绝，未放宽有效输入；case/q 名称仍只允许原 we5/we7。
- 作者监督器套件在返修首版 runner 上实际复跑 **53 项通过，0.030 秒**；driver **62 项通过，1.615 秒**。最后仅变 reject 注解，已精确比对字节，按主控要求复用这两套结果，不重复全套。
- 独立 `c6a-external-anchor-rereview-v2-repro.py` 在最终 runner 上 **17 项通过，0.017 秒**。保留旧 5 项上传/竞态回退正反控及原 10 项时钟/取消/FD/嵌套 probe 正反控；旧“bool 被接受”见证改为拒绝负控，另增非 dict/非有限值反控。原 16 项文件 SHA 保持不变。
- `git diff --check` 通过；最终 runner `--preflight` 实际返回 **exit 2 / BLOCKED_PREPARATION**。CONTRACT_READY=false、SEMANTICS_READY=false，source/tree=PENDING，资源原值不变。
- ruff、mypy、pyright、black、pylint 当前 PATH 不可用；未安装工具。注解可解析不等于完整静态类型检查通过，报告不作该声明。
- 无真实产品、socket、HTTP、信号、cargo/rustc、外盘、重型验证、远端或浏览器操作；全部进程与网络行为为纯替身。

## 精确字节

| 文件 | SHA256 |
|---|---|
| 最终 runner `.github/scripts/run-1406-c6a.py` | `3a661d69a43ddaad19e353ca2d3abd35696b05050a1338d846ee5bd1769a72d7` |
| driver `s_session/tests/tb03c_query_runtime.py` | `04e234f278e5adfdba030369e5aac69c606b2a8ae4ee8f296c656b6894e67187` |
| driver contract tests | `a6cf4dc257f040a4b179346c0f5fe676049deb59798ca45485fe908b976ddf4d` |
| 作者监督器 tests | `0404b4251a6a1872234b92c77e6e3a744f47d0cbe46270ffd8b7928cb10f5059` |
| 原独审反例，保持不变 | `07d957ae478dd97205e3e6eee9ebe805bcf69faee3754d907cde468fd72d8748` |
| 本次独立 v2 反例 | `dc931350e9f89ec138463163c4844bb7427f758e875759215915518802daa3b8` |

工作树 `.sandcastle/worktrees/resume-1406-c6a`。仍待新 source/tree 冻结、语义器真实嵌入、全 41 步合法 wire 正控、真实 Linux FD/进程组回收、两文件发行尾审及远端交付验收；不关闭 C6a、#1323 或 #1279。

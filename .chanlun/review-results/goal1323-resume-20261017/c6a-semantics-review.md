# #1406 C6a 离线语义器独立审查

日期：2026-10-17。名分：绑定 #1406 / #1323 的工作草稿。

**结论：Block。发现两项 HIGH。现有 10 项纯数据测试全部通过，但不足以解除以下阻断；不签全 41 步、真实执行或完整 C6a。**

## 范围与身份

审查文件为 `c6a-semantics/semantics.py`、`test_semantics.py`、`REPORT.md`，并对照冻结 driver、协议及原 `c6a-startup-review.md`。没有修改作者源码、两个发行文件或已有 MANIFEST。

- semantics.py SHA256：`4067195b30fb8f6e20463cea20b324de1129e2b50a14f55cc861a86ef80ed2bb`
- test_semantics.py SHA256：`2ca52c891ea52c8e04989ff34def553c733efd76d7b09b88d9111549d0d651c3`
- 已读根 CLAUDE.md、项目共同规则，并调用 Serena initial_instructions。主仓 `git diff -- '*.py'` 无输出；本次对象是具名未跟踪候选，直接精读。
- 已独立运行 `python3 -B .../c6a-semantics/test_semantics.py`：10 测试通过，8.544 秒。
- ruff、mypy、pylint、black 在 PATH 和 Python 模块路径均不可用；没有安装或把静态检查冒称通过。
- 额外纯数据反例在同目录 `c6a-semantics-review-repro.py`；运行约 0.06 秒。只使用临时文件、替身进程账和 import，不启动产品、HTTP、信号、cargo 或外盘访问。

## [HIGH] Q 起点由待验 transport 反取，能掩盖超过十秒的启动

File: `c6a-semantics/semantics.py:294`、`:299`、`:302`、`:307`

Issue: `shared` 直接取首份 `startup_ready_attempt`，再把它的 `started_at`、`case_deadline`、`work_deadline` 作为原绑定器的“可信”参数。额外区间条件仅要求最后 owner spawn ≤ started_at ≤ Q spawn；它不能区分该区间中真正的 `Case.start` 起点与改晚的起点。真实 driver 在 `tb03c_query_runtime.py:589` 先取 started_at，随后调用 `supervisor.spawn`，所以 Popen/写 spawn 原件之前的耗时必须算进原十秒。将起点推到 spawn 日志时间会丢掉这段耗时。

已执行反例（并非仅泛称自报）：

1. 用现有纯数据 ProducerCase、真实原件形状生成 Q 启动；仅替身 spawn 在 `we5-q` 处模拟延迟 20 秒。
2. 固定进程账与外层 driver 区间：真实 started_at=`0.11500000000000002`，Q spawn=`20.116`，validated_at=`20.119000000000003`。实际耗时 20.004 秒。
3. 原 proof/transport 由同一生成器给出真实起点和十秒 deadline；新 `ReplayCase.start` 正确拒绝“Q startup 晚到或非法核验时间”。
4. **保持进程账、外层起止、请求/响应原字节、manifest、端口、首末序号均不变**，仅把 proof 与 transport 的 started_at/deadline 自洽改为 `20.116` / `30.116`。
5. 新 `ReplayCase.start` 接受；旧绑定器收到独立保留的真实 `0.115…` 起点仍拒绝“Q startup 可信锚 started_at”。全清单只是文件 hash，重新封包不会补回缺失的起点约束。

这证明该调用路径削弱了原已审合同。`REPORT.md` 明示“非外部精确 case t0”可以如实描述局限，但不能据此签原十秒不续租已核。即使执行源码可信，审查范围所要求的自洽重算篡改抵抗仍有这个明确缺口。

反例只断言 Q 启动阶段被错误接受；没有声称整包 `verify_runtime` 已通过。完整合成包仍受已登记 WE7 容量边界阻断。

Fix: 外层取得并冻结真实启动/期限锚，在核验 API 中显式传递，不能由待验 proof/transport 生成同名参数。如果外层观察不到真实起点，应改采集/监督接口留下独立锚，或将该项保留未验收；不要以 Q spawn 上界替代原始起点。增加“Popen 延迟超过十秒、自洽改晚 proof/transport、独立进程账不变仍必须拒绝”的反例锁。

## [HIGH] 依赖加载未绑定被执行字节，缓存串线且 oracle 在验 hash 前执行

File: `c6a-semantics/semantics.py:52`–`:65`、`:337`、`:367`

Issue: `load_driver` 先验 11 源字节，再 `exec_module(driver)`；driver 顶层普通 import `s_socket_client`、`tb03c_oracle`。前者可能来自 `sys.modules` 旧对象；后者不在 11 源集合，其固定 hash 要到后续 `driver.load_inputs()` 才检查。当前 `__file__` 相等既不能证明内存对象来自当前已验字节，也不能避免在 hash 检查前执行已改变的 oracle。

两个独立反例均已执行：

- 保持所有磁盘文件和 11 源 SHA 不变，将已加载 `s_socket_client.validate_reply` 替换为纯函数标记。再次 `load_driver(ROOT, SOURCE)` 成功，返回 driver 的 `validate_reply is sentinel` 为 True。它通过了现有 `__file__` 检查；换言之，核验可能复用外层先前修改/旧加载的校验函数。独立 `exec` namespace 不隔离 `sys.modules` 或 `sys.path`。
- 在临时 checkout 仅复制同一 11 源，将未在该集合的 `tb03c_oracle.py` 换成只写临时标记的无害模块；清理这两个缓存槽后调用 `load_driver`。所有 11 源 hash 匹配，导入来源路径检查通过，**未经固定 oracle hash 检查的代码已执行并写出标记**。后续 `load_inputs` 才可能拒绝，但拒绝无法撤销已执行行为。没有修改真实 checkout。

`load_driver` 还永久向全局 `sys.path` 增加入口，进一步削弱“可嵌入独立 namespace”的隔离声明。这不是要求抵御一个已经控制整个解释器的攻击者：同进程旧版本缓存/外层测试 patch 已足以导致“磁盘验 A，实际执行 B”。

Fix: 在执行任何依赖前验证完整可执行依赖集合（含冻结 oracle）的字节；从已验证的同一份字节实例化模块，不复用普通全局 import 缓存。隔离 import 命名空间并恢复全局状态，或在外层监督的干净 helper 中加载；仅换唯一 driver 模块名不够。补同路径缓存对象已改变和 frozen oracle 字节不匹配的拒绝锁，后者必须在任何 oracle 顶层代码执行前拒绝。

## 其他已核范围与保留项

- `verify_runtime` 接收外层 provenance，检查构建六键、11 源集合、运行前后二进制摘要相等；外层仍负责这些值的真实来源和构建/执行监督。
- 20+21 主请求从冻结输入独立枚举；owner 请求原字节、连续 transport、精确重送、before/after、公共 current/fixed/watch/as-known/旧 token 和保存的 oracle 逐步回读。本次未找到这些路径另一个高置信新缺陷。
- H3 首次 Capture 的原 raw/pointer/event/hash/token/source_event_time 和 Recorded cut 绑定存在，原测试中的重算 envelope/hash 篡改被拒。
- 普通原件清单完整性、具名进程 argv/日志、CLI exit 0、TERM/回收记录检查存在；真实未 reap leader、Linux subreaper/WNOWAIT、资源/取消/硬截止仍归外层，不因本报告而通过。
- **WE5 20 步正控通过；全 41 步合成 WE7 12 响应超限是作者已登记的证据边界，不作为本报告第三项新发现。** 没有放宽帧上限，没有运行真实系统，不可签全 41 步成功。

上述两项修复后应按精确新 hash 复审；原 10 测试全绿不抵消反例。

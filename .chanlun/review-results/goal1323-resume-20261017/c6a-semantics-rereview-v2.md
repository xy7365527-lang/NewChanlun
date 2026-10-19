# #1406 C6a 语义器两项 HIGH 返修独立复审

日期：2026-10-17。名分：绑定 #1406 / #1323 的工作草稿。

**结论：Approve，仅针对原 `c6a-semantics-review.md` 两项 HIGH 的离线返修。未发现本次修改中另一个高置信 CRITICAL/HIGH。真实外层尚未提供独立时间锚，全 41 步合法 wire 正控仍缺，C6a 真实运行资格保持关闭。**

## 精确身份与操作范围

- `c6a-semantics/semantics.py` SHA256：`731fffd517420ab4c749bae2fe6987b25e7ee3722365ae9bc27551c92533208d`。
- `c6a-semantics/test_semantics.py` SHA256：`5558f42ee4ef5e30e9d52b847f63fa2b715ec71b85d719f7c6daf774151aad12`。
- 独立复验脚本 `c6a-semantics-rereview-v2-repro.py` SHA256：`27e54175796609cebd9af21b702b38a97fa86dd9ff19323419f8e82211745789`。
- 审查前后候选 hash 相同。只新增本报告及独立纯数据复验脚本，未修改作者源码、发行文件、driver 或准入开关。
- 已读根 CLAUDE.md，调用 Serena initial_instructions。主仓 `git diff -- '*.py'` 无输出；本轮审查对象是具名未跟踪候选，直接精读并对照冻结 driver 和两份 oracle。
- ruff、mypy、pylint、black 在 PATH 与当前 Python 模块路径都不可用，未安装，未声称这些工具检查通过。
- 未运行 cargo/rustc、真实产品、HTTP、外盘访问或重型回放。

## 原 HIGH 1：缺独立启动锚／重基起点

修复位置：`semantics.py:258`、`:406`、`:415`、`:440`。

`ReplayCase` 必须取得外层 provenance 中的 `case_startup_anchors[name]`；精确三字段为 started_at、case_deadline、work_deadline，值须为有限数。`verify_runtime` 的 required 集合同样要求该入口。原绑定器的可信参数直接取这些独立锚，proof/transport 不能再给自己提供起点和期限。已有进程账上下界继续作为交叉检查，未被冒充精确起点。

独立脚本保留原反例结构：仅替身 Q spawn 延迟 20 秒，真实 started_at 为 `0.11500000000000002`，spawn 为 `20.116`，validated_at 为 `20.119000000000003`，实际启动耗时 `20.004000000000005` 秒。

1. 删除独立锚，直接构造 ReplayCase：拒绝“缺独立 Q startup 锚”。
2. 保留原迟到原件：拒绝“晚到或非法核验时间”。
3. 保持独立锚和进程账不动，仅同步改晚 proof/transport 起点及十秒 deadline：拒绝“可信锚 started_at”。

因此原同一进程账下的重基反例已不能通过。这里证明的是核验接口正确消费可信锚，不是证明外层已经能采集可信锚。独立锚的来源、采集时机和不续租握手仍须由后续装配证明。

## 原 HIGH 2：执行前冻结与模块缓存串线

修复位置：`semantics.py:58`–`:168`。

- 执行 driver 前，核精确 11 源集合和所有 INPUT_HASHES；源码与输入原字节留在 buffers，后续代码执行使用同一份已核字节。
- driver、socket 协议模块、两份 oracle 的 import 都先经过 AST 检查。未冻结的非标准库递归依赖被拒绝，不会先执行其父模块顶层再发现。
- 三个本地 import 名称走私有模块图；动态 `spec_from_file_location` 只允许冻结 executable 集合，动态 oracle 同样由已核 bytes 执行。
- FrozenPath 供现有 driver/oracle 的 Path.open/read_bytes/read_text 读取同一份冻结数据；原对象的 `__file__` 路径保留，现有相对 fixture 定位正常。
- `sys.path`/本地模块缓存不被该加载器改写；没有复用同路径全局 socket/oracle 对象。

作者测试中，静态及动态 oracle 被替成写临时标记的模块后，都在执行前摘要检查拒绝，标记不存在；新增未冻结递归 import 的父模块顶层标记同样未执行。独审重跑了这些测试。

独立复验进一步同时向全局 `s_socket_client`、`tb03c_oracle`、`c6a_we7_oracle` 三个槽放入相同伪模块，私有 driver 仍得到自己的 validator/oracle，sys.path 不变。随后在临时 checkout 将全部冻结源码、输入文件清空：已经加载的私有 driver 的 `load_inputs()` 与 20+21 期望步骤仍与改盘前完全相同；重新 `load_driver` 则立即因摘要不同拒绝。这覆盖了现有静态、动态 oracle 和输入读取路径的“验 A 执行 B”问题。

边界：这是对当前精确冻结源及它们实际 import/读取方式的绑定器审查，不是通用 Python 安全沙箱认证。当前实现信任解释器和标准库；未来来源集合、导入方式或文件读取 API 变更仍须重新审查，不能从本次结论推导任意代码均可安全装载。

## 已运行验证与保留门

1. `python3 -B .../c6a-semantics/test_semantics.py`：14 项全部通过，9.722 秒。包含完整 WE5 20 步正控、缺锚、原迟到重基反例、缓存隔离、oracle 先验哈希与递归 import 拒绝等。
2. `python3 -B .../c6a-semantics-rereview-v2-repro.py`：独立缺锚、迟到、重基、三缓存槽隔离、全部冻结源/数据改盘后绑定、重载拒绝全部通过，工具调用约 0.3 秒。
3. 全 41 步合成数据仍按原帧界拒绝 WE7 超限，这是负控通过；不等于全 41 步成功，也不等于真实产品必然超限。

未签：真实外层锚装配、完整 41 步合法正控、发行 runner 接线、资源/取消/截止/回收最终验收、远端产品运行、完整 C6、#1323 全图。没有解除任何资金、隔离、恢复或验收要求。

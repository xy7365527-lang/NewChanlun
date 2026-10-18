# #1527 Python H1 R3 第二次实际返修

**本轮返修与定向验证完成，候选等待管理者冻结独审。** H1 的两个合法 JSON 小数反例已由实际控制成功、随后严格写拒绝覆盖；有效锚写入仍可达。M1/M2 继承固定 R2 的 **PASS_SCOPED**，本轮仅复测受影响的小范围。此交付不关闭整票、不授予合入或正式运行资格。

名分：#1527 工作草稿。管理者 `/root/manage_1527_sandcastle`，父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`。本工位是 Sandcastle 外部 Codex 工蜂；真实 session_id 由运行时事件登记，不声称为父线程原生子代理。

## 固定来源与差异

固定 HEAD 为 `52a86c684768b884aec24a3f794a89a4fe8a3402`，tree 为 `165ffdc207cc80dcdd052e718d4609052e4ad0e3`，与父目录 `PYTHON-H1-R3-BASE.json` 一致。它包含原 `44a977568948542aa6d00ecab50e2d468e20b76c` 产品和管理者后补的报告/归档尾部。产品修改前的三文件 SHA 与 R2 独审记录逐一相等。

独审来源固定为 `e395c60180501fcc639e80ad24a8f955e75b3d8d:.chanlun/review-results/issue1527-python-r4-r2-review-20261018/REVIEW.md`，原结论为 BLOCK、需求轴 0 C / 1 H / 0 M；M1/M2 PASS_SCOPED。未读取其他工位的产品 WIP，未重新做 XERJ 查询。

仅改 `s_session/recovery_admission.py` 和 `s_session/tests/issue1527_recovery/test_python_r4.py`。`s_service_control.py` 及所有旧测试函数保持逐字一致。精确两文件补丁为 `python-h1-r3-repair.patch`，SHA-256：`506946a8ce1812c2350ff328db3afb51ce19bdb6cfd8c151c4acd0ae8a181bac`。

| 文件 | 本轮最终 SHA-256 |
|---|---|
| recovery_admission.py | `bfab73da3c05e169929cbd6e88b2f2122928dd63a215f064eedf1fbe36e170ef` |
| s_service_control.py（未改） | `82d7fd46759a2f5371d32c9f4ded2642b40f8e3a638e42b58e64b0b5b26b05c0` |
| test_python_r4.py | `dc8859b54f9909e232f8542ffc1c11179b906df8b2cc92790728087abc695b69` |

## H1 修法及合同边界

固定基线的 `require_owner → _object → parse_float=_reject_number` 在检查控制权限时先拒绝 admission 内小数。新增 `_owner_control_object`（产品 `:62`）只提供控制顶层解码；`require_owner`（`:164`）消费这个结果，且不再缓存其中的 admission 为写 binding。`prepare`（`:190`）必须对已固定的 `self._owner` 原字节调用未修改的严格 `_object`，再走原 `_validate_binding` 与完整磁盘锚交叉核。

| 输入或约束 | 控制读取 | 写入口 |
|---|---|---|
| UTF-8、JSON 语法、根必须是对象 | 拒绝非法编码、非法语法和数组/标量根 | 同样严格 |
| owner 顶层字段 | 只接受原 owner2/owner3 字段集；顶层重复键（含转义同名）拒绝；db/state_dir 仍须等于实际路径 | 复用已核的控制权限，再严格解析原字节 |
| admission 小数/指数/超长整数词元 | 仅保留 JSON 数字词元，不做写类型转换 | 由原严格解析或完整 binding 类型检查拒绝 |
| admission 内嵌套重复字段 | 保留对象键值对，不把嵌套写 schema 提前套到控制权限 | 严格 `_unique` 在任意深度拒绝；测试含完整合法 binding 中重复 mode、转义重复 mode |
| 非标准 NaN / Infinity / -Infinity | 仍拒绝，不视为合法 JSON 数值 | 仍拒绝 |
| 文件来源与连续性 | 原普通文件、uid、单链接、无跟随、权限、64KiB 有界读取及固定 FD/DB 复查保持 | 同一原 FD、原字节、原累计、原 t0、原启动绝对 deadline；不采纳替身 |

所有新控制夹具 owner 均为 0600。基线原有 owner 的 0600/旧版 0644 兼容许可保持；没有修改 `_read` 或其他文件的权限合同。数字不做浮点转换，因此合法 `1e400` 不会在控制路径先溢出。根对象用 tuple 键值对与 JSON 数组区分；只有顶层执行唯一键检查。该中间结果不提供写准入资格。

`_object`、`_parse_env`、`_binding_shape`、`_validate_binding`、首错处理、计量、原件复查、可选预算、publish/fsync 等 29 个原函数/方法逐字未改。M2 所在 `s_service_control.py` 整文件未改。`prepare` 仍先 E0 解析 env，再 owner/严格写校验；半组 env 的首次 E0/0 bytes、失败锁存和已加载快路前的首错检查保持。

## 实际执行与失败记录

命令、各次源码 SHA、返回码、耗时、stdout/stderr 路径和 SHA 全在 `CHECKS.json`。以下按具名执行报告，不把组、变体和单元测试累加成总数。

| 执行标签 | 结果与范围 |
|---|---|
| `h1-fixed-baseline` | 首次外部 H1 夹具失败，rc=1。临时路径经 macOS `/var` 别名而非 canonical `/private/var`，四个对照额外报 Identity；两个小数仍报 Invalid。失败日志完整保留，不计为成功复现矩阵。 |
| `h1-fixed-baseline-resolved` | 仅将夹具根改为 `.resolve()`；固定原产品不动。四个对照通过，float_mode/float_payload 精确复现 Invalid/E1。rc=0 表示预期反例被捕获，不代表旧产品合格。 |
| `h1-cli-red` | 修产品前运行新增真实 CLI 用例：两个变体 × status/stop-s/start-q 的六个独立新 Controller 全部 rc=1、Invalid/E1，测试失败如预期。保留完整红灯日志。 |
| `h1-r3-green` | 初版六项新 H1 测试全部通过。 |
| `h1-fixed-probe-candidate` | 同一固定 R2 `probe_review.py::h1_control_matrix` 仅 H1 一组在新产品上运行，六变体全部 PASS。源脚本 SHA 为 `c56e106c3f0cafdd9e30a89d1e111c5596d39f9c93d74c78b7aed3a09720a8d8`。没有运行其另外七组。 |
| `affected-write-cache` | 六项既有小回归通过：半组 env E0/0/锁存；可选文件权限/内容/部分读失败的首错锁存；原动作异常/收据/t0；双父 fsync 失败后原对象续发；identity 整数类型；延迟写准备原时限。 |
| `h1-r3-final` | 测试精化后六项 H1 全通过：嵌套重复负控换为完整 binding；超长整数允许在解码器或 binding 类型检查处拒绝，避免依赖 Python 的整数位数配置。生产代码与上一轮相同。 |
| `static-final` | 语法 AST、差异空白、精确 patch、独占写集、未改函数/测试、旧 16 日志哈希、旧固定源缺件事实、索引未改等全部通过。 |

最终 H1 六项分别锁住：

1. 两种小数写子树下，实际 CLI status 返回两个 running 并观察两次；stop-s 复核身份后仅向 stub PID 发一次模拟 TERM、三次观察；start-q 调一次 stub 启动并返回 ready。每次均为独立新 Controller，同实例随后 start_s 严格 Invalid/E1；再 recover/start_s/publish 返回同一首错，不重开/重读、不改 FD 集、累计、原 t0/deadline；close 后保留 FD 均失效。
2. 嵌套重复键、指数、小数和超长整数的控制/写分界。
3. 顶层重复、非法 UTF-8/JSON、NaN/Infinity、数组/标量根、字段多余、错误 db/state_dir 在观察前拒绝。
4. 控制读取后 owner 被换为有效新 inode，status 或写准备均 Changed，不读取替身。
5. 控制读取后达到原启动 deadline，写动作 Deadline 并锁存，不续期。
6. 完整有效的 published 与 pending 锚在控制读取后均能实际 loader/publish/fsync 并进入 stub 写启动；严格重解原 owner 而不重读 owner，wire 只增加实际 binding/limits 读取，保留原 FD、原 t0、原期限及 DB 哈希。

所有夹具只用临时 SQLite 和 stub；真实 ps/kill/正式 S/Q 均未执行。没有 Rust/cargo/build、旧 binary、历史重型、网络/安装/索引、凭据/.env、子工蜂、commit/push/merge/切分支/改 Git 配置/认领或评论。

## 继承与交接

R2 的 M1/M2 PASS_SCOPED 及未改面的已有证据继续继承；上述小回归不包装成 M1/M2 全量重审。原 42/4/8、旧八组均未全重跑，也不与本轮计数相加。后一次测试精化只改新增 H1 方法；既有六项回归方法逐字未改，其验证继续复用。

原 `44a977...` 固定对象确实没有作者当时清单所列 16 份 stdout/stderr，本轮用固定树清单再次确认该事实。管理者在 `52a86c6...` 按原 hash 补入的 16 份日志均存在且匹配 `PYTHON-H1-R3-BASE.json`，未改写旧报告。归档补核和其他所有 tracked 原件不在本轮 diff。

本目录新增 **16 份** stdout/stderr（与上述原 16 份不同），包括两次 rc=1 的完整输出；全部逐项列在 `MANIFEST.json` 的 `artifacts` 和 `force_add_paths`。日志被 `.gitignore` 忽略，交管理者只对这些具名实体 force-add；本工位未操作索引。清单列本轮全部实体与 SHA（清单自身不自哈希），产品 patch 可直接复核。

**仍 OPEN / NOT_RUN**：完整 9011、64GiB/store、148GiB 卷独立双跑、I-2/I-3/I-4/I-5、完整资源归属/共享 extent 峰值、源 RW 与 SQLite pathname 连续性、硬 I/O/掉电/EINTR/取消/真实信号回收、Linux renameat2、49GB 历史资格、同核金融隔离及恢复双跑、全部六目标。首次新库 prepare 和既存无锚接管未获准加入。#1405/#1420 生产 v4 仍 **BLOCK_NOT_MERGEABLE**。等待管理者冻结本候选并安排独审。

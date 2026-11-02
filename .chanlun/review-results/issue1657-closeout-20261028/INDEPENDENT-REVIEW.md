# #1657 非作者独审：PASS

候选 `507c3fed75ddc2538578999a85a1e3f259dcedc4`；基线 `3ddd2323c47e0658eba56f8205027a573e9f1f2e`。审查者 `/root/review1657_final`，非作者。结论覆盖本票签撤、合同门及文档切片，不代表 #1675 正式成功收据或 main 合入批准。

## 合同与实现核验

- `admission_issuance.rs:24–48`：版本、规范摘要、AwaitingMeasurement、窗口共用一个 authorize；生产注册表编译内固定，测试合同只在 cfg(test) 模块构造。七项未测值没有备用值；当前实际 CLI 拒签。
- `Site::open/check`：复用现有锁实现，state.control → DB.control → writer → maintenance；核进程/文件 uid、mode、nlink、目录和 DB FD 身份。具体库身份留 R4，不新增部署 inode 白名单。
- `prepare`：合同先验、published/pending/revoking 拒绝；0600 控制文件、0700 pending，fsync 后 owner 原子写及 no-replace 发布。不改 DB。失败保留 pending，显式撤销处理。
- `revoke`：持久 intent、原件目录移动留档、owner 去 admission 后完成归档；同 id 可续作已持久阶段，异 id/身份变更/归档撞名拒绝。intent 完整落盘前 I/O 失败不自动修复，ADR0049已明确保全原件人工处置边界。
- `s_service_control.py:201–218,372–393`：Python 原件核验后、publish/Rust R4 前调用同一二进制合同门；使用已核摘要和原绝对期限，校回执并重验原件。拒绝不发布、不启动；status/stop 不新增合同门。
- `recovery_admission.py` 无改动；R4 消费代码仅新增 issuance 模块指针。ADR0039 相对 origin/main 仅新增本票三入口登记；ADR0049编号不占已有0047/48。
- 已对照 #1657/#1567 票面、origin/main ADR0039/0042/0046及1567-contract-values.md。八项值、S86/MIN12、倍率2、一遍审计、窗口让路保持；未来有效版本的适用库辨识归 #1675，当前没有用文件名代替资格。

## 验证证据

- 独立实跑：`PYTHONDONTWRITEBYTECODE=1 python3 -B -m unittest discover -s s_session/tests -p 'test_tb01c_admission_contract.py' -v`，5/5，1.612秒，退出0。
- 复核 owner 原件 `rust-final-release.log`：真实 Windows WSL，14/14、0失败；九具名负控确实列出并执行；release 构建成功、无 test_prepare 符号、实际未测合同拒签；总命令退出0。
- 复核 `python-tests.log`：相邻82/82。旧生命周期测试显式隔离合同门，专用5测试实际经过 Controller/子进程并核拒绝不publish、原JSON不变和控制锁可重取，不把隔离夹具冒充合同正控。
- 已读全部新增Rust测试与Python测试，正控使用test-only Contract，生产注册表无注入口。未发现删除原断言或仅编译空通过。5a06675c88→最终候选仅文档，代码验证仍适用。
- CI workflow 实跑列表与负控、相邻Python及release门，未用 --no-run；当前远端CI状态仍由主控收齐，不由本报告冒称全绿。

注释维度已并入：新增约束/公开合同说明保留；无新增 lint suppression，无 MUST KILL 阻断。未运行 macOS Rust，未做全史重放。

## 交付门

主控收最终PR CI并按 main 批准门推进；合入顺序 #1527 → #1657。正式签锚和成功收据仍等 #1675 实测、适用库辨识和审定新版本。本报告未授权签锚或部署。

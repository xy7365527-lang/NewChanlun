# 根已定的必要验证方向（2026-10-18）

本文件记录根在本票验证准备中给定的工程方向，不是运行准入。产品 `baf7f0d8ef90943f575e33ad4f5ee30b065b74db` 已获第二独立会话 PASS_LIMITED_STATIC，Rust/Lean/真实链仍NOT_RUN；CI工位未合源码待独审。

- lib使用默认features域，列名与执行一致；不为本票引backtest_bin。S使用s_session。
- 每目标只编译一次，从cargo JSON绑定本次实际binary；再list核完整测试名非零且唯一，原样逐项exact，不以零匹配exit0过关。
- Lean单文件与fixture纳入同一具名远端验证任务，共享复用现有rust-check的总60分钟剩余截止，不为每一步另给60分钟。清理/上传保留实际失败；jobs1/10秒telemetry/2GiB起始门保持，不称OS硬帽。
- 现有CI语义不缩不删；本票允许新增专用Falseworkflow/runner，现有fixture job须显式持续检查新增Lean单文件。新增入口、False身份/合同及所有字节独审后交根准入；不得启True/push/dispatch。
- 当前本机重型禁令不解除；fixture脚本可能自动lake build，本机不得为准备验证运行它。

后续返修、CI接线或#1419/#1488组合改变源码时重新绑定source/tree/hash；不得拿本次有限静态结论证明另一源已运行。VALIDATION-CONTRACT-DRAFT仍为先前草案发生史，其中features候选以本方向明确收口，最终False合同由实际runner生成。

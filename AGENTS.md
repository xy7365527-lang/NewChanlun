# NewChanlun

用简体中文回复和写文档。主线为 `main`；产品路线见 `docs/ROADMAP.md`。

## 开发流程

- 执行链：pstack 提供工作方法，Wayfinder 管票与图，Sandcastle 提供隔离执行，LangChain Deep Agents 驱动执行 loop。入口见 `.agents/skills/`、`.sandcastle/README.md`、`.sandcastle/docs/graph-engineering.md`、`.sandcastle/deepagents-pilot/README.md`。
- 复用已有票；新开票前查 tracker 的全部状态及仓内相关材料。明确每项工作的唯一负责人和写入范围，不覆盖别人的改动。
- T、P 与 spec 都经过起草、非作者独审，再由代理裁定；SPEC 使用绑定正文的结构化独审裁定记录；不等待人工批准或 `/graph-approve`。审查 BLOCK 必须返修或说明未解决问题，不能撤标签冒充通过。
- 合入前须非作者独审和适用 CI 通过，并遵守当前任务的合入授权；Deep Agents 生产链使用异家族独审与 `deepagents/review`。审查依据候选补丁和有效上下文，显式重审不得被历史 PASS 覆盖。
- 实施、审查和发布沿现有入口；恢复沿同一任务身份对账，不重复派工或提交。不要另造批准门、状态板或平行调度器。

## 运行边界

- macOS 本机不运行 `cargo` 或 `rustc`。Rust 验证使用现有 CI、Boat 或已授权的远端环境。保留行为正确性，按实际差异做必要验证。
- 不读取、复制、记录或输出密钥。认证使用现有受管入口；不伪造签名、权限或审查状态。不把开发流程授权扩大为交易或账户授权。
- 变更前检查当前目录、分支、未提交工作和在途任务。部署前准备可独立使用的恢复路径，保护主图开发和生产账本。
- 大型构建、快照、临时文件和过程证据放 `/Volumes/AgentStorage`，不入仓。子任务结束、提交已推远端且无未保存工作或活跃进程后，用 `git worktree remove` 回收工位。
- 仓库保留产品源码、产品文档、测试输入及三件套实际消费的配置。迁走报告前检查消费者；真实 fixture 先迁入产品测试目录并同步调用者，不能直接删除。

## 产品语义与依据

- 词汇见 `CONTEXT.md`，架构决策见 `docs/adr/`，缠论教义正本见 `.chanlun/definitions/`。教义裁定须落正本并列出受影响的 Rust、Lean、Python 实现。
- 缠论语义以 `docs/chanlun/text/blog/` 原文为最高依据，其次为 chan99，再次为思维导图。引用须核原文、作者归属与行内注；历史研究文档不能直接充当裁定依据。
- 原文管语义，`formal/Origin/` 管原文未定的形式化边界。生产代码不能以现状证明定义，但实测反例可以否决错误裁定。
- 同一判断不得有宽松实现接管严格实现的失败；同一判定每级保持同一语义，差异只来自输入。用行为测试验证，不以代码形状或 grep 归零证明。
- 不改变缠论或交易判据来迁就基础设施。有关例外、退场条件及已裁口径随对应教义正本维护，不在本文件复制历史判例。历史纪律及验收原件见 `/Volumes/AgentStorage/restart-20261107/slim/docs-archive/before-3d61fb/`，按原仓内路径查找。

# PR1664 P1 首次无效 bar 污染时序返修

确认是真缺陷。原精确候选 `8ba59b157302ee6009376984fee74a6afe696c1d` 的 `capture_oos.rs` 在价格退出之后调用 `seg_step`，后者才给当前持仓加 `parse_invalid`。窗界强平、顺延出场、止损已经 `open.take()` 的交易会漏标。旧R2没有覆盖这个交叉场景，不能据旧PASS核消线程。

## 实施

独立工位 `/Volumes/AgentStorage/issue1585-p1-repair-20261026`，本地分支 `codex/1585-p1-repair-20261026`。原集成工位干净且保持不动；没有其他1585实现owner，原GitHub assignee保持。

将现有价格退出块原序提取为 `Replay::step`，主回测与负控共用。唯一行为改变是在任何退出之前给当前持仓记污染。窗界→顺延→止损→段确认顺序、劣侧价格、untradable挂起、边界计数及聚合剔除口径不变。`seg_step`只由step调用，原真实段确认接线负控也改走step。

新增 `tb1585_first_invalid_bar_taints_each_price_exit`。三条实际退出路径分别与有/无首次标记交叉，六场景检查交易仍退出、成交价和stopped/boundary状态不变、污染标记与聚合筛选结果正确、标记区间入报告。

## 远端验证

红测提交 `057f7a007d70bd2dfb8639f5862883f188d83ccb` 仅提取原逻辑并加负控。nc-remote实际运行失败：首次标记窗界退出的TradeRec.parse_invalid为false而预期true，0通过1失败，EXIT101。这是语义失败，不是编译或未命中测试。

修复提交 `892a870de255fbdf89f9f84f0c2c90261d8c9b04`。远端fmt指出两处格式，后续 `91ce01b293` 按输出修正。最终候选 `91ce01b293f50e0ec4ad606e0de71d5744fe05d0` 的远端 `cargo fmt --check && cargo test --locked --jobs 1 --lib tb1585` 通过，5 passed、0 failed，EXIT=0。日志 `1664-p1-validation26/20261026-081540-91ce01b293.log`。

原始红测与格式日志位于 `1664-p1-validation26/`。首次以detached HEAD送bundle未进入远端测试，改用具名branch后正常；未拿这次启动失败当红测。所有Rust均远端执行，commit跳过宿主hook并以远端fmt/test补验。

已普通快进推送到原 `sandcastle/issue-1585`，GitHub回读head为 `91ce01b293f50e0ec4ad606e0de71d5744fe05d0`。未合入、关票或resolve review thread；交主控非作者独审。

GitHub最终CI句柄：CI `40248717824`、DevSkim `40248717716`，创建后均queued。主控按精确head消费终态与非作者复审后，才可回复/resolve原P1并合入。


## 第二轮：独审 F1 聚合入口负控返修

独审 `1664-P1-INDEPENDENT-REVIEW26.md` 指出测试复制过滤表达式，没有调用主回测真实聚合入口。本轮只修该项，不重做已通过的污染时序。

新候选 `10ba8943062482b5ca6374de99a342e01721b3f4` 将既有诊断及四类聚合原样提为 `summarize_trades`，主回测与六场景测试共同调用。逐窗、pooled WF、pooled OOS、yearly 均断言污染交易不计入、正常交易计入。全部交易仍计入诊断：`tainted` 随标记变化，空理论层下 `no_theory=1`，总交易打印仍取原交易集长度。原 clipped 与 OOS_START 条件不变。

远端 `cargo fmt --check && cargo test --locked --jobs 1 --lib tb1585`：5 passed，0 failed，EXIT=0。日志 `1664-p1-validation26/20261026-082115-10ba894306.log`。

在 nc-remote 临时工位删除真实共享入口的 `!tr.parse_invalid` 过滤后，实际运行新增负控：编译通过，1 项测试命中并在“逐窗”断言失败（实际 1，预期 0），cargo EXIT101；包装输出 `MUTATION_REJECTED_AS_EXPECTED`、EXIT0。日志 `1664-p1-validation26/20261026-082251-10ba894306.log`。首次含 Unicode 的命令包装启动失败未算证据；已清理该次专属临时工位。mutation 不修改候选分支。

此次只更新 capture_oos.rs，不改经营规则、资源帽或隔离。交原非作者 reviewer 复审 F1，后由主控核对精确 head 与 CI；本工位不合入、不关票、不 resolve 线程。

普通快进推送完成，GitHub 回读精确 head 为 `10ba8943062482b5ca6374de99a342e01721b3f4`。新 CI `40249383882`（pending），DevSkim `40249383805`（queued）；旧 head 的 CI 不作为本候选验收。

# #1456 Sandcastle/Codex 管理入口与完整交付

本票交付 `.sandcastle/codex-child.{ts,mts,test.ts}` 和 `delivery-bundle.{ts,mts,test.ts}`，用法见 `.sandcastle/CODEX-CHILD.md`。产品提交为 `aa208aed38188ecc78205853e50c4b4d8ece4dd0`；后续封包仅追加报告和文档，最终版本由仓外交付 manifest 绑定。main 合入尚待具体批准，本报告不宣布 #1456 或 #1323 已关票。

原生管理子代理负责派发、等待、取消及接回；实际工蜂由 Sandcastle 0.12.0 的 Codex provider 启动，使用 Codex CLI 0.153.4、`gpt-6-astra`、`xhigh`。父任务、原生管理者、外部进程和 session 分别登记。旧 PrimeAgent/常驻队列、全局默认和业务政策保持原范围。

## 真实运行与故障处理

| 实际运行 | 外部会话 | 证据与结论 |
|---|---|---|
| 初次只读复核 | `01a09cab-357f-7c51-90cd-dd9934120168` | [工蜂原报告](first-worker-review.md)、[管理者核验](first-worker-postcheck.json)；复核指定文档并退出，输入无修改。初次参数位置错误的失败尝试已保留，不能算成功运行。 |
| 可复用入口只读独审 | `01a09cb7-6cdc-7a22-9cf9-2ca4f68911e3` | [原始审查](entry-initial-review.json)、[后置核验](entry-initial-postcheck.json)；发现 P1 监督异常清理和 P2 取消误报，正常退出不代表审查通过。 |
| 最终入口实际写入 | `01a09cc6-b0ab-73c2-b8c9-1e1e1af431be` | [实际结果](final-smoke-result.json)、[源绑定及清理](final-smoke-postcheck.json)；只写指定 37 字节探针，内容匹配，接回后仅删除该探针，工位恢复干净，进程组已消失。 |

三次均由 `/root/tb03b_contract_preparation` 管理。最终源码修复了同步状态回调、日志失败和 CLI 断管导致的脱管路径；会话查找等待后的取消保持取消状态。实际异常覆盖来自假 Codex 子进程夹具，不把单次真实写入当作全部故障场景验证。

## 验证与独立复核

- [验证实体](VALIDATION.json)：新增六文件 strict TypeScript 检查通过；入口 39 项、交付预检 26 项，共 65 项定向测试通过。保留实际 [入口日志](entry-tests.log)、[预检日志](delivery-tests.log) 和 [类型检查日志](typecheck.log)。
- [最终入口补审](ENTRY-FINAL-REVIEW.json)：继承原三文件完整外审，逐字重建原快照到最终源码的全部差异并补审，P1/P2 已修复，无剩余阻断；未重复执行已经通过的 39 项测试。
- [预检独审](PREFLIGHT-INDEPENDENT-REVIEW.md)及[完整证据](PREFLIGHT-INDEPENDENT-REVIEW.json)：正常对照可请求批准；索引隐藏修改、权限隐藏、子模块隐藏和 Git replace 四条误放行路径均被阻断。报告校验证据绑定，不代替报告语义判断。
- 产品提交的 [CI 34785441522](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34785441522) 四项均成功：test、rust-check、s-observer、fixture-drift。最终文档版本的实际状态应看 PR，不能把此运行改称最终文档提交的运行。

全仓 `tsc -p tsconfig.sandcastle.json` 仍有五项旧错误：旧 `main.mts` 的重复 import 和旧 `run-with-extraction.ts` 的泛型错误。固定 base 导出使用同一依赖复查，错误集合逐字相同；本票不宣称全仓类型检查通过。验证实体保留对照方法及原错误。

## 扫描与封包边界

实际扫描与逐项分诊见 [SCAN-TRIAGE.json](SCAN-TRIAGE.json)。扫描没有通过零发现门，旧基线问题仍按原票处理；本票不修改规则、忽略项或 baseline。任何例外都须随具体 PR/head 明确批准，不能将预检“可请求批准”解释为已获批准。

实体报告随产品放在同一 PR。交付 manifest 在仓外绑定最终 head、产品内容/权限、报告摘要、审查覆盖、已执行检查及扫描到最终文档之间的每项变化，避免报告将自身 commit 或摘要写回自身。最终预检输出是对这些既有材料的机械核对；若出现新的产品问题，仍须修复和补审。

本入口使用宿主 `noSandbox` 加 Codex `read-only/workspace-write`，不是 Docker 或全宿主读取隔离。原始工蜂输出不作通用脱敏；进程清理限于记录的原进程组，异步会话查找取消保证返回后终态正确而非即时中断，物理磁盘完全不可写时无法保证留下实体结果。完整限制见两份独审。

验证工位与分支已经核干净并删除；其固定提交仍由 #1374 的待交付分支保留。#1456 实施工位保留至具体批准和关票。此设施修复不增加 #1323 的业务完成数，也不恢复暂停的执行队列。

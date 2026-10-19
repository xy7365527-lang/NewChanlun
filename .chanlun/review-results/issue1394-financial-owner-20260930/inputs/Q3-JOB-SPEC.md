# #1394 C Browser 累计 Job：修订 2 独立 Spec 增补

结论：**Spec PASS**，仅对固定 `b1a8d8d45c5dacd1245e2f33f20f7d514ae2446e` 的纯 Job 核心生效。原 10 组与新增 4 组独立组合探针全部通过，未发现本次改动违反既有 Spec 的余缺。**工程复评未执行，仍须另行 fresh 工程轴。**

## 固定对象与差异

- parent：`61acbd43babe28397265e530b59c56342706547e`。
- candidate／复核时 HEAD：`b1a8d8d45c5dacd1245e2f33f20f7d514ae2446e`。
- `s_session/browser/s-content-job-v3.js` SHA256：`df8c2a3c9864fd2290e1c929dfb5d9cdad799539cca585e424440d29776b402c`。
- `s_session/tests/test_s_content_job_v3.cjs` SHA256：`642ff3efe62a43518dd187420508ce38bd8f6226bc33e52aef51992674e1cba4`。
- 未变内容核心 SHA256：`483d2c175b44320944112696a755462e0ece40d8b90f64214224bb72f5a7334e`。
- 未变权威 `APPROVED-SOURCE.md` SHA256：`fab912c6ebe3613969d65fc924368a1d5c9b343f0c3f7a123241645413771d30`，仍据 §5.6.1；既有 Rust Job 只解释已定账目语义。

小 diff 核对：统一 clock 回调重入门，clock 调用与值解析全部纳入永久停止路径，回调后重新检查 live；finish 的 open grant 判据移到 clock gate 后；config/limits/existing/node 字段先 exact 再单次捕获。测试仅追加五个工程回归组，原 25 项保留。

读取原工程报告 `revision1/STANDARDS-REVIEW.md`（SHA256 `423571c40a5b2c818f8ea26204fd78b2b040b91254b49dcbcac1ece5a868fad1`）以明确增补触发原因。原 61ac Spec PASS 保留原件，不用于抵销其后发现的 ENG-1/2/3。本报告只确认新实现遵守已定合同，**不替代工程复评或为工程问题直接结案**。

## 关键契约核对

1. **永久 actual**：仍先核入口 live 与实际量类型，再永久累计，然后读 clock／核 cap 和期限。倒退、非法返回及 callback throw 都保留已发生的 `3+7=10`；非法 actual、already-stopped 和未发生的 precheck 不新增计费。没有恢复原 SPEC-1 丢量次序。
2. **首次停止原因**：callback 异常或重入产生永久 `job_clock`；若 callback 先 cancel 再 throw，首次 `job_cancelled` 保留。恢复时钟只允许观测与合法清理，不能使 preIoCheck/finish 再通过。取消不回退累计。
3. **回调边界与清理**：callback 内即使吞掉重入异常，也不能改变累计、open、pending 或 committed；外层 gate 复核永久停止。正常 callback 外，stopped 后仍可释放旧 grant、真实 reconcile 和回收本 job staged；基线、保留 store 与 peak 语义不变。
4. **取消后的准入**：11 个入口逐个验证 callback cancel；预留不产生新句柄，async node 核 hash 后取消不接纳，actual 入口只保留已经发生的量。finish clock 返回后才核无 open；回调重入不能新建 open 后蒙混成功。
5. **exact 与快照**：schema/control/job_limits getter 单次读取，输出只含通过验证的字符串／安全正整数／冻结 limits；合法 getter 不改变配置域，不接受 object schema、字符串 control、数字 limit 或额外字段。node 每字段只读一次，不冻结 caller；node getter 内取消后不接纳。existing/limits 仍复制成独立基线与 cap。
6. **时钟和账目其余行为**：单调、绝对期限不续期、半开期限、>u64 精确计数、节点双 cap 接纳前判定、空间双账原子预留、opaque handle、深冻结快照均保持。

## 实际验证全集

`SPEC-probes.cjs` 开始时强制核新源码 SHA，包含原 revision1 十组及四个新组。输出见 [SPEC-PROBES.json](SPEC-PROBES.json)，**14/14 PASS，exit 0**。

| 组 | 覆盖 | 结果 |
|---|---|---|
| 原 1 | SPEC-1 六个 clock 错误／actual 反例 | PASS |
| 原 2 | preIoCheck 未发生不虚计，stopped charge 拒绝 | PASS |
| 原 3 | 两空间 cap 原子性、停止后清理及峰值 | PASS |
| 原 4 | actual/space >u64 与首次停止原因 | PASS |
| 原 5 | 大 Decimal clock、不续期、半开期限与非法 clock | PASS |
| 原 6 | foreign this、伪造／跨 job／复用 grant | PASS |
| 原 7 | 同根并发只接纳一次、坏 hash 不经 dedup 绕过 | PASS |
| 原 8 | Browser VM pending hash 取消与 caller 不冻结 | PASS |
| 原 9 | 三 actual 入口非法类型／already-stopped 不计费不读钟 | PASS |
| 原 10 | node 两 cap 在 seen／累计之前拒绝，wire 保留 | PASS |
| 新 11 | 13 种 clock 重入操作被捕获后仍永久停止且不改账，回调外清理合法 | PASS |
| 新 12 | 11 入口 callback cancel 后不准入，actual 保留 | PASS |
| 新 13 | 三 actual 入口 callback throw、cap=9/actual=10 停止，cancel→throw 首因保留 | PASS |
| 新 14 | exact getter 单读、类型域不放宽、node getter 取消不接纳 | PASS |

另由审查者执行 `node --test s_session/tests/test_s_content_job_v3.cjs`：**30/30 PASS**；两职责文件 `node --check`：**2/2 PASS**；parent→candidate `git diff --check`：**PASS**。原工程探针中“重入创建 open 后 finish 必须报 job_reservation”的旧断言不适用于新统一重入门：现在在创建 open 前永久 `job_clock`，这是更早拒绝同一非法操作，不是放宽成功条件；本独立探针直接检验不改账与不可安装。

## 交接范围

仅新增 revision2 的本报告、结构化 JSON、独立探针和结果。旧候选／原红／原 Spec／原工程报告与探针不覆盖；源码与测试由作者提交，本审查未修改。没有启动服务、真实浏览器或全 cargo，也没有执行 main／push、权限、预算或原件修改。

Browser physical allocation 域、existing 真实测量、adapter 在途结算、消费者与 Q3 接线、完整语义与安装、资源 profile、9011 仍不由纯 Job Spec PASS 证明。未新增这些范围的决定。下一步可在**同一固定 SHA**上做 fresh 工程复评。

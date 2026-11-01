# 收据 RUNTIME-INDEPENDENT-REVIEW.md

- 原件：`RUNTIME-INDEPENDENT-REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`a024735d01d90da8e639bb50959294fa54ee519ca8af5729a54b5ccd6644e524`
- 字节数：13199；行数：57

## 关键行（行号: 原文）

```
1: # #1488 run39095659808 原件独审与StaleWriter分叉诊断
3: 只读独审完成：PASS_FAILURE_EVIDENCE，仅表示保存的 API 记录与本地失败原件一致。六项结果为 1 成功、5 失败，不能判产品通过。03/04/05/06 共享跨请求预算归属缺口；01 是 Recover 撤销租约后仍在原 owner 续行的独立生命周期分支，涉及合同冲突，需根裁定。
5: ## 发现
27: 【夹具与产品边界、根控制点】03/06 使用正常 handle、固定 S7，预算缺口属于产品实现；04/05 明确修改私有预算字段作收紧探针，不能单独代表服务允许运行时改 S7，但也不能推翻03/06。01 则直接调用底层 recover、手改 Driver.epoch（driver.rs:980），并要求同 life 延续；公开恢复入口在 driver.rs:454 明写由新 Driver 继续，construction_state.rs:15 明写失效后须重建 owner。需根裁定同 owner 的成功 Recover 是否必须保留增量资格：若原成功合同要求保留，产品缺少受控 Recove
29: 【最小生产修复建议】预算分支应集中修复 owner 的跨请求接纳边界：由 owner 完成一次受控预算交接，绑定当前冻结合同及实际父账，完整携带 candidate/raw/source/set 的真实保留费和未释放许可；同一请求的 Accept→Begin→Publish 复用该预算域。交接必须在首次写入前完成，失败保持严格拒绝。不能只赋值 candidate.limit、换一个空 Budget、抹去旧 live/peak，或删除 :1192 比较。实施至少涉及 audit_owner.rs、request_audit.rs 与 native.rs 的归属接口；不扩大 Native 功能许可
35: ## 实际验证与限制
37: 保存的 TERMINAL-RUN.json/JOBS.json 绑定 run 39095659808、attempt 1、head 654a7f7a66fae4bb15cf8b5b5dc87afc23f33392、job 117581444860；run/job 均 completed/failure，执行步骤失败、上传步骤成功。工作流日志记执行退出码1。未访问 live GitHub API，此结论仅为保存记录与本地原件一致性。
45: HEAD/tree 核实为654a7f7a66fae4bb15cf8b5b5dc87afc23f33392 / 0e25bfb4036d809f424b087ca5b98f057bd9be19；产品 source/tree 为5cb7253d46d973669e7d10cd3331e12c9ef313b1 / 34eba1cfb6652d9a2981d07ff52c72bfeb9ed1c1。两个 Git 对象的 rust 下均恰580个 blob，与源map及当前文件逐项SHA-256相符、彼此同字节。产品 source 是执行 head 祖先，尾差恰为两份CI文件；CI及旧runner摘要均
55: 仅使用文本读取、纯Python标准库内存处理及只读Git对象检查；未导入或执行产品/runner，未读取.env或凭据，未联网、写checkout、提交、推送、派发或操作真实服务。收尾HEAD未变，tracked工作区无差异。
```

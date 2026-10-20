# #1394 页面工程返修增补

固定候选 `00b15b08bcd8d8e33406e29b8c7136bdef6b8513`，前驱 `54a6e5fd3df4e14a5e38edc9da39cfb6359ffc15`。只改现有 index 与必要页面回归测试；原三条工程红原件在 revision2，未覆盖。独立 Spec、Standards 复核由主控负责，作者未自称两轴通过。

页面将 `pageEpoch` 保留为请求、失败反馈和安装的代际；`q3ViewContext` 是已提交图的稳定呈现上下文。失败刷新、失败 Watch 或取消未换 committed 时，旧 details 和流式导出仍绑定同一完整图；新 committed 安装后旧呈现上下文失效。目录选择继续有自身 token，异步目录错误反馈也核请求与选择代际。没有借稳定呈现上下文放行旧候选或旧请求。

Q3 沿原 live 按钮解除 hidden，load/Watch 使用原 busy 与 finally 调度。健康读取结束只续排一个 timer，使用已冻结配置的 pollMs；暂停、错误和过期 timer 不再发请求。没有新增 poll 默认、页面、导航或接口。

旧 plain/legacy 调用者保留全部验证和原断言，在 State、Delta、目录轴、publication 的 await 后复核请求 epoch，并在 nested catch 反馈和原子安装前再核。R12 原 HTTP fixture 的 load 0..12 与 stream 0..40 微任务调度全部保留最新请求状态，健康 legacy load 仍可安装。

当前页面四项实际 bundle/Client 状态转换测试通过，94.894秒；DOM、timer、HTTP replay 和 allocation 仍是具名 TestOnly。旧 Browser 42、R12、既有选择/单 script 正反例及 bundle 漂移锁通过。没有重跑无影响的完整 Q3 387秒套件、Rust 构建或真实服务；没有声称真实 Browser、物理存储或9011成功。完整 SHA、补丁与原始 receipt 见 AUTHOR-RESULT.json。

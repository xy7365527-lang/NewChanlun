# #1394 客户端最终交付

最终固定SHA：9f923877e7d2dd9bb9c0a8cfe0c065bbfbda38da；父a67bad583ad5ed59e219c54c3ada0e70e1f4f7b3。

a67完整可见输入/原状态关联检查256/256沿用。最后只在Close分支新增既有text(request.reason)一行，依据finance.rs:1142及session_protocol.rs:77–82，不trim、不排序、不改跨请求owner纪律。

- reason-red：固定a67，两版null/空文本/布尔/对象/数组，10条全部Missing expected rejection。
- reason-green：受影响Close集合25/25、零失败/零跳过；含合法原文、空白与数字文本0、无排序唯一集合、不同slot请求同owner复用、过期豁免及实际Abort/Closed。
- 两修改JS node --check、git diff --check通过。256与25是组合复核，不称新SHA执行281项全量。
- reason-first-*保留初次测试构造问题：断言少了现有错误文本空格，numeric0本就由shared canonical在重签前拒绝；测试改为可传输类型，并把数字文本0作为合法正例。生产仍只有一行新门。

INPUT-RESULTS.md保留a67原件；INPUT-CHECKLIST.json是完整Rust可见输入解析/派生/使用核对表及接口不能认证的边界。REASON-PATCH.diff仅最后小增补，DELIVERY-FULL-PATCH.diff为基线至最终四文件补丁；DELIVERY-ARTIFACTS.json列原件SHA。

未启动服务/browser，未改case2/mounted副本/Rust/S3/FIFO/设计/资源配置/资金政策，未push或合main。由根接回最后小diff独评和实际browser，原件和全部旧FAIL保持。

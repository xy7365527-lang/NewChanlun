# #1394 完整既有 Q3 消费者交付

冻结候选为 `54a6e5fd3df4e14a5e38edc9da39cfb6359ffc15`。它保留初候选 `1feac8148da25981cd2f93f90d08d19a766c55d6` 和返修 `1cdc8b0f113c740c0362795148b54e5139ac284e`；独立红原件仍在本目录 `SPEC-probes-*` 与 `revision1/`，作者没有修改这些文件。整票与两轴验收由主控负责。

现有 `tb01c-client.js` 是原静态路由唯一脚本入口，Browser 分支装载固定字节 core/Job/私有模块，CommonJS 分支仍实际 require 同模块。更新入口为 `node s_session/tests/build_tb01c_browser.cjs --write`，漂移锁为同命令 `--check`；`BUNDLE-INPUTS.json` 逐项绑定源码 SHA、canonical runtime SHA 和最终 bundle SHA。`s-content-v3.js` 为已独评 07 修补同字节，Job 文件未改。

`TB01C.Client` 在显式 `protocol: "s-session/3"` 或显式 `/3` 资源 schema 时选择完整 Q3 客户端。Q3 要求完整 `s-query-client-resources/3`、八字段 job_limits 和私有 exact measured allocation 依赖；不提供默认额度或物理存储假值。旧 v2 Client 保留原默认和线协议。页面通过原有进程内 `SQueryClientContext` 传入协议/资源/私有依赖，URL 的 protocol 只选版本；没有资源或物理依赖时显示具体未验证原因。没有新增服务端 GET、host API、CLI 合同、页面或导航。

Q3 discovery 为同一 Job 的 `loadReady()`，只 GET `/api/ready`，核原四字段与 cut 六身份；后续仅 POST `/api/v3/snapshot|watch|content`。`load()` 可接受已明确版本和身份的有界发现值。请求、外部身份与资源均在异步边界前捕获；暂存、同代对拍、Gap 和安装共一 Job。全部六族、16 fixed_cut、七字段 projection、三声明值边、原排序与全文验证通过才一次安装；失败保旧 committed/cursor。

普通值和图值共用 Q3Values 与 q3-typed 的异步字段/位置判据。原公开 `validateTyped`、`validateTB02ShapeRefs`、`validateTB02Axes`、`validateTB02Raw`、`validateTB02CutSources` 已是异步函数，调用者已 await。旧纯静态 `validateTB02Catalog` 不再单独导出；其实际测试调用者改用完整共用 `Q3.validateState`，涵盖冻结目录、同 cut 证据、来源、生命周期、ID 和见证。旧 HTML 三 hooks 也支持 async；R9/R12 和 tb02b Python 的嵌入 Node 调用同步完成后再安装。

值层保留精确 JSON number token，按 UTF-8 键顺序与位置遍历，不整体展开大图为 JSON 对象。完整来源绑定 raw 历史、修订、payload_hash/receipt、唯一生效版本、三组和三槽。witness 保留原 wit- 配方；wire raw 必须先满足生产完整字段/类型前件，之后才恢复明列数字与 price=close。额外 price、未知键或缺字段不会被逆投影覆盖。

计费准入仍只由冻结 pure Job 和原 cap 决定。每次真实六族/七变化集合顶层访问在可能失败的验证之前记账，普通 nested raw_bars 条目不冒充顶层记录；独立 raw_history 完整重读另计。取消同步永久停止 pure Job，Hash 晚到不能接纳节点、不能安装。已发起的 I/O 若在永久 stop 后才返回，私有同 operation 实际账只保留真实尾量，不能重开或通过另一账成功；receipt 的 `accounting_operation`、`actual_total_fields`、`actual_tail` 指明实际账，节点/存储/准入仍来自 pure snapshot。clock 失效时未知 pure metrics 明写 unknown，不用零或估值补足。

原页面各区域保留，图视图使用 committed/page epoch 与独立目录选择 epoch。对象、撤回、raw、观察、目录及关系/见证 details 都能逐字段访问与流式导出；未取到的显示尚未加载。目录原七列与 raw 原八列由实际图视图驱动的模拟 DOM 已核。真实 Browser/视觉/9011 未运行；存储物理范围与真实适配器仍待用户决定。TestOnly 分配单位与模拟 DOM/HTTP replay 不构成 Browser 物理限额或产品前端 #1460 成功。

原件范围为 single-bar source-backed HTTP 与新 Controller 双 fresh 页31原件，加 S16 原生已封存 cut3/4/5 source-root oracle；没有新跑 S16 分类、服务或新 profile。S16 的 batch3 来自真实 manifest，未从 snapshot 子集构造。全部产品文件、fixture SHA、补丁 SHA、分字节测试 receipt 见 `RESULT.json`。初候选完整16项绿色只归其初候选字节，返修按同根靶向核验，未将旧绿色改标为新字节全量绿色。

Python helper 的独立审查原件为本目录 `AUTHOR-PYTHON-REVIEW.md` 与 `AUTHOR-PYTHON-RECEIPT.json`：零发现、六项真实探针，范围仅该调用者，不是完整消费者两轴评审。R9 缺历史七 cut 捕获输入，本轮只做脚本语法检查；未补造输入或启动服务。economic 默认32项通过，238依赖未提供原生 oracle 的项保持 skip；不得记作全270通过。

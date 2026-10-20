# Spec PASS — #1394 现有页面 revision3

固定候选 `00b15b08bcd8d8e33406e29b8c7136bdef6b8513`，增量 `54a6e5fd3df4e14a5e38edc9da39cfb6359ffc15..00b15b08bcd8d8e33406e29b8c7136bdef6b8513`。首尾 HEAD 一致、tracked clean。仅修改原 `index.html`，新增必要可移植测试 `test_q3_page_revision2.cjs`。未改 core、Job、正文语义、bundle、协议或资源合同。

本轮概念/Spec层通过，未发现新增或未解决的Spec冲突。原完整范围和已闭合语义/计费缺陷承接 `../revision2/SPEC-REVIEW.md` 及其指向的前轮证据；这里补页面状态转换范围。工程轴三项E1/E2/E3的原报告与红证据保留，工程最终复核由原Standards工位执行，本报告不替它给结论。

## §5.6 两态保留与完整访问

批准源 §5.6:288 要求未完成新请求不替换旧committed，渲染、来源追溯和导出仍通过同根遍历全部字段。本次 `index.html:356–373` 将请求的 `pageEpoch` 与已提交图的 `q3ViewContext` 分开；稳定呈现上下文只有在候选等于实际committed且匹配当前view对象时才能取得。失败、取消或迟到的新读取不能使旧已验证视图闭包失效；新committed的 `:728–730` 更新view对象，使旧闭包失效。目录选择仍另有selection token，没有把稳定view当作旧请求安装许可。

独立新增的实际边界探针使用当前原Browser bundle、真实Client、封存单bar内容与可控HTTP replay。步骤为健康提交→绑定一个完整projection details/export→真实刷新传输失败→继续使用原闭包→重新健康提交→再次触发旧闭包。结果：

- 失败后committed与view对象均保持相同，旧details完整列出7字段；不是把未加载范围当空。
- 旧流式导出写出78,963字节，使用独立内容编码器按精确JSON解析重建得到 `sc1-25bf56f161e22afb27f420ef0dadd3b70964bbca49604d31061ecd0622a57108`，与原已验证projection根逐字相同。完整导出SHA为 `32a78f3af812ea7210628be9a9cbba3c65c4c4d740bab92d59b39c313b83a298`。
- 详情与导出期间没有新增内容请求，读取的是已完整验证的保留图。
- 新committed安装后，原导出闭包不再打开文件选择器，也不发送请求。选择器仅调用1次，成功关闭1次。

证据为 `SPEC-probes-retained-export.cjs/json`。测试复用作者的模拟DOM/传输基础设施，独立断言完整导出根和新旧view边界；没有调用作者的测试断言函数。DOM、分配与文件接收器均为TestOnly，不是真Browser或物理资源验收。

## 原 Watch 与旧入口语义保持

| 范围 | 增量审查结论 |
|---|---|
| 持续Watch | `:754` 只解除原live控件hidden；`:756–789` 为load/Watch补原busy/finally调度；`:792–810` 清旧timer，按当前live/busy/请求epoch执行，沿实际Client冻结的pollMs。每次调用仍进入原Client.watch，其Delta、逐代同cutSnapshot及Content仍属同一个Job，没有片段重置预算。 |
| 停止和失败 | v2Failure仍停止live并保留committed；旧operation的catch/finally因epoch不同不能覆盖新请求反馈或续排。未删字段、简化显示或以空结果伪装成功。 |
| legacy Snapshot | `:1754–1757` 在完整异步validateRequestedState之后核epoch，才更新模式/渲染；原全部校验保留。 |
| legacy Gap | `:1893–1908` 在整份验证后、内部catch反馈前及安装前核epoch；Gap会话/指定cut绑定没有被删除。 |
| legacy Delta | validateDelta、目录轴、同代State与comparePublication的await后均核epoch；`:2117–2126` 在原六集合与cursor原子提交前再核。原完整publication与逐字段对拍仍执行，未废弃legacy路线。 |
| 对象选择/布局 | 原selection token继续约束目录异步渲染，选择错误反馈额外绑定请求和选择代际；原布局和单script入口不变。 |

逐一读过以上实际调用点及新增可移植测试，不依赖作者承诺。作者当前SHA的页面转换4/4已terminal，94894.212292ms；旧实际validator微任务load13种、stream41种调度均零旧安装，另有健康legacy安装正控。旧Browser42、R12、selection/singleScript2项和bundle漂移证据均已绑定当前原件；未把旧54页面SHA移贴为当前通过。没有重复运行387秒全套。

## 固定来源与适用边界

唯一批准源SHA仍为 `fab912c6ebe3613969d65fc924368a1d5c9b343f0c3f7a123241645413771d30`，旧语义复用盘点SHA仍为 `be8ee8edf38ef7202110954ebf64ec2e89f36fdea91716cf3b76a9aae7f9a2ef`。这次没有新增缠论概念定义，采用与前轮相同的定义正本及完整业务判据。

独立核验原RESULT与revision3覆盖清单合并后的18项产品SHA、6份当前receipt SHA与页面补丁SHA，全部匹配固定git对象/原件。页面patch SHA为 `abb93663b028ad61b95ada77350ce5696633e5ffd4f3cc41862993b369e72ca5`。独立bundle `--check` 通过，bundle仍为 `19bad6e5dd7d9c06f68f288e212afc82d8976fdc612d30622165ff7879f8c6ed`。当前index SHA为 `5ccad5409e57420f3a5313432ced67ca58810fa6dc20f7d307aef64d7c978770`，新测试SHA为 `9caeb17e018037ce42c4d064ca1f5f4f605cf0d0d5e88bd38a2a118ee46aab82`。

报告、独立probe和所有复用证据的SHA见本目录 `SPEC-REVIEW.json` / `SPEC-REVIEW.SHA256`。未修改产品、作者原件或旧审查结果。

结论：**Spec PASS，0项发现。** 这是固定实际caller及原页面的合同增量审查，下一步为工程轴复核；仍不表示真实Browser、物理分配适配、S16正式Q、9011、#1323整图或#1460前端产品已完成。没有main合入、push、服务或资源域扩展。

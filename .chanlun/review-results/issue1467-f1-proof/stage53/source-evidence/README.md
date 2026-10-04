# Stage53 源义作者包

本目录与上级 `seed-departure-source-scope.md` 是同一作者包，非独评。主发现是 `061-第61课.md:44` 的五段角色次序与 70–73 / 69–72 判例。本包只排除固定三成员种子下第三成员充离开的原义证书，不排除初始 Ω 的内部数值引用，也不单凭段数否定任意已延伸成员的所有其他证据用途。

`source-excerpts.json` 保留15份指定正本或历史稿的165行逐字摘录、完整源文件哈希、作者/编注说明。所有完整源文件与指定基线逐字节一致。四份 issue 快照均为只读获取；812只留票面及含Z-5或S-3的评论，241只留票面，不把这些筛选快照写成完整评论史。网络转载复核与图像读取失败记录在 `web-crosscheck.json`。

`check-source-scope.mjs` 只复算六事件以及旧主例前七事件，用原始订单消息重建q和已封腿；输出为 `source-scope-checks.json`。没有复跑Stage52解析器，没有新市场样本。所有原义身份与发生/可知/准入字段仍为空。

复现作者计算与字节锁：

```sh
node .chanlun/review-results/issue1467-f1-proof/stage53/source-evidence/check-source-scope.mjs
node .chanlun/review-results/issue1467-f1-proof/stage53/source-evidence/verify-freeze.mjs
```

在研究worktree根目录运行。第一条确定性重写同字节的作者结果，第二条只读核验。清单本身不自哈希；交接时单独报告其SHA-256。

新上下文审查应核：061的作者归属和最小五对象作用域；S-3是否确给破坏/三类点同一身份；S53-S1是否只作用于同一个固定种子；037证据引用与构成归属是否分开；Z-5的连接对象是否被偷换；四类时刻是否混同；重锚C′是否缺实际前置对象；初始F₁自由与原义F₂桥是否分开。不要把这里的作者断言作为审查结论。

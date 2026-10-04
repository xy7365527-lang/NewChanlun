# Stage54源义作者小包

入口为 `../exit-member-source-scope.md`，工作票 #1467，冻结基线 `23f33fb8ae0df856bf2281d196e3d3190579f295`。只审触核材料、离开连接与中心 outer 的来源合同。027:854/856作者资格继续暂停；037限于同式消费；Z-5政策不重判。

- `source-audit.json`：本地源文件哈希、逐字行与作者/编注边界。
- `web-crosscheck.json`：直接新浪作者页的本轮正文交叉核验摘要，非网页全文存档，不认证缺失评论。
- `check-member-source.mjs` / `member-source-checks.json`：一条既有7事件合成订单控制的逐ID重放，以及两个9坐标数学词的成员/范围/时钟检查。均不填语义身份。
- `frozen-author-manifest-v1.json`：作者件与外部输入字节身份。
- `verify-freeze.mjs`：哈希和逐字行校验，不判断语义，也不代替独评。

复现时先运行 `node .chanlun/review-results/issue1467-f1-proof/stage54/source-evidence/verify-freeze.mjs`，再运行 `node .chanlun/review-results/issue1467-f1-proof/stage54/source-evidence/check-member-source.mjs`，最后再次校验字节。算术输出应逐字稳定。`freeze-source-evidence.mjs` 仅供作者首次冻结，复审不要运行它重写清单。

没有新增原生订单、市场样本或枚举批次。原生账为7事件引用/8状态/7次应用；数学账为1个Stage53旧词与1个新手写范围反例。没有完整F₁、F₂、P或原义连接证书。本包尚待新上下文独立审查。

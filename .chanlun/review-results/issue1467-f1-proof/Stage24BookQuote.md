# Stage24：相同合法前缀的报价轨迹不能自由选择

日期：2026-10-04，#1467，`BOOK-QUOTE-PREFIX-v1`。接续[开工卡](Stage24WorkCard.md)，沿用Stage23的价量模型与Step，不改变其有效域。

## 结果

已证明：初始每个价位的数量相同、原事件前缀相同，且双方轨迹均满足原Step时，每个前缀位置的价量汇总相同，正确的报价值及有无有效报价也一致。因此，送入原construct的两份正确报价轨迹相同，不能任意改变几何输出。

这里不要求价格单位列表本身相同。见证的初始列表及第一步之后的列表都不同，后续报价和空侧结果仍一致。这补上了Stage23报价轨迹来源的一项一般义务，没有把轨迹当作自由辅助输入。

## 计数到报价的证明链

[BookQuoteSpec.lean](BookQuoteSpec.lean)以每个价格的计数相等定义SameInventory；买卖两侧分别相等定义SameBook。“相同”覆盖模型中的全部价位，不仅是当时的最优买卖价。

对单次Update，证明每个查询价格p的数量平衡：新增时`after.count p = before.count p + added.count p`；取消/执行时`before.count p = after.count p + removed.count p`。added/removed是事件指定价格、数量的单位列表。由此推出，相同事件的两次合法Step保持SameBook。

计数相等进一步给出价格成员集合相同。结合BestBid/BestAsk的独立极值定义，得到QuoteOf的可传递性和唯一性，不依赖列表排列。

Read有两个有区别的含义：some q必须实际满足QuoteOf；none必须证明不存在满足QuoteOf的报价。**none不是“还不知道”**。断档、尚未收到快照或迟到导致的暂不可知，不能直接用这项否定证明替代。

## 前缀与构造结果

Through要求前n条事件实际存在，并逐条满足Step。一般前缀定理允许两个完整事件列表在n之后不同，只要求`take n`相同；从初始SameBook出发，对所有i≤n证明SameBook及Read唯一。

因此该证明不读取未来事件来选择当前报价。进一步对长度为n+1的两份正确报价列表逐项证明相等，再接原MovingQuote.construct得到完整输出相等。

“正确报价列表”是带Reads证据的列表，不是任意长度匹配的数组。原构造器依然不是完整协议验证器，实际实现还需证明其报价与事件转换符合这些前提。

## 非平凡见证

两个初始买侧分别为`[100,101,100]`和`[100,100,101]`，卖侧均为`[103]`。它们列表不同，但同价位数量相同。依次执行：撤100一单位、撤101一单位、执行100一单位。

| 状态 | 买侧A | 买侧B | 正确报价 |
|---|---|---|---|
| 初始 | [100,101,100] | [100,100,101] | 101/103 |
| 第一步后 | [101,100] | [100,101] | 101/103 |
| 第二步后 | [100] | [100] | 100/103 |
| 第三步后 | [] | [] | 无有效双边报价 |

每条转换均有实际Step见证，各状态均符合模型非交叉条件。原construct仍保存三个价量事件；因这个未确认尾块跨到空买侧，geometry为空。不能把“内部列表对象相等”当作本轮结论，两份列表已经实际不同。

## 保留的边界

本轮证明是**给定两条均合法的轨迹，其报价读出确定**，没有证明每条任意消息流都存在合法轨迹。旧Update的删除仍要求连续同价数量块见证，因此也未证明仅凭总数量足够就能执行任何删除；没有借此次证明放宽该规则。

该模型不识别订单ID或FIFO，报价相等不代表订单身份、排队位置、成交机会或实际执行结果相等。真实场所的消息投影、初始化、数据覆盖范围和状态转换仍需独立验证；有限深度缺失不能当作该价位数量为0。

这里使用逻辑事件前缀。报价何时真正可知、捕获时钟、缺口恢复后何时发布，仍属P2接线，未由价格确定性自动解决。一般locate到Completed精化及完整F₂、独立审查、四臂价值实验也继续开放。

## 检查记录

证明见[BookQuoteProof.lean](BookQuoteProof.lean)。精确根`BookQuote.exact_root : COUNT_TARGET ∧ PREFIX_TARGET ∧ WIRE_TARGET ∧ WITNESS_TARGET`首轮通过，run `99f242c63e744b848bcbfc7b39612dae`。Lean4.31、exit0，公理仅propext/Quot.sound，无sorryAx、自定公理、未知或不安全依赖，声明锁未变。独立语义审查not_reviewed。

完整证据位于`/Users/silencehan/Documents/Codex/research-evidence/issue1467/book-quote-v1/`。freshness、命令及体积见[机器摘要](stage24-lean-verification-summary.json)，输入绑定见[源指纹](source-sha256-stage24.json)，复现与审查入口见[待审包](Stage24PendingReview.md)。没有修改formal/生产、下载或开展确认实验，研究票与goal保持开放。

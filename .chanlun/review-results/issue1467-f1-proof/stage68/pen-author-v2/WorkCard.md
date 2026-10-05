# DP68-v2 撤销传播修复工作卡

研究票1467；2026-10-06；只写pen-author-v2及adoption指针，不改任何v1冻结字节。本轮依据pen-review唯一MEDIUM，实际补实现，而非仅限缩声明。模型仍为DP68；原义各门仍null。

新增withdraw_active接口：撤销当前tentative笔证据时，沿terminalCandidate清除raw/whole当前end、终点提案引用、端点knownAt、冻结缓存与原始证齐状态；完整旧载荷留在append-only事件中。已stable笔或已由stable承诺支持的冻结对象不得撤销，先拒绝并留证，不改变其承诺载荷。active历史记录保留，当前有效性另记。后续绑定必须逐项核候选载荷与当前末笔，冻结前再核当前末笔、terminalCandidate、end与stable证书完全一致。

新active若未被历史proposal表示，清旧依赖并记unrepresented/pending，不创造过去的proposal；只有已有合格proposal且直接点K线几何通过才重建端点。端点版本替代与整族淘汰仍分开；不以版本不可变证明原义身份稳定。

验证预算：一次443既有前缀主回放（必要代码修复验证），≤96MiB、零R_W、零新历史、无Lean。回放内捕获122与138状态；最多10条接口状态消息控制，验证已有待稳定端点的撤销传播、缺proposal时pending、已有proposal重新绑定、稳定对象撤销/改写被拒、冻结前身份不符被拒。控制是接口消息测试，不是第二条真实或合法行情历史；真实active替换仍未在本历史覆盖。

在线读取边界沿用v1并明确：source数组预载，prefix流文本按65536字符缓冲（不是字节精确边界），Stage67结果逐行到当前cut；终值文件先散列字节，在线seal后才JSON解析。主回放后的v1有限核心结果逐字段/哈希比较。最终交付完整v2代码/Report、diff、输入前后hash、一次主运行及状态控制回执、manifest/FINAL；父工位另派独评。

# Hyperliquid 回报映射

`HyperliquidPaperFacade` 将 Strategy 的接受、拒绝、成交、撤单回报写入 JSONL；原始字段递归过滤 private key、token、secret、password、credential 和 api key。查询 facade 只读取 Strategy cache 的未结单与净持仓。

本文件记录 #1523 的 adapter 接口边界；paper/live 同核、完整责任闭环和旧入口退役仍分别属于 #1367、#1362、#1365。

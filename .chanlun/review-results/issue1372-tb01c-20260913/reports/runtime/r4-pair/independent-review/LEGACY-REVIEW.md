旧 launcher Q 启动参数修复：有界通过。

固定提交 `2317369c3de2e3d9e8d90a545dd32d0e759d2b8b`，源码 SHA256 `28e052bb0ec7f77bddfac932c602f89c2e0eb4d902df2e451c35da09b03cad0f`。仅新增明确的资源与 Q 化身参数，在构建、reset、init、accept 前调用 Q 自身解析器；没有静默默认。原 service 提前 exec 与 stop 分支不变。

独立读取三份拒绝原件（缺参数、epoch=01、资源不存在）及真实旧入口回执；三个失败目标当前均不存在。三端独立的 ok/producer_epoch 包装均正确，state 内 catalog/snapshot 与对应端点业务内容逐字段相等：cut=1、Q epoch=1，无 v2 session_generation，继续已有 legacy 浏览器分支。本工位未重新启动服务。HTTP200来自根检查事实，存盘 body 未包含HTTP原头。

R4/GUI仍绑定 1a57fb684e5a6aa18e4f962b68ba3d9f9a908c66。此文只支持新 launcher 兼容增量。精确原件与源码行号见 LEGACY-REVIEW.json；临时oracle的接口包装误判亦已具名记录。

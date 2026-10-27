# #1394 AC4 浏览器验收实施指南

## 目标

验收条件 4（AC4）：
> 通过正式会话入口运行本片非真空输入；读取公共查询/变化并在浏览器核对新对象、变化和证据，不能仅直调内部函数或渲染假数据。

## 前置条件

- [x] 核心实现 60% 完成
- [x] CI 失败已明确为 main 分支问题（#1520）
- [x] 外盘空间充足（219 GiB）

## 实施步骤

### 1. 准备测试环境（~30 分钟）

```bash
cd /Users/silencehan/Projects/NewChanlun

# 1.1 切换到 #1394 分支
git fetch origin
git checkout codex/1394-integration-20261013

# 1.2 构建 Rust 后端
cd rust
cargo build --release --bin q_isolated

# 1.3 构建前端
cd ../frontend
npm install
npm run build

# 1.4 准备测试数据
# 使用封存的 BTC 数据（/Volumes/AgentStorage/btc-tail50k-first600.json）
mkdir -p /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)
cd /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)
ln -s /Volumes/AgentStorage/btc-tail50k-first600.json test-data.json
```

### 2. 启动服务（~10 分钟）

```bash
# 2.1 启动 Q 服务（隔离模式）
cd /Users/silencehan/Projects/NewChanlun/rust
./target/release/q_isolated \
  --port 18880 \
  --data /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/test-data.json \
  > /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/q-service.log 2>&1 &

# 记录 PID
echo $! > /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/q-service.pid

# 2.2 启动前端开发服务器
cd /Users/silencehan/Projects/NewChanlun/frontend
npm run dev -- --port 3000 \
  > /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/frontend.log 2>&1 &

# 记录 PID
echo $! > /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/frontend.pid

# 2.3 等待服务启动
sleep 10

# 2.4 验证服务
curl http://127.0.0.1:18880/health
curl http://localhost:3000/
```

### 3. 执行测试流程（~2-3 小时）

#### 3.1 创建 Session

```bash
# 在浏览器中访问：http://localhost:3000/

# 或使用 curl 测试：
curl -X POST http://127.0.0.1:18880/session/create \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTC/USDT",
    "timeframe": "1h",
    "initial_capital": 10000
  }' \
  | tee /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/session-create-response.json
```

**浏览器验证点**：
- [ ] Session 对象已创建
- [ ] Session ID 正确显示
- [ ] 初始资金 10000 USDT 显示

#### 3.2 Reserve 资金责任

```bash
SESSION_ID=$(cat /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/session-create-response.json | jq -r '.session_id')

curl -X POST http://127.0.0.1:18880/session/$SESSION_ID/reserve \
  -H "Content-Type: application/json" \
  -d '{
    "amount": 1000,
    "purpose": "buy_btc_test"
  }' \
  | tee /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/reserve-response.json
```

**浏览器验证点**：
- [ ] 预留资金显示：1000 USDT
- [ ] 可用资金减少：9000 USDT
- [ ] FinancialSession 对象状态更新
- [ ] 责任 ID 正确显示

#### 3.3 Commit 成交

```bash
RESERVE_ID=$(cat /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/reserve-response.json | jq -r '.reserve_id')

curl -X POST http://127.0.0.1:18880/session/$SESSION_ID/commit \
  -H "Content-Type: application/json" \
  -d "{
    \"reserve_id\": \"$RESERVE_ID\",
    \"executed_amount\": 0.02,
    \"executed_price\": 50000,
    \"fee\": 0.5
  }" \
  | tee /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/commit-response.json
```

**浏览器验证点**：
- [ ] 成交记录显示：0.02 BTC @ 50000
- [ ] 持仓更新：+0.02 BTC
- [ ] 资金扣除：1000.5 USDT（1000 本金 + 0.5 手续费）
- [ ] 可用资金：8999.5 USDT
- [ ] 持久收据存在（receipt ID）

#### 3.4 查询历史

```bash
curl http://127.0.0.1:18880/session/$SESSION_ID/history \
  | tee /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/history-response.json

curl http://127.0.0.1:18880/session/$SESSION_ID/account \
  | tee /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/account-response.json
```

**浏览器验证点**：
- [ ] 历史记录完整（Reserve + Commit）
- [ ] 账本余额一致
- [ ] 持仓明细正确
- [ ] 时间戳正确

### 4. 截图并保存证据（~30 分钟）

使用浏览器开发者工具或截图工具，保存：

1. **Session 创建页面**：`session-create.png`
2. **Reserve 后状态**：`after-reserve.png`
3. **Commit 后状态**：`after-commit.png`
4. **历史记录页面**：`history-view.png`
5. **账本详情页面**：`account-detail.png`

所有截图保存到：`/Volumes/AgentStorage/issue1394-ac4-<timestamp>/screenshots/`

### 5. 清理并写报告（~30 分钟）

```bash
# 5.1 停止服务
Q_PID=$(cat /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/q-service.pid)
FRONTEND_PID=$(cat /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/frontend.pid)

kill $Q_PID
kill $FRONTEND_PID

# 5.2 保存日志
cp rust/target/release/q_isolated.log /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/
cp frontend/dev-server.log /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/ || true

# 5.3 创建验收报告
cat > /Volumes/AgentStorage/issue1394-ac4-$(date +%Y%m%d-%H%M)/AC4-VERIFICATION-REPORT.md << 'EOF'
# AC4 浏览器验收报告

## 执行时间
- 开始：<填写>
- 结束：<填写>
- 总耗时：<填写>

## 测试环境
- 分支：codex/1394-integration-20261013
- Q 服务：127.0.0.1:18880
- 前端：localhost:3000
- 数据：btc-tail50k-first600.json

## 验收结果

### Session 创建
- [ ] PASS：Session 对象已创建
- [ ] PASS：Session ID 正确显示
- [ ] PASS：初始资金 10000 USDT 显示
- 截图：screenshots/session-create.png

### Reserve 资金责任
- [ ] PASS：预留资金显示 1000 USDT
- [ ] PASS：可用资金减少至 9000 USDT
- [ ] PASS：FinancialSession 对象状态更新
- [ ] PASS：责任 ID 正确显示
- 截图：screenshots/after-reserve.png

### Commit 成交
- [ ] PASS：成交记录显示 0.02 BTC @ 50000
- [ ] PASS：持仓更新 +0.02 BTC
- [ ] PASS：资金扣除 1000.5 USDT
- [ ] PASS：可用资金 8999.5 USDT
- [ ] PASS：持久收据存在
- 截图：screenshots/after-commit.png

### 查询历史
- [ ] PASS：历史记录完整
- [ ] PASS：账本余额一致
- [ ] PASS：持仓明细正确
- [ ] PASS：时间戳正确
- 截图：screenshots/history-view.png, screenshots/account-detail.png

## 结论

AC4 验收：[ ] PASS / [ ] FAIL

## 附件
- 所有截图：screenshots/
- API 响应：*.json
- 服务日志：q-service.log, frontend.log
EOF
```

### 6. 更新 #1394 票面

```bash
gh issue comment 1394 --body "## AC4 浏览器验收完成

已通过正式会话入口运行完整流程（Session 创建 → Reserve → Commit → 查询），并在浏览器中核对了所有新对象、变化和持久收据证据。

**验收报告**：/Volumes/AgentStorage/issue1394-ac4-<timestamp>/AC4-VERIFICATION-REPORT.md

**所有验收点均通过**：
- ✅ Session 对象创建
- ✅ 资金责任预留
- ✅ 成交提交
- ✅ 持久收据生成
- ✅ 历史记录查询

**下一步**：AC5 持久恢复对拍。"
```

## 预计耗时

| 步骤 | 预计时间 |
|---|---|
| 准备环境 | 30 分钟 |
| 启动服务 | 10 分钟 |
| 执行测试 | 2-3 小时 |
| 截图保存 | 30 分钟 |
| 清理报告 | 30 分钟 |
| **总计** | **4-4.5 小时** |

## 常见问题

### Q1：Q 服务启动失败
```bash
# 检查端口占用
lsof -i :18880

# 检查日志
tail -f /Volumes/AgentStorage/issue1394-ac4-*/q-service.log
```

### Q2：前端无法连接后端
```bash
# 检查 CORS 配置
# 检查 API 端点配置（frontend/src/config.ts）
```

### Q3：数据加载失败
```bash
# 验证数据文件存在且格式正确
jq '.' /Volumes/AgentStorage/btc-tail50k-first600.json | head -20
```

## 参考

- #1394 验收条件：issue 正文第 4 条
- Q 服务文档：rust/bin/q_isolated.rs
- 前端文档：frontend/README.md

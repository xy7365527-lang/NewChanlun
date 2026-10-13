# #1394 资源实际 load 绑定，revision1 候选

本轮只修 Python Standards MEDIUM 的文件改值又恢复窗口。原候选 `b86c0c291952879af67882e79d9687f51ce1741f`、旧 binary、原 API/source manifest/patch/Spec/Standards 已封存在 baseline-artifacts 与 baseline-ab54-s_structure_session，不覆盖旧失败和旧结果。

`Controller.start_process` 的 S3 冻结资源身份存在时，从其已核 SHA 生成私有 `S_CONTROL_RESOURCE_SHA256`，覆盖 ambient 同键。无此 S3 身份时清除该私有声明，旧 `/2` 与 Q 不传它。调用方的环境对象不被修改。现有进程 nonce、argv、PID/UID、socket、资源自有文件检查与新旧预算重启口径均保留。

Rust S3 serve 在 `SResources::load` 后即核该声明，位于 clock/profile、写锁、数据库打开及监听之前。声明须为 64 位小写十六进制规范 SHA256；非 UTF-8、非法声明或实际资源 canonical digest 不匹配时明确拒绝。无私有声明的 direct CLI 继续按其显式资源文件执行。expect digest 针对实际加载并保留在 SResources 内的规范七值，即使同 UID 写者随后恢复原文件，也不能改变已经核过的资源值。

未新增公共配置字段、宿主端点、默认资源数值、资源量纲或业务判据。具名原七资源数字没有改变。旧 Driver19/state7 的源码无 diff，可复用其已执行日志；本轮实际执行 Python 来源/覆盖/旧2不传锁、Rust helper 无声明/正确声明/非法声明/改值不匹配锁及 non-test check/build。计数与新二进制 SHA 见本目录 AUTHOR-RESULT.json。

`check-resource-binding-negative.py` 是供根独立工位执行的真实 CLI 负例。必须显式给当前 binary、原具名 resource file、尚不存在的自有 output-dir 及验证 timeout-ms。脚本仅把私有声明设为错 digest 或非法大写文本，使用自有数据库 sentinel，要求进程先给私有绑定诊断，数据库不变且 writer lock/WAL/SHM/socket 均不存在。clock/profile 故意不是可服务配置，因此它还验证绑定拒绝早于这两项。作者仅做语法检查，未执行此 CLI、未启动服务。合法服务生命周期与增量独评仍由根任务完成。

示例参数由根选择自有 output-dir 后执行：

```text
python3.12 revision1/check-resource-binding-negative.py
  --binary <revision1 对应 binary>
  --resource-config <根已冻结 resources.json>
  --output-dir <新的自有负例目录>
  --timeout-ms <显式验证期限>
```

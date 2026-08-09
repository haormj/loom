# Django 日志

## 何时使用

仅当任务拥有 Django 日志基础设施时才使用此参考。View、service、command 和 job 否则仅在各自任务拥有的诊断边界使用已配置的 logger。

## Provider 决策

1. 保留仓库已有的 Django/Python 日志配置。
2. 对于全新项目，通过 Django 的 `LOGGING`/`dictConfig` 边界使用标准 Python `logging`。
3. 仅当仓库已选择 structlog 或其他 provider，或已接受的结构化事件需求证明其合理性时才保留它们。

## Implementation Focus

慎重地使用模块 logger 和 Django category。不要 wholesale 禁用 Django 已有的日志树，或在传播启用时将同一 handler 挂载到父和子 category。

## 配置归属

将 formatter、filter、handler、category 级别和目的地保存在从环境特定配置组合的 settings 中。避免导入时 `basicConfig`，不要在基础 settings 中放置密钥或生产路径。

将 access/security category 与业务诊断分开。保留 Django 安全日志行为，在自定义 filter 或 formatter 序列化之前脱敏敏感的请求元数据。

## 关联与边界

当支持 async 执行时，在中间件中使用安全的请求元数据和 `contextvars` 建立一次请求关联。每次响应后清除上下文。Management command、Celery task 和其他 worker 在各自的入口边界创建或传播操作 id。

关键转换、依赖结果、async 结果、重试、终态失败和意外错误仅记录一次。DRF/Django 异常处理负责最终的意外请求失败；view 和 service 不记录并重新抛出同一异常。

## 异步与文件输出

仅当已接受缓冲输出时才使用生命周期拥有的 queue listener。限定队列大小，定义过载行为，并在实际 WSGI/ASGI 或 worker 生命周期下停止/排空它。

Console 输出是全新应用的默认选择。文件输出需要已接受的应用需求加上进程安全的轮转、压缩、保留、磁盘限制和路径不可用行为。多进程 WSGI worker 不得共享不安全的轮转 handler。

## 归属与失败策略

任务必须确定它是否拥有 provider 设置、事件埋点、队列缓冲、文件输出或仅验证。可选的遥测失败不得决定请求正确性；安全和恢复事件仍受已接受的过载策略约束。

将日志生命周期和 settings 组合保持在所选的 Django 启动边界中。Deploy 不发明 handler、文件路径或保留行为。

## Evidence Focus

记录所选的 settings 边界、category 和传播策略、拥有的事件、关联和脱敏规则、worker 模型以及聚焦的运行时证据。

## Verification Focus

- 加载实际的 Django settings，验证 handler/category 传播不会产生重复事件。
- 捕获请求或 task 事件，断言稳定字段、关联、级别和脱敏。
- 证明预期的验证/权限结果不会被当作意外服务器失败发出。
- 当拥有队列或文件输出时，测试 worker 生命周期、饱和、进程假设、轮转、保留和目标失败。

## 配置审查

- 从已验证的环境特定 settings 派生级别、handler 和目的地。
- 有意地将 Django、server、安全和业务 category 分开。
- 在选择文件或队列 handler 之前验证进程和 worker 假设。
- 在缺少必需 settings 时执行启动，确认失败是可操作的。
- 保持脱敏和关联行为在请求、command 和 job 之间稳定。
- 在更改全局日志树之前记录所选策略。

## 不安全默认

- 未审查 Django 和 server category 就使用 `disable_existing_loggers: true`。
- 重复 handler 与传播组合。
- 记录请求 body、cookie、凭证、token 或原始 ORM/provider 错误。
- 在 view、service、task 和异常中间件中捕获/记录/重新抛出。
- 文件轮转配置未考虑多个应用 worker。

# FastAPI 日志

## 何时使用

仅当任务拥有 FastAPI 日志基础设施时才使用此参考。仅发出所拥有事件的路由和 service 任务使用已配置的 logger，不重新设计全局日志。

## Provider 决策

1. 保留仓库已有的 Python 日志 provider 和配置。
2. 对于全新 FastAPI 应用，使用通过 `dictConfig` 配置的标准 `logging` API。
3. 仅在已建立或显式需要时保留 structlog 或 Loguru。不要在已有 provider 旁边引入第二条事件管道。

## Implementation Focus

应用模块通过 `logging.getLogger(__name__)` 获取命名 logger。慎重地保持 Uvicorn access/error 集成，使应用错误和访问事件不重复。

## 配置归属

在应用工厂或启动模块中创建一个配置边界。将级别、formatter、handler、传播和目的地选择保持在结构化配置中。不要从导入的功能模块运行 `basicConfig`。

使用稳定的事件名和结构化字段。如果所选 formatter 不能保留字段，在组合根配置 adapter/formatter，而非在业务代码中拼接 JSON 字符串。

## 关联与边界

在 ASGI 中间件中创建或接受一次请求 id，并使用 `contextvars` 存储安全的关联上下文。在 `finally` 中重置上下文。为没有请求的定时任务、worker 或消费者生成操作 id。

在一个任务拥有的边界记录关键转换、外部结果、重试终止、async 结果和意外失败。异常 handler 映射公共错误；service 和路由不得重复记录同一异常。

## 异步与文件输出

当已接受缓冲日志时，使用 `QueueHandler` 和生命周期拥有的 `QueueListener`，配以有界队列和显式饱和行为。随应用 lifespan 启动和停止它；不要在测试中留下 listener 线程。

默认使用 console 输出。应用拥有的文件需要已接受的需求、稳定的路径、进程安全的 handler 选择、大小/时间轮转、压缩/保留策略和目标失败行为。标准轮转 handler 不自动对多 worker 进程安全。

## 归属与失败策略

任务必须确定它是否拥有 provider 设置、事件埋点、队列缓冲、文件输出或仅验证。可选的日志或 exporter 失败不得改变业务结果；必需的安全和恢复事件遵循已接受的过载策略。

将 handler 生命周期保持在应用 lifespan 中，将部署、密钥和外部 collector 排除在此参考之外。

## Evidence Focus

记录 provider 和配置边界、拥有的事件、关联和脱敏规则、worker 假设以及所选输出行为的聚焦运行时证据。

## Verification Focus

- 通过真实 lifespan 启动 ASGI 应用并捕获一个任务拥有的事件。
- 验证稳定字段、关联传播、级别选择、脱敏以及无重复的 Uvicorn/应用错误。
- 执行并发请求以证明关联值不在上下文间泄漏。
- 当拥有队列或文件输出时，验证饱和、listener 关闭、多进程假设、轮转、保留和不可用路径。

## 配置审查

- 从已验证的运行时配置派生级别、handler 和目的地。
- 为应用和 Uvicorn 日志保持一条权威配置路径。
- 在选择文件或队列 handler 之前验证进程和 worker 假设。
- 在缺少必需 settings 时执行启动，确认失败是可操作的。
- 保持脱敏和关联行为在请求和 worker 边界间稳定。
- 在更改全局日志行为之前记录所选策略。

## 不安全默认

- 模块导入期间的 `print`、分散的 `basicConfig` 或 logger 配置。
- 记录整个 Pydantic 模型、请求 body、授权 header 或 provider 响应。
- 在路由、service 和全局 handler 层捕获/记录/重新抛出。
- 无界的 `QueueHandler` 队列或泄漏的 listener 线程。
- 假设轮转文件 handler 对所有 Uvicorn worker 配置安全。

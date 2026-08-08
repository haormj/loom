# NestJS 日志

仅当任务拥有 NestJS 日志基础设施时才使用此参考。Controller、provider、消费者和 job 否则通过已配置的 logger 发出任务拥有的事件，不替换应用级日志。

## Provider 决策

1. 保留仓库当前的 Nest logger provider 和模块配置。
2. 当 Nest 内置 `Logger` 满足已接受的 console 和生命周期行为时使用它。
3. 对于全新的结构化 JSON console 输出，通过已建立的 Nest 集成使用 Pino。
4. 仅当仓库已选择 Winston 或应用拥有的文件轮转是已接受需求时才使用 Winston。

不要同时安装 Pino 和 Winston，也不要在功能 service 中调用 provider 专属 API。在 bootstrap/组合边界一次性绑定 provider。

## 配置归属

将级别、脱敏、serializer、目的地和 provider 选项保持在类型化的已验证配置中。Bootstrap 应对无效的必需设置明确失败，不打印密钥。保留操作服务所需的 Nest system/startup 诊断。

使用稳定的事件名和对象字段。避免将字段扁平化的 JavaScript 字符串插值或序列化完整的请求、响应、DTO、错误或用户对象。

## 关联与边界

在中间件/interceptor 中创建或接受一次关联，并通过所选的请求上下文机制传播。队列消费者、调度器、WebSocket handler 和独立应用建立自己的操作上下文；它们不能假设 HTTP 请求 scope。

在一个任务拥有的边界记录关键状态变更、依赖结果、重试、async 结果和终态失败。异常 filter 拥有最终的意外传输失败。Provider 不捕获、记录和重新抛出 filter 会再次记录的错误。

## 异步与文件输出

仅当已接受时才使用 Pino transport 或所选 provider 的异步机制。限定缓冲，定义丢弃/阻塞行为，保留严重事件，并在 Nest 关闭 hook 期间刷新。

Console JSON 是全新结构化应用的默认选择。应用拥有的文件需要已接受的需求；使用所选 provider 经验证的轮转 transport，配以大小/时间限制、压缩、保留、磁盘限制和目标失败行为。Deploy 不选择或配置此 transport。

## 归属与失败策略

任务必须确定它是否拥有 provider 选择、事件埋点、异步缓冲、文件输出或仅验证。Logger 或 exporter 失败不得静默改变业务结果，除非已接受契约使遥测成为必需依赖。

在 bootstrap 和关闭边界保持 provider 生命周期。功能模块接收仓库 logger 抽象，不独立配置 transport。

## Verification Focus

- 用所选 logger provider bootstrap 真实 Nest 应用或测试模块。
- 捕获一个拥有的事件并断言级别、稳定字段、关联和脱敏。
- 验证一个意外异常仅发出一次，而非由 provider、controller 和 filter 分别发出。
- 当拥有消费者、调度器或 gateway 时执行非 HTTP 上下文传播。
- 当拥有 transport 缓冲或文件时，验证饱和、关闭刷新、轮转、保留和不可用目的地。

## 配置审查

- 在 bootstrap 从已验证配置派生 provider 选项。
- 将 transport 注册和脱敏策略保持在一个组合边界中。
- 分别验证 HTTP、消费者、调度器和关闭生命周期假设。
- 在缺少必需设置时执行启动，确认不打印密钥。
- 保持关联和事件字段在不同 provider 适配器间稳定。
- 在更改应用级 logger 之前记录所选策略。

## 交付证据

记录所选 provider、bootstrap 配置、拥有的事件边界、关联和脱敏断言以及证明所选 logger 行为的运行时命令或聚焦测试。成功的 Nest 编译不能证明 transport 生命周期或事件安全。

## 不安全默认

- 多个 logger provider 或 provider 专属调用分散在功能模块中。
- 完整的 DTO/请求/错误对象传给 logger。
- 仅为关联方便而引入请求范围 logger provider。
- Provider、controller 和 filter 中的重复异常日志。
- 没有保留和磁盘限制的无界 transport 或滚动文件。

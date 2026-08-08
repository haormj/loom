# ASP.NET Core 日志

仅当任务拥有 ASP.NET Core 日志基础设施时才使用此参考。其他任务在各自拥有的边界使用已配置的 `ILogger<T>`，不替换 provider 或应用级设置。

## Provider 决策

1. 保留已有的 provider、配置和 enrichment 约定。
2. 对于全新应用，使用 `Microsoft.Extensions.Logging` 和 `ILogger<T>`，配合 host 内置的 provider。
3. 仅当仓库已选择 Serilog 或 NLog，或已接受的需求需要内置 provider 不具备的机制（例如应用管理的滚动文件）时，才使用 Serilog 或 NLog。

不要注册相互竞争的 console provider，也不要将 Serilog/NLog 的直接 API 混入业务代码。Provider 专属 API 应留在组合根（composition root）中。

## 配置归属

将 category 级别和 provider 选项保存在 `appsettings.json` 以及环境特定的覆盖中。通过现有的 options 模式绑定并验证自定义日志选项。切勿将密钥、连接字符串或仅生产环境的绝对路径放入已提交的默认配置中。

在适当情况下，对稳定的高频事件使用 source-generated 日志方法。优先使用命名的结构化属性而非字符串插值，以便 scope 和 provider 保留字段标识。

## 关联与边界

使用 `Activity`、已接受的 trace 集成和 `ILogger.BeginScope` 进行请求或操作关联。在中间件或所属的 worker 边界中建立一次 scope 并可靠地释放它。仅在 hosted service 和消息消费者之间保留安全的不可变上下文。

仅在任务拥有相关决策的边界处记录关键状态转换、依赖结果、重试、终态失败和意外错误。全局异常处理负责最终的意外 HTTP 错误；下层不应记录并重新抛出同一异常。

## 异步与文件输出

不要仅因为存在日志就添加异步队列。当已接受的需求选择缓冲输出时，使用所选 provider 的有界 sink/queue，并定义溢出和关闭行为。

内置 console provider 仍然是全新应用的默认选择。应用管理的滚动文件需要已接受的文件输出需求以及已存在或经过慎重选择的 Serilog/NLog sink，需具备大小/时间限制、保留策略和路径不可用行为。Deploy 不选择 provider 也不创建保留策略。

## 归属与失败策略

任务必须确定它是否拥有 provider 选择、事件埋点、缓冲、文件输出或仅验证。可选的 logger 或 exporter 失败不得影响业务正确性；必需的安全和恢复事件遵循已接受的过载策略。

将 provider 注册和生命周期保持在 host 组合根中。功能服务使用 `ILogger<T>` 或仓库抽象，不独立配置 sink。

## Verification Focus

- 使用真实的 provider 注册启动应用，并断言不存在重复的 provider。
- 捕获所拥有的事件，验证级别、event id/name、结构化属性、scope 关联和脱敏处理。
- 执行一个预期失败和一个意外失败，以证明级别选择和单边界日志记录。
- 当拥有缓冲或文件输出时，验证溢出、关闭刷新、轮转、保留和目标失败行为。

## 配置审查

- 从已验证的 host 配置和环境覆盖中派生 provider 选项。
- 将 provider 注册、scope、脱敏和生命周期保持在一个组合边界内。
- 分别验证 hosted-service 和请求管道的假设。
- 在缺少必需设置的情况下执行启动，并确认不打印密钥。
- 保持关联和事件字段在不同 provider 适配器间稳定。
- 在更改 host 级日志行为之前记录所选策略。
- 将部署拓扑和 collector 选择排除在应用日志契约之外。
- 添加字段时保留仓库已有的错误和 trace 标识符。

## 交付证据

记录所选的 provider、host 注册、拥有的事件边界、关联和脱敏断言，以及聚焦的 host 或集成证据。成功的 `dotnet build` 不能证明 provider 唯一性、结构化字段或 sink 生命周期。

## 不安全默认

- 注入静态/全局 logger 而非 `ILogger<T>` 或仓库抽象。
- 使用字符串插值事件，丢弃结构化属性名称。
- 在内置 provider 已满足契约的情况下添加 Serilog 或 NLog。
- 日志属性中包含敏感的 claim、header、payload 或异常数据。
- 无界的缓冲或滚动文件，没有保留和磁盘限制。

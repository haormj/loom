# ASP.NET Core 运行时与 Hosting

本参考负责应用运行时配置、hosting、中间件、依赖健康、出站客户端、后台服务和诊断。Docker、Compose、Kubernetes、proxy 拓扑、registry 和部署资产生成仍在 Loom deploy 指引中。

## 配置与选项

使用现有的配置优先级，涵盖基础设置、环境设置、环境变量、user secret/本地覆盖和外部 provider。切勿将生产密钥或环境特定的 URL 放入已提交的默认配置中。

将内聚设置绑定到类型化选项，并在启动时验证必需值：

```csharp
builder.Services
    .AddOptions<ProviderOptions>()
    .BindConfiguration(ProviderOptions.SectionName)
    .ValidateDataAnnotations()
    .Validate(options => Uri.TryCreate(options.BaseUrl, UriKind.Absolute, out _),
        "Provider BaseUrl must be absolute")
    .ValidateOnStart();
```

对稳定设置使用 `IOptions<T>`，对 scoped 重载语义使用 `IOptionsSnapshot<T>`，仅当安全支持实时更新时使用 `IOptionsMonitor<T>`。不要在业务方法中读取分散的配置键。

保持本地默认可运行且生产默认安全。缺少强制依赖应使启动明确失败；可选能力应暴露显式的禁用/降级状态。

## 中间件与 Host 管道

将异常处理放在产生响应的中间件之前，保留仓库的路由、forwarded header、HTTPS、静态文件、CORS、认证、授权、限流、缓存和端点顺序。

仅从已配置的 proxy/网络信任 forwarded header。不要在 TLS 终止和 forwarded proto 未正确配置的拓扑中强制 HTTPS 重定向。

使用优雅关闭并将 `ApplicationStopping`/取消传播到 hosted 工作。从已接受的运行时需求而非教程常量配置请求/body 限制、超时和服务器端点。

## 健康与就绪

当运行时平台同时消费两者时，将轻量级进程 liveness 与依赖感知的 readiness 分开。Liveness 不应因数据库或 provider 暂时不可用而失败；反复重启可能加剧故障。

为 readiness 检查打标签，仅包含服务所声明能力所需的依赖。限定健康检查超时，避免繁重的业务查询、迁移、写入或扇出调用。

根据运行时契约映射健康路径和响应详情。不要公开暴露凭证、内部主机名、异常文本或依赖拓扑。

## 出站 HTTP 与弹性

使用 `IHttpClientFactory`/typed client，配以已验证的 base address、有界超时、认证 handler、序列化和 provider 错误翻译。正确传播取消并释放响应流。

仅当已接受的交互拥有重试、断路、对冲和超时策略时才应用它们。仅重试瞬时/幂等操作或使用显式幂等机制。保持总超时预算和重试次数有界；不要盲目叠加 client、库、proxy 和应用的重试。

通过并发安全的 handler/provider 刷新 token/凭证，并从日志中脱敏 header/body。DNS/handler 生命周期应遵循平台行为，而非每次请求手动创建 `HttpClient`。

Service discovery 或 gateway 路由必须遵循已接受的运行时架构。通过所选平台/client 配置解析逻辑服务名，保持浏览器/公共路径与内部服务地址区分。不要在应用代码中硬编码 Compose 或集群主机名。

## 应用缓存

仅当应用缓存需求拥有真值来源、键维度、TTL/新鲜度、失效、一致性、大小和失败行为时，才添加 `IMemoryCache`、`IDistributedCache`、HybridCache 或 provider 适配器。

当 tenant/authorization/locale/version 维度影响值时，将它们包含在键中。切勿在共享键下缓存凭证、不安全变更或用户特定数据。分布式缓存丢失应遵循已接受的降级策略；它不得成为第二个真值来源。

HTTP 输出缓存是独立的传输关注点。不要仅因为端点有 `Cache-Control` 或 ETag 语义就添加 Redis。

## 后台服务

仅对已接受的作业、消费者或维护工作实现 `BackgroundService`/`IHostedService`。当需要 scoped 依赖时，每次迭代/消息创建 service scope。

遵守 `stoppingToken`，处理部分失败，并定义重试/死信/幂等/并发行为。当调度器或 broker 拥有时间控制时，避免无界的内存队列和任意 `Task.Delay` 循环。

启动不得在测试、design-time 迁移或热重载下启动重复 worker。关闭应停止摄入、完成/取消有界工作并干净地关闭 client。

## AOT、Trimming 与序列化

仅当所选运行时和依赖支持时才启用 native AOT/trimming。验证重度依赖反射的序列化器、DI 扫描、验证器、EF provider 行为、OpenAPI 生成、动态代理和配置绑定。

在需要时使用 source-generated 序列化/metadata，并在声称 AOT/trimming 支持之前运行发布产物，而非仅 `dotnet build`。

## 验证

- 使用有效和无效的必需选项启动应用，并断言预期的启动结果。
- 当变更时通过真实 host 验证中间件顺序、forwarded-header、CORS/auth、限制和异常行为。
- 探测 liveness/readiness 和依赖状态转换，不暴露内部信息。
- 使用可控的 provider 测试出站超时、取消、瞬时映射和重试/幂等边界。
- 当拥有缓存时，验证缓存键隔离、失效/新鲜度、miss/降级行为和真值来源恢复。
- 验证后台 service scope、关闭、重复预防和失败处理。
- 验证结构化遥测字段，并对 AOT/trimming 声明运行发布产物。

## 交付证据

标识运行时边界以及证明它的启动/host/probe/client/worker 断言。配置文件、注册的健康检查或成功构建不能证明优先级、中间件顺序、依赖状态转换、取消、关闭或发布运行时兼容性。

## 不安全默认

- 在应用运行时指引中重复 Docker/Kubernetes 资产。
- 密钥或生产 URL 提交到 `appsettings.json`。
- Liveness 中包含依赖检查。
- 每次请求创建 `HttpClient` 或对非幂等写入添加重试。
- Hosted service 忽略取消或从根 provider 解析 scoped service。
- 敏感/高基数数据作为遥测 label 输出。
- 仅从编译声称 AOT/trimming 而未运行发布产物。

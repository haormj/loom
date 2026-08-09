# ASP.NET Core Minimal API

仅当所选技术栈和仓库使用该端点风格时，才通过 Minimal API 实现已接受的 HTTP 接口。保留方法、路径、schema、状态、错误、认证策略和暴露规则，而非围绕框架示例重新设计 API。

## 端点组织

通过 `MapGroup` 或聚焦的 `Map...Endpoints` 扩展对内聚端点进行分组。有效路径结合应用基础路径、group 前缀和端点模式；避免重复或缺失 `/api` 段。

```csharp
public static IEndpointRouteBuilder MapOrderEndpoints(this IEndpointRouteBuilder routes)
{
    var orders = routes.MapGroup("/api/orders").WithTags("Orders");

    orders.MapPost("/", CreateOrder)
        .WithName("CreateOrder")
        .Produces<OrderResponse>(StatusCodes.Status201Created)
        .ProducesValidationProblem()
        .ProducesProblem(StatusCodes.Status409Conflict);

    orders.MapGet("/{id:guid}", GetOrder)
        .Produces<OrderResponse>()
        .ProducesProblem(StatusCodes.Status404NotFound);
    return routes;
}
```

当路由归属变得难以检查时，不要将所有端点放在 `Program.cs` 中。除非仓库有明确的边界，否则不要在一个能力中混用 controller 和 Minimal API。

## 绑定与 DTO

使用显式路由约束和类型化的 path/query/header/body 参数。`[AsParameters]` 可以分组 query 值，但结果类型必须保持默认值、可空性、验证和 OpenAPI 形态与契约一致。

使用请求和响应 record/class，而非 EF/domain 实体。将服务端拥有的 actor、tenant、状态、审计和生成字段排除在客户端可写模型之外。

使用已确立的验证策略：端点过滤器、FluentValidation 集成、data annotation 或应用验证。传输验证不替代唯一性、归属、状态转换或并发数据库约束。

## 类型化结果与状态语义

当类型化结果能改善编译时响应元数据时优先使用：

```csharp
static async Task<Results<Ok<OrderResponse>, NotFound, ProblemHttpResult>> GetOrder(
    Guid id,
    IOrderQueries queries,
    CancellationToken ct)
{
    var order = await queries.Find(id, ct);
    return order is null ? TypedResults.NotFound() : TypedResults.Ok(order);
}
```

当创建可检索资源时返回带已接受 location 的 `Created`。保持 `204` 无 body。保留已声明的 header，如 ETag、Location、Retry-After、分页链接或幂等结果。

通过 `IExceptionHandler`、Problem Details 或已确立的端点边界一次性转换预期的应用失败。避免每个端点都有 catch-all 过滤器，切勿暴露堆栈跟踪、SQL 消息、token 错误或内部类型名。

## 依赖注入与取消

通过 DI 绑定应用 service/handler 和受信任的当前用户抽象。端点 delegate 应执行传输映射并调用一个应用操作，而非查询 `DbContext`、协调外部调用或拥有事务。

接受 `CancellationToken` 并通过 EF Core、HTTP client、stream 和应用 handler 传播它。不要将取消转换为通用 `500` 或在继续副作用时吞掉它。

## 路由元数据与 OpenAPI

当仓库发布契约时，使用 name、tag、summary、`Produces`、认证需求和 OpenAPI 元数据。元数据必须反映运行时行为；`.Produces(404)` 不会让端点返回 not found。

不要从约定添加 Swagger UI、API versioning 或开发服务器 URL。这些是 bootstrap/API 契约决策，应保持环境感知。

## 过滤器、中间件与策略

端点过滤器适用于端点局部验证或可复用的传输行为。认证/授权应使用 ASP.NET policy 和端点/group 元数据。全局异常处理、CORS、限流、输出缓存和请求日志属于应用管道。

保持中间件顺序的慎重性，避免在过滤器中重新实现中间件行为。Route-group 授权仅可通过显式已接受策略来收窄或公开。

## 集合与条件请求

对分页大小和过滤/排序字段使用有界和允许列表。使用确定性排序和稳定的响应元数据。空集合通常返回成功的集合形态。

仅在声明时实现 ETag/If-Match/缓存语义。对于认证的、租户特定的、可变的或用户变化的响应，输出缓存默认不安全。

## 验证

- 使用 `WebApplicationFactory<Program>` 或仓库的真实测试 host 来验证路由注册和全局管道行为。
- 断言精确的成功/错误状态、body、header、绑定、验证和响应字段排除。
- 测试接口拥有的 not-found、冲突、认证、授权、取消、分页和条件行为。
- 仅当发布的契约变更时验证路由 name/OpenAPI。
- 构建所属项目以确保 delegate 签名和类型化结果 union 编译通过。

## 交付证据

命名有效路由和证明绑定、类型化结果、header 和相关策略的测试 host 请求。直接调用静态端点方法不能证明 route group、中间件、全局 handler、认证、验证过滤器或 OpenAPI 元数据。

## 不安全默认

- 为基于 controller 的技术栈加载 Minimal API 指引。
- 端点 delegate 中包含 EF Core 查询和业务转换。
- 直接返回 Domain/EF 实体。
- 将状态元数据当作运行时行为。
- 在没有已接受契约的情况下添加 Swagger、versioning 或 `/api` 前缀。
- 跨 I/O 调用忽略 `CancellationToken`。
- 对个性化或变更响应启用输出缓存。

# Spring Boot 外部服务集成

本参考负责外部客户端/适配器构造、provider 契约映射、认证传播、失败转换和适配器测试边界。HTTP 特定规则仅在已接受的交互协议为 HTTP 时适用。Spring Cloud discovery/gateway 和弹性策略是独立关注点。

## 客户端选择

使用与应用技术栈对齐的客户端：

| 应用形态 | 客户端 |
|---|---|
| Spring MVC/阻塞 service | `RestClient` 或仓库已建立的阻塞客户端 |
| WebFlux/响应式 service | 具有非阻塞组合的 `WebClient` |
| gRPC service | 包装在应用拥有端口后面的生成 stub/channel |
| 已有生成/provider SDK | 包装在应用拥有适配器后面的 SDK |

不要仅为获得 `WebClient` 而向阻塞应用添加 WebFlux。不要从响应式请求路径调用 `.block()`。阻塞 worker 可以使用具有显式连接/读取超时的阻塞客户端。

## 类型化客户端边界

将 provider 传输保持在 domain/application 逻辑之外。

```java
@Component
final class PricingHttpClient implements PricingGateway {
    private final RestClient client;

    PricingHttpClient(RestClient.Builder builder, PricingClientProperties properties) {
        this.client = builder.baseUrl(properties.baseUrl().toString()).build();
    }

    @Override
    public PriceQuote quote(QuoteRequest request) {
        ProviderQuote body = client.post()
            .uri("/quotes")
            .body(ProviderQuoteRequest.from(request))
            .retrieve()
            .body(ProviderQuote.class);
        return body.toDomain();
    }
}
```

使用与内部 domain 和公共 API DTO 分离的 provider DTO。将 base URL、凭证、超时、payload 限制和可选 proxy 设置保留在已验证的类型化配置中。

## HTTP 行为

对于 HTTP 交互，定义：

- 精确的相对路径和 method
- 请求/响应 media type 和字符集
- provider 状态到 domain 错误的映射
- 连接、响应和整体操作时间预算
- 最大响应/body 缓冲
- 认证/header 传播
- 选中时的关联/请求标识符
- 相关时的重定向和压缩行为

默认不要记录完整 provider payload。脱敏凭证和敏感字段。当契约需要数据时拒绝意外的成功空 body。

将 provider 错误映射到稳定的应用异常，如 not found、rejected、rate limited、unavailable、timeout 或无效 provider 响应。保留支持所需的安全 provider 代码，而不向调用者泄漏原始 payload。

## 非 HTTP Provider 适配器

对于生成的 SDK 和 gRPC，将生成/provider 类型保留在适配器边界。一次性配置 channel/client 生命周期，应用已接受的 deadline 和消息限制，将 provider 状态码翻译为调用者使用的相同应用失败模型。不要通过 domain/application 接口暴露 stub、channel、SDK session 或 provider 异常。

事件和 job 交互需要其选定的消息/异步边界来处理投递、确认、排序、重复处理和持久化。不要仅为复用 `RestClient`/`WebClient` 指引而将它们建模为 HTTP 客户端。

## 认证传播

仅当信任模型需要委派时才传播终端用户凭证。Service 凭证、OAuth client 凭证、API key 和签名请求需要独立归属和轮转。不要将每个入站 header 转发给下游服务。

将 token 获取/缓存保留在安全感知的客户端组件中，而非每个业务 service 方法中。

## 响应式客户端行为

对于 `WebClient`，将副作用保持在管道内并在 body 解码之前映射状态。仅通过所选弹性策略应用重试。限定大响应聚合，仅当已接受的接口支持时才使用流。

## Verification Focus

使用协议合适的 fake 或测试服务器（如 HTTP 的 MockWebServer/WireMock）证明：

- 序列化的请求 method/path/body/header
- 非 HTTP 适配器的操作/消息和 provider 状态映射
- 成功的响应映射
- provider 验证/not-found/冲突响应映射
- 格式错误或意外 payload 行为
- 连接/响应超时行为
- 认证和关联传播
- 拥有时的响应大小或流行为
- 不存在环境特定的硬编码值

## 不安全默认

- 为每个请求构建新客户端。
- 硬编码 provider URL 或凭证。
- 通过产品 API 返回 provider DTO。
- 将所有入站 header 转发给下游。
- 在没有已接受操作策略的情况下在客户端内重试。
- 将 provider 错误吞入 `Optional.empty()` 或假成功数据。

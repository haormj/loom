# Spring Cloud 应用集成

此参考仅在 Technical Baseline 选择 Spring Cloud 且任务拥有 Config、服务发现、Gateway 或客户端负载均衡时适用。通用出站 HTTP 客户端和弹性策略有独立参考。

## 能力门控

不要因为 Spring Cloud 依赖可用就引入 Spring Cloud 组件。每个组件需要已接受的运行时或集成角色：

| 组件 | 所需当前能力 |
|---|---|
| Config Client/Server | 中心配置归属和运行时可用性契约 |
| Discovery Client/Registry | 逻辑服务查找和注册生命周期 |
| Gateway | 公共/内部路由归属、重写、安全和失败边界 |
| LoadBalancer | 基于发现的客户端选择配以已定义健康/重试行为 |

## Spring Cloud Config

对于 Config 客户端，定义：

- application/profile/label identity
- 认证和传输安全
- fail-fast 与本地回退
- 重试分类和有界尝试
- 配置不可用时的启动行为
- 可刷新与需重启的设置

默认不要暴露宽泛的 refresh 端点。动态刷新可以在请求活跃时替换 bean 状态；仅对证明可安全刷新的属性使用它。密钥和结构性 datasource/安全变更通常需要更强的轮转/重启行为。

Config Server 是独立的运行时角色。不要在仅需外部化属性源的应用任务中创建它。

## 服务发现

仅当发现是已接受的边界时才使用逻辑服务名。定义注册名、元数据、健康状态、租约行为、查找失败和本地/测试替代。

不要在基于发现的客户端旁硬编码实例 URL。不要将注册表条目视为实例可以服务所需端点的证明。注册、readiness 和客户端健康语义必须一致。

当所选 hosting 环境已提供服务发现时避免添加注册表服务器。

## Gateway 路由

Gateway 路由必须保留已接受的公共 API 契约：

- 路由 id 和归属
- host/path/method 谓词
- 精确前缀剥离/重写行为
- 认证和受信任 header 处理
- CORS 边界
- 需要时的请求/响应大小限制
- 已接受时的限流策略
- 下游超时和失败映射

```java
@Bean
RouteLocator orderRoutes(RouteLocatorBuilder routes) {
    return routes.routes()
        .route("orders", route -> route
            .path("/api/orders/**")
            .filters(filters -> filters
                .stripPrefix(1)
                .removeRequestHeader("X-Internal-Actor"))
            .uri("lb://orders-service"))
        .build();
}
```

将路径重写视为契约行为。测试外部可见路径和下游路径。切勿信任调用者提供的 identity header，除非受信任的 gateway 替换并签名/控制它们。

除非已接受的操作是重试安全的否则 Gateway 重试禁用。不要重试任意 `POST`、`PATCH` 或状态转换流量。回退不得伪造成功的业务数据。

## 负载均衡

客户端负载均衡需要基于发现的服务实例和健康模型。在集成/弹性层保留请求超时和重试归属。在没有尝试预算的情况下不要叠加 gateway 重试、客户端重试和库重试。

## 本地与测试行为

提供确定性的本地/测试路径，除非任务显式验证那些运行时角色否则不需要共享注册表或 config server。在所属边界使用测试配置、静态服务实例或 mock 的发现/config 客户端。

## Verification Focus

有用的 Spring Cloud 证据包括：

- 可用和不可用源行为的 Config 启动
- profile/label/属性优先级和可安全刷新的属性
- 发现注册和查找失败行为
- Gateway 谓词和路径重写测试
- 跨 gateway 的安全/header/CORS 行为
- 没有假成功的下游不可用映射
- 非幂等操作不被重试的确认
- 没有不相关共享云基础设施的本地/测试启动

## 不安全默认

- 因为依赖存在就创建 Config Server、Eureka 或 Gateway。
- 启用发现定位器 catch-all 路由。
- 对带凭证流量使用通配符 CORS。
- 信任入站 identity header。
- 每条路由重试三次。
- 从回退返回假 domain 数据。

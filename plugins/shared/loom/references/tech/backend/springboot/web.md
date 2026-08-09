# Spring Boot Web 实现

以已接受的 HTTP 接口为方法、路径、请求形态、响应形态、状态码和错误行为的权威。本参考负责它们的 Spring MVC 或 WebFlux 实现，而非 API 重新设计。

## Web 技术栈边界

在添加代码之前确认所选基线和仓库使用哪个 Web 技术栈：

| 技术栈 | Controller 形态 | 客户端/测试形态 | 持久化边界 |
|---|---|---|---|
| Spring MVC | 同步 controller 方法和 servlet filter | `MockMvc`、`RestClient`、servlet 安全 | controller 之外允许阻塞 repository |
| Spring WebFlux | `Mono`/`Flux` controller 方法和响应式 filter | `WebTestClient`、`WebClient` | 阻塞调用需要显式有界调度器；响应式持久化优先使用 R2DBC |

不要为一个端点向 Spring MVC 应用添加 WebFlux。不要在响应式请求链中调用 `.block()`。不要仅为包装阻塞 service 调用而返回 `Mono`。

## Controller 边界

Controller 拥有传输工作：

- 绑定 path、query、header 和 body 输入
- 触发 Jakarta Validation
- 通过已建立的安全机制解析已认证调用者数据
- 调用一个 application/service 操作
- 将结果映射到已接受的响应和状态

Controller 不拥有 repository 访问、事务边界、状态转换规则、跨记录验证或下游重试循环。

```java
@RestController
@RequestMapping("/api/orders")
final class OrderController {
    private final OrderApplicationService orders;

    OrderController(OrderApplicationService orders) {
        this.orders = orders;
    }

    @PostMapping
    ResponseEntity<OrderResponse> create(@Valid @RequestBody CreateOrderRequest request) {
        OrderResponse created = orders.create(request);
        URI location = ServletUriComponentsBuilder.fromCurrentRequest()
            .path("/{id}").buildAndExpand(created.id()).toUri();
        return ResponseEntity.created(location).body(created);
    }
}
```

保留从生产源码或构建元数据中发现的真实 Spring Boot 基础包。将 controller、advice、配置和应用 service 保持在该包树下；切勿将 `com.example` 等教程根引入生产代码。

## DTO 与验证边界

对外部输入使用请求 DTO，对输出使用响应 DTO 或读模型。不要序列化 JPA 实体、懒代理、内部版本字段、凭证字段或双向关系。

```java
public record CreateOrderRequest(
    @NotBlank String supplierName,
    @NotEmpty List<@Valid OrderLineRequest> lines
) {}
```

将 `@NotBlank`、`@Size`、`@Pattern` 和嵌套 `@Valid` 等注解应用于传输形态规则。将唯一性、归属、生命周期资格、授权、库存和跨记录规则保留在所属的 application/domain service 中。查询 repository 的自定义验证器不能替代数据库约束，除非写入路径也处理约束违规否则容易受到竞态影响。

仅当 create/update 契约确实不同且分离 DTO 会造成更多歧义时才使用验证组。避免将 JPA 实体上的注解作为唯一 API 验证契约。

## 错误转换

使用一个与已接受 API 错误契约对齐的 `@RestControllerAdvice`。Spring Boot 3 支持 `ProblemDetail`；当已有项目 envelope 时保留它。

```java
@RestControllerAdvice
final class ApiExceptionHandler {
    @ExceptionHandler(OrderStateConflict.class)
    ResponseEntity<ProblemDetail> handleConflict(OrderStateConflict error) {
        ProblemDetail problem = ProblemDetail.forStatus(HttpStatus.CONFLICT);
        problem.setTitle("Order cannot change state");
        problem.setDetail(error.userMessage());
        problem.setProperty("code", error.code());
        return ResponseEntity.status(HttpStatus.CONFLICT).body(problem);
    }
}
```

当已接受的接口声明时，分别转换验证、not-found、业务冲突、认证、授权、临时依赖失败和意外失败。切勿暴露 SQL 文本、持久化异常名、token 解析失败、类名、堆栈跟踪或文件路径。

当接口声明可重试的可用性响应时，保留其精确状态集，仅当应用能提供有意义的延迟时才发出 `Retry-After`。此调用者可见的策略不授权内部重试循环；下游重试行为需要独立的弹性边界。

不要将预期的验证和 not-found 结果记录为服务器错误。在有请求关联数据的边界处一次性记录意外失败。

## 集合端点

对于无界集合，使用已接受的分页/过滤契约配以确定性排序。保持 controller 默认和最大页大小与 service/query 行为对齐。当 API 契约定义了稳定的响应模型时，避免返回 Spring `Page` 内部；显式映射页面内容和元数据。

在构造查询之前对可排序/可过滤字段验证允许列表。空列表是成功的集合响应，除非 API 契约声明其他行为。

## HTTP 缓存与条件请求

在传输/读模型边界实现已接受的 HTTP 缓存行为。与接口契约一致地使用 `Cache-Control`、`ETag` 和 `Last-Modified`，并在序列化完整响应之前评估条件请求。

对于条件读，仅当当前验证器匹配时返回 `304` 且无响应 body。对于并发敏感的写入，保持 `If-Match`/版本检查与已接受的陈旧更新状态以及 service 或持久化版本边界对齐。可能错过有意义状态变更的弱时间戳或哈希不是安全的验证器。

HTTP 验证器和 cache-control header 不需要 Spring Cache、Redis 或应用对象缓存。仅当独立的应用缓存需求拥有真值来源、键、新鲜度、失效和失败行为时才添加 `@Cacheable`。

## CORS 与浏览器绑定

CORS 属于实际的浏览器/运行时边界。当前端和 API 共享 origin 时使用无 CORS 的同源路由。对于跨域客户端：

- 外部化允许的 origin
- 使用显式 method 和 header
- 切勿将带凭证的请求与通配符 origin 组合
- 保持 Spring Security CORS 配置和 MVC/WebFlux CORS 配置一致
- 对受保护的写入操作测试 preflight 行为

不要将单一开发 origin 硬编码为生产规则。

## HTTP 客户端分离

将出站 HTTP 客户端保持在 controller 之外。`RestClient` 和 `WebClient` 配置、provider 错误转换、认证传播和超时归属属于 Spring Boot 集成参考。WebFlux 操作符和背压行为在选中时属于 Java 响应式参考。

## Verification Focus

有用的 Web 证据包括：

- 聚焦的 MVC 或 WebFlux 测试证明已接受的成功状态和响应 DTO
- 具有稳定错误形态的验证和业务阻塞响应
- 通过真实组件扫描根的 controller 发现
- 当拥有时的列表过滤、确定性排序、分页和空结果
- 接口声明时的 cache-control、验证器、`304` 和陈旧更新行为
- 存在跨域浏览器边界时的 CORS preflight 行为
- 响应路径中不存在实体/懒代理序列化

## 不安全默认

- 因为教程使用版本化路径就添加 `/v1`。
- 从 controller 直接返回实体。
- 在每个 controller 中捕获每个异常。
- 在 controller 方法中执行 repository 写入或外部调用。
- 在 controller 中重试出站请求。
- 将 HTTP 验证器视为添加应用缓存的理由。
- 全局启用宽松 CORS。
- 在没有已接受边界的情况下混合 servlet 和响应式 Web 技术栈。
- 在 `com.example` 或 `org.example` 下创建生产 controller。

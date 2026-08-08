# Spring Boot 安全实现

实现当前接口和架构已拥有的认证和授权策略。当受保护操作不是任务的一部分时，不要引入 JWT、OAuth2、session、role、refresh token 或账户管理。

当 bearer JWT profile 激活时，已接受的 JWT 算法、issuer、audience、token 类型和密钥来源契约在 `tech/api/jwt.md` 中。本文件仅负责 Spring Security 配置和框架特定验证。`server_session` profile 是独立的 cookie 支持的 session 路径，不得实现为 JWT。

## 安全机制边界

从已接受的基线和已有仓库中选择机制：

| 客户端/信任模型 | 典型 Spring Security 形态 |
|---|---|
| 同源浏览器 session | Session 认证、安全 cookie、CSRF 保护 |
| 无状态 bearer-token API | OAuth2 Resource Server 配以 issuer/JWK 验证 |
| 服务间 API | Resource Server、mTLS 感知基础设施或已有 gateway identity |
| 内部未认证运行时 | 仅在已接受时显式公共路由；无占位认证系统 |

当需求是 issuer/JWK 或签名 bearer token 时，优先使用 Spring Security 的 OAuth2 Resource Server 支持而非自定义 JWT 解析 filter。仅当已有 token 契约 Resource Server 无法表示时才使用自定义 filter。

## Filter Chain

使用 Spring Security 6 `SecurityFilterChain` 和显式路由归属。

```java
@Bean
SecurityFilterChain apiSecurity(HttpSecurity http) throws Exception {
    return http
        .securityMatcher("/api/**")
        .authorizeHttpRequests(authorize -> authorize
            .requestMatchers(HttpMethod.GET, "/api/catalog/**").hasAuthority("catalog:read")
            .requestMatchers(HttpMethod.POST, "/api/orders/**").hasAuthority("orders:write")
            .anyRequest().authenticated())
        .oauth2ResourceServer(oauth -> oauth.jwt(Customizer.withDefaults()))
        .build();
}
```

保持 matcher 顺序从特定到一般。避免宽泛 `permitAll`、意外阻止健康/静态资源的 catch-all 保护，以及具有重叠 matcher 的并行 filter chain。保留已建立的 role 与 authority 约定。

不要习惯性禁用 CSRF。不使用 cookie 认证的无状态 bearer-token API 可以慎重禁用它。浏览器 session 和基于 cookie 的认证需要 CSRF 保护。在配置或测试中记录实际的客户端机制。

对于 `server_session` profile，从服务端 session 存储加载已认证 principal，将其稳定用户 id 和 role 映射为 Spring authority，并用 CSRF 验证保护不安全的浏览器请求。不要为此 profile 创建 OAuth2 resource-server filter、bearer token 解析器、issuer/audience 配置或 JWT 算法配置。当已接受的运行时依赖声明 `session` 能力时，Redis 是 session 存储；session 生命周期和 cookie 设置仍是后端任务拥有的框架/配置关注。

## 认证材料

- 使用仓库所选的自适应编码器存储密码；切勿明文或可逆加密。
- 外部化 issuer、audience、JWK 位置、客户端凭证、签名材料、token 生命周期和允许的时钟偏差。
- 将所选的 `tech/api/jwt.md` 契约映射到 Spring 的 resource-server 验证；不要在框架配置中放宽算法或 claim。
- 切勿记录 bearer token、refresh token、密码、授权 header 或解码的敏感 claim。
- 将生产凭证排除在默认应用配置之外；允许占位符和环境绑定。

自定义 refresh-token 流程需要独立的 token 类型、持久化或撤销语义、轮转/重用检测、过期、登出行为和盗窃响应。清除 `SecurityContextHolder` 不会撤销无状态 token。

## 授权边界

路由授权保护端点类。方法授权保护可从多个入口点到达的业务操作。

```java
@PreAuthorize("hasAuthority('orders:approve') and @orderAccess.canApprove(#orderId, authentication)")
public OrderResponse approve(UUID orderId) {
    return orders.approve(orderId);
}
```

将资源归属和生命周期资格保留在授权/domain service 中，而非在 SpEL 中嵌入 repository 查询。UI 可见性不是授权。当 job、消息或其他 service 可以调用同一操作时，仅 controller 检查是不够的。

区分：

- 缺失或无效认证：`401`
- 缺乏权限的已认证调用者：`403`
- 受保护资源存在性：根据已接受策略隐藏或暴露
- 授权后的业务不合格：domain 冲突或验证响应

使用 `AuthenticationEntryPoint` 和 `AccessDeniedHandler` 产生已接受的安全错误 envelope。不要暴露解析器异常、账户存在性、内部 role 映射或堆栈跟踪。

## 当前用户解析

优先使用仅包含稳定 identity 和 authority 的不可变应用面向 principal。不要通过 domain 代码传递 `HttpServletRequest` 或 Spring Security 内部。仅在操作需要时加载当前可变用户状态。

对于响应式应用，使用响应式安全上下文 API；ThreadLocal 假设不能安全跨响应式边界。

## CORS 与安全

与 Web 边界共享一个显式 CORS 策略。带凭证的浏览器请求需要显式 origin。Preflight 必须到达 CORS/安全 filter 而不意外允许受保护操作本身。

## Verification Focus

有用的安全证据包括：

- filter-chain 上下文启动
- 每个变更的受保护策略的一个允许请求和一个拒绝请求
- 当拥有时的缺失、格式错误、过期、错误 issuer/audience 和权限不足的 token 行为
- 方法级资源归属拒绝
- 不暴露密钥的密码哈希和无效凭证行为
- 匹配 session 或 bearer-token 客户端风格的 CSRF 行为
- 真实浏览器 origin 边界的 CORS preflight
- 稳定的 `401` 和 `403` 响应形态

## 不安全默认

- 从教程复制手写 JWT filter 和 token service。
- 将无状态登出视为仅 `SecurityContextHolder.clearContext()`。
- 在没有 token 类型强制的情况下将 access-token 解析器复用于 refresh token。
- 在不识别客户端模型的情况下禁用 CSRF 并启用带凭证的 CORS。
- 硬编码 localhost origin 或签名密钥。
- 用堆栈跟踪记录每个无效 token。
- 在已接受阶段未认证时添加 role 表和认证端点。

# ASP.NET Core 认证与授权

实现已接受的 identity 和授权模型。ASP.NET Core 支持 cookie、bearer token、Identity、外部 provider、证书和自定义 scheme；除非架构和客户端信任模型选择了 JWT 或 Identity，否则不要引入它们。

已接受的 JWT 算法和 claim 契约在 `tech/api/jwt.md` 中；本文件负责 ASP.NET Core 的 scheme、policy、中间件和选项配置。

## Scheme 选择与配置

当存在多个 scheme 时，配置显式的默认 authenticate/challenge/forbid scheme。通过 policy scheme 或端点元数据将 cookie 和 bearer 端点分开；不要让 scheme forwarding 意外接受较弱的凭证。

通过已验证的选项绑定 issuer、audience、authority、密钥、生命周期、cookie 设置和 provider 凭证。切勿提交签名密钥或记录 token。将 `UseAuthentication` 放在 `UseAuthorization` 之前，并在两者之后映射受保护的端点。

对于 JWT bearer 验证，将所选的 `tech/api/jwt.md` profile 绑定到显式的 bearer 选项和 policy。当已接受系统信任外部 identity provider 时，不要生成本地自签发的 token。

## Identity 与密码工作流

使用 ASP.NET Core Identity 或现有维护的凭证边界来处理密码哈希、锁定、确认、重置、MFA、恢复和 security-stamp 行为。当 Identity 已拥有凭证时，不要按照教程实现自定义的 PBKDF/密码格式。

注册、登录、验证、重置和账户恢复是独立的任务拥有工作流。避免账户枚举，在需要时使用过期的单次 token，切勿在普通 DTO 中返回密码哈希、重置 token、refresh token 或 security stamp。

Refresh/session 工作流需要显式的轮转、撤销、重用检测、设备/session 归属和登出语义。长期存活的 bearer token 不是完整的 refresh 设计。

## 策略与资源授权

对可复用的 claim/role/requirement 组合使用命名策略。对于依赖应用事实而非端点 delegate 中的分支逻辑的策略，优先使用自定义 `IAuthorizationRequirement` handler。

```csharp
builder.Services.AddAuthorization(options =>
{
    options.AddPolicy("CanApproveOrders", policy =>
        policy.RequireAuthenticatedUser()
              .AddRequirements(new OrderApprovalRequirement()));
});
```

Role 和 claim 确立粗粒度能力；它们不能证明行/租户归属或有效的生命周期状态。对列表查询进行范围限定，并在应用边界强制执行资源/操作授权。当需要已加载的资源时，使用 `IAuthorizationService.AuthorizeAsync(user, resource, policy)`。

保持 fallback/default policy 行为明确。匿名端点必须被慎重标记和审查，尤其是在 route group 共享授权元数据时。

## 当前用户边界

通过窄的请求上下文抽象或端点 `ClaimsPrincipal` 读取 identity，然后将稳定的 actor/tenant 标识符传入应用操作。Domain 和基础设施代码不应直接依赖 `IHttpContextAccessor`，除非请求上下文是其已接受的职责。

验证 claim 的存在和格式；切勿因为某个 issuer 通常提供 `sub`、tenant、role 或 email claim 就假设它们存在。当挂起/撤销需要当前数据时，将外部 identity 映射到内部账户状态。

## 浏览器控制

Cookie 认证的不安全请求需要 antiforgery 保护。为真实的部署拓扑配置 secure、HTTP-only、SameSite、domain/path 和过期行为。

CORS 控制浏览器 origin 访问，不是认证。使用显式的 origin/method/header，切勿对带凭证的 origin 使用通配符。保留 forwarded-header/proxy 信任，以便 HTTPS 和 secure-cookie 决策不会被伪造。

不要将 bearer token 移至不安全的浏览器存储，也不要为绕过前端集成问题而禁用 antiforgery/CORS。

## 错误与披露策略

保留 `401` challenge 与 `403` forbid 行为以及已接受的 problem envelope。错误归属者的资源可能有意返回 not found；在 list/detail/mutation 路径中一致地应用该策略。

从日志/trace 中脱敏凭证、token、诊断不需要的 claim、cookie 和授权内部信息。生产错误不得暴露验证密钥、加密细节、数据库消息或堆栈跟踪。

## 验证

- 通过真实的中间件管道执行允许、未认证、无效/过期 token、禁止策略、错误归属者/租户和显式匿名路径。
- 将列表隔离与资源授权和变更策略分开测试。
- 仅当拥有这些工作流时，才验证 issuer/audience/key/lifetime 处理和 refresh/撤销。
- 通过真实响应序列化断言密码哈希和敏感字段排除。
- 为所选的浏览器拓扑测试 cookie/antiforgery/CORS/proxy 行为。
- 验证缺少强制 auth 选项时的启动失败，且不暴露密钥。

## 交付证据

命名所选的 scheme、policy/resource 边界、受保护操作以及允许和拒绝的 HTTP 断言。仅凭解码的 token、mock 的 `ClaimsPrincipal` 或端点元数据检查不能证明中间件顺序、scheme 选择、归属隔离、撤销或密钥脱敏。

## 不安全默认

- 因为 .NET 示例使用了 JWT 或 Identity 就添加它们。
- 签名密钥和 token 选项从未验证的字面量读取。
- 将 role 当作对象/租户授权。
- 在 domain/application 代码中到处注入 `IHttpContextAccessor`。
- 发放 refresh token 而没有存储、轮转或撤销策略。
- 为让集成通过而禁用 cookie antiforgery 或 CORS。
- 隐藏在宽泛 route group 中的匿名端点例外。

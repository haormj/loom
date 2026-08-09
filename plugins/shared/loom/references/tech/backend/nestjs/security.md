# NestJS 认证与授权

仅实现已接受的 identity 和授权模型。NestJS guard、Passport strategy、JWT helper 和元数据是机制；它们不决定产品是否使用 session、bearer token、gateway identity、API key 或其他信任边界。

已接受的 JWT profile 和 API 拒绝行为在 `tech/api/jwt.md` 中；本文件负责 NestJS guard、Passport strategy 配置和策略组合。

## 认证边界

保留所选的机制和仓库约定：

| Identity 来源 | NestJS 职责 |
|---|---|
| 同源 session/cookie | 验证 session，保留 CSRF 和安全 cookie 行为 |
| Bearer token | 验证受信任的 issuer、audience、签名、过期和 token 类 |
| Gateway/service identity | 仅在已配置边界信任已验证的 proxy/service 断言 |
| 公共操作 | 显式元数据/路由策略，窄范围限定 |

当 Passport 的 strategy 生命周期匹配应用时使用它。对于简单的 gateway 或 service-token 验证，聚焦的自定义 guard 可能更清晰。不要仅因为框架支持就添加 Passport/JWT。

## Guard 与元数据组合

认证建立 identity；授权决定该 identity 是否可以执行操作。即使 guard 被组合，也保持这些决策分离。

用 `Reflector.getAllAndOverride` 或已建立的合并策略读取 public/role/permission 元数据，使 handler 级决策可预测地覆盖 controller 默认值。为元数据 key 使用稳定的 symbol/常量，保持 `@Public` 例外可审查。

全局 `APP_GUARD` provider 仅当每个公共例外都显式时才默认保护。当 guard 需要配置或 service 时，通过依赖注入注册它们，而非在 bootstrap 中手动构造。

```typescript
const isPublic = this.reflector.getAllAndOverride<boolean>(IS_PUBLIC_KEY, [
  context.getHandler(),
  context.getClass(),
]);
if (isPublic) return true;
return super.canActivate(context);
```

使用仓库的元数据 key 和优先级约定。代码片段演示确定性的 method/controller 解析，而非规定 JWT。

仅 role 检查不足以进行 tenant、owner、关系或对象状态授权。限定 repository 查询范围并在应用边界强制执行操作策略，使列表、详情、变更、后台和非 HTTP 入口点保持一致。

## JWT 与 Token 生命周期

将所选的 `tech/api/jwt.md` profile 映射到已验证的启动配置和仓库的 Passport/resource-server strategy。切勿在没有策略的情况下接受来自不受信任 token header 的算法或密钥。

根据已接受的安全契约保持 access token 短生命周期。Refresh token 需要显式的轮转、重用检测、存储/哈希、撤销、设备/session 处理和登出语义。不要发放长期签名的 token 并称之为 refresh 工作流。

Claim 应包含稳定的授权输入，而非敏感的 profile 数据或假设无限期保持最新的可变事实。在契约要求撤销或挂起及时生效的地方验证 subject/account 状态。

## 密码与账户流程

使用已建立的密码 hasher，配以已配置的成本和常数时间验证行为。切勿通过普通用户 DTO 存储或返回明文密码、哈希、重置 token、验证 token 或 refresh token。

注册、登录、验证、重置、恢复、锁定和 MFA 是独立的工作流。仅实现任务拥有的流程，在需要时使用单次/过期密钥，在可行的情况下在响应和时序中避免账户枚举。

## 浏览器与跨域控制

Session/cookie 认证需要对不安全方法的 CSRF 保护。CORS 控制浏览器 origin；它不是授权。配置显式的 origin、credential、method 和 header，确保 proxy/HTTPS/cookie 设置匹配真实部署边界。

浏览器存储中的 bearer token 有不同的威胁模型。不要作为集成失败的本地权宜之计而切换存储或禁用 CSRF/CORS 保护。

## 错误、日志与密钥

根据已接受的披露策略，对缺失/无效认证保留 `401`，对已认证但不允许的操作保留 `403`。错误归属者的查找可能有意映射为 not found；一致地应用该策略。

对密钥、生命周期、issuer/audience、cookie 和哈希设置使用 `ConfigModule` 或仓库的类型化配置边界。从日志、trace、异常元数据和 Swagger 示例中脱敏凭证和 token。所选的应用可观测性参考拥有跨栈脱敏和关联契约。

## 验证

- 执行允许、缺失 identity、无效/过期 token、权限不足、错误归属者/租户和显式公共路径。
- 通过真实 Nest 应用运行受保护操作，使全局 guard 和元数据解析执行。
- 仅当拥有这些 token 行为时测试 issuer/audience/算法和 refresh 轮转/撤销。
- 验证密码哈希并确保每个响应/序列化路径排除密钥。
- 将列表隔离与详情和变更授权分开测试。
- 仅针对所选的浏览器/部署模型执行 CSRF、CORS preflight、安全 cookie 或 proxy 信任。

## 交付证据

标识 identity 机制、guard/策略边界、受保护操作以及允许和拒绝断言。仅 mock 的 guard 或解码的 token 不能证明全局注册、元数据优先级、归属范围限定、撤销或响应脱敏。

## 不安全默认

- 在没有已接受 identity 契约的情况下添加 JWT/Passport。
- `@Public` 元数据仅在 handler 上检查，忽略 controller 优先级。
- 将 role 当作行级授权。
- 没有轮转或撤销模型的长期 refresh token。
- 通过分散的 `process.env` 调用读取密钥。
- 为让浏览器请求通过而禁用 CORS 或 CSRF。
- 密码/token 字段从一个 DTO 中省略但通过另一个 serializer 泄漏。

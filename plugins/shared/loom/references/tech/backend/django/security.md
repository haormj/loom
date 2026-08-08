# Django 与 DRF 安全

使用 Django/DRF 已确立的机制实现已接受的认证和授权模型。当阶段不拥有 SimpleJWT、注册、role、API key 或账户工作流时，不要添加它们。

已接受的 JWT 算法、claim 和 HTTP 行为在 `tech/api/jwt.md` 中；本文件负责 Django/DRF 认证类、权限配置和框架特定的 settings。

## 何时使用

对 Django 认证、DRF 认证类、session、SimpleJWT、密码/账户流程、权限类、queryset/对象归属、CSRF/CORS、host/proxy 安全或受保护端点错误使用此参考。

## Implementation Focus

### Identity 机制

保留所选的客户端/信任模型：

| 客户端 | Django/DRF 边界 |
|---|---|
| 同源浏览器 | Django session 认证、安全 cookie、CSRF |
| Bearer-token API | 受信任的 issuer/token 验证或已配置的 SimpleJWT 契约 |
| 服务客户端 | 现有 API key、gateway identity、mTLS 感知或 service token 边界 |
| 公共端点 | 仅在已接受处显式 `AllowAny` |

保持全局 `DEFAULT_AUTHENTICATION_CLASSES` 和 `DEFAULT_PERMISSION_CLASSES` 保守。仅在契约不同时使用按 view/action 覆盖，并使公共例外可见且经过测试。

### Django 安全设置

从环境感知的已验证配置中加载 secret key、allowed host、trusted origin、proxy SSL header、CORS origin、数据库凭证和 token 设置。在本地开发之外保持 `DEBUG` 关闭，切勿提交生产凭证。

根据实际 hosting 边界配置 secure/HTTP-only/SameSite cookie、HTTPS 重定向/HSTS 和 proxy 信任。不要信任来自任意客户端的 forwarded host/proto header。

### 密码与用户模型

使用 `set_password`、`check_password`、`create_user`、Django 验证器和所选的密码 hasher。切勿将原始密码赋值给模型字段或通过 serializer/admin 日志暴露哈希。

当项目需要自定义用户模型时，在初始迁移之前选择。后期替换是迁移项目，而非常规字段编辑。注册、email 验证、重置、锁定和恢复各自需要显式的工作流归属。

### JWT 与 Refresh Token

当已接受的 profile 选择 JWT 时，将 `tech/api/jwt.md` 映射到 SimpleJWT 或仓库已有的验证器。Refresh、轮转、黑名单和登出行为是独立的能力，除非 API 契约拥有它们否则不得添加。

### 权限与归属

对操作级访问使用 `has_permission`，对已检索对象使用 `has_object_permission`。同时为列表隔离限定 queryset 范围以避免泄漏对象存在性。

```python
class IsOrderOwnerOrApprover(permissions.BasePermission):
    def has_object_permission(self, request, view, order):
        if request.method in permissions.SAFE_METHODS:
            return order.requester_id == request.user.id or request.user.can_approve
        return order.requester_id == request.user.id
```

当多个入口点必须强制执行 tenant、归属和生命周期检查时，将它们保持在可复用的 policy/query/service 边界中。UI 可见性和 serializer 字段省略不是授权。

### Session、CSRF 与 CORS

Session 认证的不安全请求需要 CSRF 保护。不要仅因为端点返回 JSON 就豁免它。仅 token 的客户端和浏览器 session 可能需要独立的路由或显式认证类。

CORS 允许浏览器 origin；它不是认证。使用显式的 origin/method/header，切勿对带凭证的 origin 使用通配符。测试中间件顺序和受保护写入的 preflight。

### 错误与数据披露

根据已接受的披露策略保留安全的 `401`/`403`/not-found 行为。在 login/reset/registration 中避免账户枚举，切勿暴露 token 解析器、数据库、权限内部信息或堆栈跟踪。

## Verification Focus

- 测试允许、匿名、无效 session/token、过期/错误 token、禁止、错误归属者和 admin/approver 路径。
- 独立于对象权限验证列表 queryset 范围限定。
- 在不暴露凭证的情况下测试密码哈希和 user-manager 行为。
- 仅当拥有时执行 refresh 轮转/黑名单/撤销。
- 针对真实的 session/bearer 浏览器模型验证 CSRF 和 CORS。
- 当安全 settings 变更时运行 Django 部署/安全检查，以及聚焦的行为测试。

## Evidence Focus

标识认证类、权限/归属规则、queryset 范围和精确的允许/拒绝断言。仅凭权限类声明或成功创建 token 不能证明端点保护。

## 不安全默认

- 因为 DRF 教程中常见就添加 SimpleJWT。
- 使用 `AllowAny` 或空权限类让测试通过。
- 对象权限没有列表 queryset 范围限定。
- 原始密码赋值或 API 输出中的密码哈希。
- 通配符带凭证的 CORS 或豁免 CSRF 的 session 写入。
- 硬编码的 secret key、allowed host、origin 或 token 生命周期。
- 在没有受控 proxy 边界的情况下信任 forwarded header。

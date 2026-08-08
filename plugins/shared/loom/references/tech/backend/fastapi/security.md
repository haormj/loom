# FastAPI 认证与授权

仅实现已接受的 identity 和访问策略。FastAPI 依赖是传输/应用守卫；它们不能成为在未认证阶段添加 JWT、密码登录、role、refresh token 或账户存储的理由。

已接受的 JWT profile 和 API 行为在 `tech/api/jwt.md` 中。本文件负责 FastAPI 依赖、中间件和配置。

## 何时使用

当任务拥有认证提取、token/session 验证、当前 actor 依赖、权限、资源归属、密码处理、安全中间件或受保护路由行为时使用此参考。

## Implementation Focus

### 机制边界

保留仓库和已接受的信任模型：

| 客户端/信任模型 | 合适的 FastAPI 边界 |
|---|---|
| 外部 bearer-token API | OAuth2 bearer 提取加 issuer/JWK 或受信任的 JWT 验证 |
| 同源浏览器 session | 安全 session cookie、CSRF 保护和当前 actor 依赖 |
| 服务间 | 现有 gateway identity、mTLS 感知基础设施或 service token 验证 |
| 显式公共/内部端点 | 无占位登录系统；声明路由为公共 |

`OAuth2PasswordBearer` 提取 bearer token 并文档化 scheme；它不验证签名、claim、用户或权限。当 identity provider 拥有 token 时，优先使用维护中的 issuer/JWK 验证路径而非手写 JWT 解析。

### Token 验证配置

实现来自 `tech/api/jwt.md` 的 claim、算法、issuer、audience、过期、not-before、subject、token 类型、密钥来源和时钟偏差需求。FastAPI 层必须将这些设置传递给维护中的验证器，不得添加第二个 token 契约。

```python
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="/auth/token")

async def require_actor(
    token: Annotated[str, Depends(oauth2_scheme)],
    users: Annotated[UserRepository, Depends(get_user_repository)],
) -> Actor:
    claims = token_verifier.verify_access_token(token)
    actor = await users.find_actor(claims.subject)
    if actor is None or not actor.active:
        raise unauthenticated()
    return actor
```

对缺失或无效的 bearer 凭证使用稳定安全的 `401` 响应，配以 `WWW-Authenticate: Bearer`。不要透露账户是否存在或哪个 token 检查失败。

### 密码与登录

使用维护中的自适应密码哈希库和仓库批准的参数。切勿存储明文或可逆密码、手动比较原始哈希或序列化密码哈希。

仅当 API/安全契约接受时，登录端点才需要速率/滥用处理。使用通用的无效凭证结果，避免账户枚举。密码重置、验证、锁定和恢复是独立的能力，不由登录隐含。

### 授权与归属

对粗粒度端点权限使用类型化依赖，对资源归属和生命周期资格使用应用 service。仅 role 检查不能证明对特定记录的访问。

```python
def require_permissions(*required: Permission):
    async def dependency(actor: CurrentActor) -> Actor:
        if not set(required).issubset(actor.permissions):
            raise HTTPException(status_code=403, detail="Forbidden")
        return actor
    return dependency
```

对未认证调用者保持 `401`，对缺乏权限的已认证调用者保持 `403`。根据已接受的策略隐藏或暴露受保护资源的存在性。UI 可见性绝不是授权。

### Refresh、撤销与登出

仅当已接受契约需要时才添加 refresh token。强制执行独立的 token 类型、audience/scope、生命周期、轮转或重用策略、存储/撤销模型和盗窃响应。切勿在 access-token 依赖中接受 refresh token。

删除浏览器/客户端 token 不会撤销无状态服务器 token。登出行为必须匹配实际的 session、denylist、轮转或短生命周期 token 模型。

### Cookie、CORS 与 CSRF

Cookie 认证的浏览器需要 secure、HTTP-only、适当的 `SameSite` cookie 和对状态变更请求的 CSRF 保护。Bearer-token API 不应习惯性地禁用不相关的浏览器保护。

使用显式的 CORS origin、method 和 header。通配符 origin 不能安全地与带凭证的浏览器请求组合。保持 CORS 和认证中间件顺序已验证。

### 敏感数据与错误

切勿记录授权 header、bearer/refresh token、密码、签名密钥、原始凭证 payload 或敏感 claim。公共错误保持稳定；所选的应用可观测性参考拥有受保护的支持关联的诊断。

## Verification Focus

- 测试任务拥有的允许、缺失认证、无效 token/session、过期、错误 issuer/audience/type 和权限不足路径。
- 独立于宽泛 role 证明资源归属拒绝。
- 验证 `401`、`403`、`WWW-Authenticate` 和已接受的安全错误 body。
- 在不暴露凭证或哈希的情况下测试密码哈希/验证。
- 仅当该能力存在时测试 refresh 轮转/重用/撤销。
- 针对真实的浏览器信任模型验证 cookie、CSRF 和 CORS 行为。

## Evidence Focus

标识受保护操作、identity 机制、claim/permission 规则和精确的拒绝/成功断言。依赖出现在函数签名中或 OpenAPI 锁图标不能证明认证或授权行为。

## 不安全默认

- 因为 FastAPI 示例使用了自定义 JWT 登录就添加它。
- 硬编码的密钥、算法、issuer、audience、生命周期或 origin。
- 将 token 提取当作 token 验证。
- 将 refresh token 当作 access token 复用。
- Role 检查没有资源归属检查。
- `401` 和 `403` 合并为一个模糊结果。
- 通配符带凭证的 CORS 或没有 CSRF 保护的 cookie 认证。
- 日志或响应模型中的 token、密码、哈希或敏感 claim。

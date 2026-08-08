# FastAPI 路由与应用边界

实现已接受的 HTTP 接口，不重新设计方法、路径、payload、状态码或错误语义。本参考负责 FastAPI router 组合、依赖边界、端点生命周期和传输行为。

## 何时使用

当任务拥有 FastAPI 路由、router 组合、HTTP 依赖、端点错误转换、后台响应、WebSocket 或 OpenAPI 暴露时使用此参考。纯 Python service 和仅持久化任务不需要它。

## Implementation Focus

### 应用与 Router 归属

在一个组合根中创建 `FastAPI` 应用。在那里或通过仓库标准的应用工厂注册 router、异常 handler、中间件和 lifespan 资源。避免将全局应用对象导入 domain 或持久化模块。

按业务能力用 `APIRouter` 分组路由。前缀和 tag 遵循已接受的接口和仓库布局；不要因为示例使用了 `/v1` 就发明它或更改 trailing-slash 行为。

```python
router = APIRouter(prefix="/orders", tags=["orders"])

DbSession = Annotated[AsyncSession, Depends(get_db_session)]
CurrentActor = Annotated[Actor, Depends(require_actor)]

@router.post("", response_model=OrderRead, status_code=status.HTTP_201_CREATED)
async def create_order(
    command: OrderCreate,
    session: DbSession,
    actor: CurrentActor,
) -> OrderRead:
    created = await order_service.create(session, actor, command)
    return OrderRead.model_validate(created)
```

### 端点与 Service 边界

端点函数拥有传输工作：参数绑定、依赖解析、请求验证、一个应用操作和响应映射。业务状态转换、归属规则、事务决策、provider 重试和多记录不变量属于 service 或 domain 组件。

对稳定的共享依赖使用 `Annotated` 别名。不要将实质不同的授权或事务行为隐藏在一个模糊的依赖别名后面。当依赖函数拥有请求范围的资源时，必须用 `yield` 声明清理。

### Async 与阻塞工作

`async def` 仅在调用链执行可 await 的 I/O 时有用。切勿直接在事件循环上调用同步数据库客户端、HTTP 客户端、文件系统调用或 CPU 密集型工作。为有界的同步工作保留同步端点，慎重使用仓库的线程池边界，或端到端选择 async 适配器。

不要为必需的业务工作调用 `asyncio.create_task`。FastAPI `BackgroundTasks` 在响应之后的同一进程中执行，不是持久的。仅对有界的、可容忍丢失的、可观测失败的后续工作使用它们；持久工作需要已接受的 job 或消息边界。

### 参数与集合

对传输约束使用类型化的 `Path`、`Query`、`Header` 和 body 模型。将路径参数声明放在冲突的动态路由之前。对过滤和排序字段使用显式允许列表验证。

无界集合需要已接受的分页契约、确定性排序、有界的最大大小和稳定的元数据。空集合返回已声明的成功集合形态，而非 not-found 错误。

### 错误转换

通过聚焦的异常 handler 或一致的端点边界转换 domain/application 失败。保留已接受的错误 body，并区分验证、not found、冲突、未认证、禁止、依赖不可用和意外失败。

```python
@app.exception_handler(OrderConflict)
async def order_conflict_handler(
    request: Request,
    error: OrderConflict,
) -> JSONResponse:
    return JSONResponse(
        status_code=status.HTTP_409_CONFLICT,
        content={"code": error.code, "message": error.user_message},
    )
```

切勿返回原始的 SQLAlchemy、JWT、provider、traceback、文件路径或类名细节。不要在每个端点捕获每个异常；当此任务拥有可观测性时，意外失败应到达 `tech/code/observability.md` 选择的唯一边界。

### Lifespan、流与 WebSocket

对共享客户端和资源使用 FastAPI lifespan，配以显式的启动失败和关闭行为。当迁移拥有 schema 演进时，不要在应用启动期间创建 schema 或植入业务数据。

流响应需要有界的生产者、取消清理、media type 和断开连接行为。WebSocket 需在接受受保护 session 之前认证、连接归属、消息大小限制、背压、断开清理和定义的多实例投递模型。

### OpenAPI 契约

当 `response_model`、状态码、参数约束、摘要和文档化错误是已接受契约的一部分时设置它们。慎重地排除内部/admin 路由。生成的 OpenAPI 是传输形态的证据，而非业务行为有效的证明。

## Verification Focus

- 通过 `httpx`/ASGI 传输执行已接受的成功和阻塞路径。
- 断言任务拥有的精确状态、响应/错误形态、header、分页、过滤和排序。
- 验证 router 包含、路径优先级、依赖清理和异常 handler 注册。
- 通过实际运行相关生命周期的客户端测试 lifespan 或 WebSocket 行为。
- 对变更的公共路由检查 OpenAPI 操作和 schema。

## Evidence Focus

标识变更的路由、依赖、异常边界或生命周期资源以及证明其外部可见行为的测试。路由出现在 OpenAPI 中或返回任何 `2xx` 响应不足以作为验证、授权、事务或失败语义的证据。

## 不安全默认

- 端点函数中直接包含 repository 调用和状态转换。
- `async def` 中的阻塞 I/O。
- 跨请求共享的全局可变依赖状态。
- 无界的列表端点或任意客户端选择的排序字段。
- 用 `BackgroundTasks` 或 `create_task` 调度的持久业务工作。
- 暴露内部消息或将失败转换为成功的 catch-all 异常 handler。
- 添加已接受接口中不存在的版本化前缀。

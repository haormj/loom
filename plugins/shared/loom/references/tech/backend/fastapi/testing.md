# FastAPI 异步测试

使用能证明任务拥有行为的最小边界。此参考仅在任务显式拥有 FastAPI 测试实现时适用；框架可用性本身不产生测试工作。

## 何时使用

对 FastAPI 端点、依赖、lifespan、异常 handler、安全、WebSocket、后台行为或异步 SQLAlchemy 集成测试使用此参考。纯 Python 单元测试仍在 Python 测试参考中。

## Implementation Focus

### 测试边界选择

| 行为 | 首选边界 |
|---|---|
| 纯领域/service 规则 | 带真实小型协作者或聚焦 fake 的纯 pytest |
| HTTP 路由/验证/错误形态 | 带 `ASGITransport` 的 `httpx.AsyncClient` |
| 依赖组合 | 带范围限定依赖覆盖的 ASGI 请求 |
| SQLAlchemy 映射/查询/事务 | 异步 provider 支持的持久化测试 |
| 认证/授权 | 真实安全依赖和合成 token/session |
| Lifespan 资源 | 执行启动和关闭的客户端/lifespan 管理器 |
| WebSocket | 框架测试客户端或支持实际 session 生命周期的异步客户端 |

不要为纯验证/计算启动数据库或完整应用。不要用 mock 替换被测试行为然后仅断言 mock 调用。

### 异步 HTTP 客户端

一致地使用一个异步测试后端。`ASGITransport` 不总是自动执行 lifespan；当启动资源有影响时使用仓库的 lifespan 管理器或应用工厂。

```python
@pytest.fixture
async def client(app: FastAPI) -> AsyncIterator[AsyncClient]:
    async with LifespanManager(app):
        async with AsyncClient(
            transport=ASGITransport(app=app),
            base_url="http://test",
        ) as http:
            yield http


@pytest.mark.anyio
async def test_conflicting_state_returns_contract_error(client: AsyncClient) -> None:
    response = await client.post("/orders/42/approve")
    assert response.status_code == 409
    assert response.json() == {
        "code": "ORDER_STATE_CONFLICT",
        "message": "Order cannot be approved from its current state",
    }
```

使用 `pytest.mark.anyio` 或仓库已建立的 asyncio 插件，而非混合事件循环 fixture 和 marker。将未 await 的协程警告和泄漏的 task 视为失败。

### 依赖覆盖

按测试构建应用或在 `finally`/fixture teardown 中恢复每个覆盖。覆盖必须具有兼容的依赖形态，包括 async/yield 清理。

```python
@pytest.fixture
def override_actor(app: FastAPI, actor: Actor) -> Iterator[None]:
    app.dependency_overrides[require_actor] = lambda: actor
    try:
        yield
    finally:
        app.dependency_overrides.pop(require_actor, None)
```

当其他 fixture 可能拥有覆盖时，不要在共享套件中调用 `dependency_overrides.clear()`。避免跨并行测试的可变全局应用状态。

### 持久化 Fixture

对 dialect 敏感行为使用所选 provider 或仓库批准的兼容测试 provider。保持 engine/session 生命周期有范围和确定性。事务回滚 fixture 仅在行为不依赖提交、post-commit 事件、约束时序或另一个连接观察数据时有用。

当迁移/运行时兼容性在测试范围时运行迁移。`metadata.create_all()` 证明映射可以创建表，而非迁移完整。

### 安全 Fixture

对安全行为使用带真实验证的合成凭证。覆盖缺失、无效、过期、错误类型、拒绝和允许案例。仅当测试显式针对下游业务行为而非 auth 配置时才覆盖当前 actor 依赖。

### 后台工作与 WebSocket

对于 `BackgroundTasks`，在响应生命周期之后断言后续效果或失败信号。对于持久 job，测试生产者边界而非假装进程内后台工作是持久的。

WebSocket 测试应覆盖任务拥有的认证、接受/拒绝、消息验证、断开清理和有界广播行为。

### 断言与隔离

断言完整契约结果：状态、body、header、持久化状态、发出的命令或稳定错误。避免仅断言非 null、列表长度、无异常或 mock 调用的测试。

冻结或注入时间、ID 和随机性。不要使用任意 sleep；在支持的地方使用完成信号、事件或虚拟时间。

## Verification Focus

- 先为变更的 FastAPI 行为运行窄测试模块或 marker，当风险跨共享依赖时运行所属包套件。
- 覆盖成功和重要的验证、not-found、冲突、认证和依赖失败路径。
- 验证依赖覆盖和 lifespan 资源在每个测试后已恢复。
- 当持久性或另一个请求观察结果时证明数据库提交/回读。
- 当 router 或 schema 暴露变更时检查变更的 OpenAPI 操作。

## Evidence Focus

记录精确的测试边界、场景、命令和有意义的断言。测试计数或通过的应用启动不能证明任务的路由、失败、持久化、安全或清理行为。

## 不安全默认

- 同步 `TestClient` 在无理由的情况下混入有意异步套件。
- 假设 `ASGITransport` 自动运行 lifespan。
- 测试间泄漏的全局依赖覆盖。
- SQLite 用作 provider 特定 SQL 或锁的证明。
- 在唯一的端点测试中绕过安全过滤器/依赖。
- 测试级回滚用于证明 post-commit 行为。
- 任意 sleep、共享可变 fixture 或顺序依赖测试。

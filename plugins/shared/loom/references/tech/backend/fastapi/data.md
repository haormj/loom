# FastAPI 异步 SQLAlchemy 数据访问

此参考仅在已接受的技术栈使用 SQLAlchemy 时适用。它负责 FastAPI 应用的异步 engine/session 集成、事务边界、查询行为和迁移对齐；provider SQL 仍在所选的 SQL 参考中。

## 何时使用

对 SQLAlchemy 2.x 模型、async session、repository/query 组件、事务行为、关系加载、Alembic 集成和 provider 支持的持久化测试使用此参考。不要将其应用于 Tortoise ORM、Django ORM、Spring Data、jOOQ 或无持久化的 API 工作。

## Implementation Focus

### Engine 与 Session 生命周期

为应用生命周期创建一个 async engine 和 `async_sessionmaker`。URL 必须使用所选的 async driver。从运行时需求配置连接池行为，而非将生产规模的默认值复制到每个项目。

```python
engine = create_async_engine(settings.database_url, pool_pre_ping=True)
session_factory = async_sessionmaker(engine, expire_on_commit=False)

async def get_db_session() -> AsyncIterator[AsyncSession]:
    async with session_factory() as session:
        yield session
```

请求依赖拥有 session 的打开和关闭，而非每个请求的自动提交。应用/service 操作应通过 `async with session.begin()` 或仓库标准的工作单元边界使事务边界显式。不要让端点、service 和依赖层各自独立提交。

切勿跨并发任务或请求共享 `AsyncSession`。不要将其放在模块全局可变状态中。在应用 lifespan 关闭期间释放 engine。

### 映射边界

使用 SQLAlchemy 2.x 类型化映射，配以 `Mapped` 和 `mapped_column`。将标识符、可空性、长度、enum、小数、时间戳、默认值、外键、唯一约束、索引和版本字段与已接受的迁移和 provider 行为对齐。

将 ORM 实体排除在公共响应契约之外。Pydantic 响应映射在所需数据已加载且 session 边界已知时进行。避免序列化中的隐式懒加载。

### 查询形态与加载

使用 `select()` 和显式的标量/基数操作：

- 当恰好需要一行时使用 `scalar_one()`
- 对唯一可选查找使用 `scalar_one_or_none()`
- 对有意有界的结果使用 `scalars().all()`
- 对有界的关系集合使用 `selectinload`
- 对合适的单值关系使用 `joinedload`

在查询边界定义分页、确定性排序、过滤允许列表和投影形态。不要加载无界表并在 Python 中切片。通过将 loader 策略与实际的 list/detail 读路径匹配来避免 N+1 访问。

### 写入与事务语义

当在提交之前需要生成标识符或约束时序时使用 `flush()`；仅对必须重新加载的值使用 `refresh()`。Flush 不是持久提交。

批量 `update()`/`delete()` 绕过正常的对象状态和生命周期处理。指定同步/版本行为，不要在批量操作之后返回陈旧的内存对象。

在回滚之后将已知的唯一性、外键、乐观并发和完整性失败映射到稳定的应用结果。存在性检查可以改善反馈，但不能替代并发下的数据库约束。

外部 HTTP、email、支付或消息副作用不与 SQL 事务原子化。使用已接受的 post-commit、outbox、幂等或补偿边界。

### Async 正确性

不要从异步请求路径调用同步 engine、session、driver 或阻塞的 provider helper。避免通过属性访问的隐藏 I/O。Await 每个数据库操作，并在独立的 session 上保持独立的并发查询。

取消和超时可能中断应用等待，而数据库操作状态不确定。为重试敏感的写入定义事务回滚和幂等。

### Alembic 与启动

Alembic 或所选的迁移工具拥有生产 schema 演进。不要在正常应用启动期间调用 `metadata.create_all()` 作为迁移的替代。

保持迁移配置与运行时模型 metadata 和 provider URL 对齐，而不导入整个 FastAPI 应用。当一步无法安全应用时，分离 schema 变更、回填、约束激活和清理。

## Verification Focus

- 对所选或兼容的仓库测试 provider 运行异步 write/flush/commit/回读测试。
- 证明任务涉及的 ID、默认值、enum、小数、时间戳、关系、约束和版本行为。
- 测试查询基数、过滤、排序、分页、eager loading 和空结果。
- 在预期和意外失败之后验证回滚和 session 清理。
- 当迁移变更时通过迁移升级干净的 schema 并针对它启动映射。
- 对原生 SQL、生成值、锁、JSON/array 类型或 dialect 行为使用 provider 特定测试。

## Evidence Focus

标识 session/事务归属、查询形态、迁移或完整性行为以及证明它的 provider 支持断言。提交前的内存对象断言不能证明 schema、约束、事务或回读行为。

## 不安全默认

- 为多步工作流在请求依赖中隐藏自动提交。
- 一个 `AsyncSession` 跨并发任务共享。
- `async def` 端点中的同步 SQLAlchemy 调用。
- ORM 实体直接通过 API 返回。
- Pydantic 序列化期间的懒加载关系访问。
- 无界的 `scalars().all()` 列表查询。
- `create_all()` 用作生产迁移行为。
- 仅 SQLite 测试被声称为 provider 特定 SQL 的证明。

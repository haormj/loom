# Spring Boot 数据访问实现

本参考负责 Spring Data repository 行为、Spring 事务代理、查询集成和 Boot 迁移启动。JPA 实体语义仍在 Java 持久化参考中；SQL 语法和 provider 行为仍在所选的 SQL/provider 参考中。

## Spring Data 边界

使用 repository 接口进行具有明确基数的持久化操作。将业务工作流保留在 application/domain service 中。

| 需求 | Spring Data 形态 |
|---|---|
| 按唯一键查找 | `Optional<T>` 加数据库唯一约束 |
| 存在性检查 | `boolean existsBy...` 用于早期反馈；仍需处理写入竞态 |
| 有界集合 | `Page<T>`、`Slice<T>` 或显式限制的投影 |
| 动态过滤 | `Specification<T>` 或专用查询组件 |
| 列表/详情读模型 | 仅包含必需字段的接口/类/record 投影 |
| 批量更新/删除 | 显式事务内配以已定义持久化上下文行为的 `@Modifying` 查询 |

派生查询名适合简短稳定的谓词。将不可读的方法名替换为 JPQL、Specification、Query by Example 或专用查询实现。原生查询是 provider 特定的，需要所选 provider 参考和 provider 兼容测试。

## 事务归属

将事务边界放在通过 Spring 代理到达的公共 application/service 方法上。

```java
@Service
final class OrderApplicationService {
    private final OrderRepository orders;

    OrderApplicationService(OrderRepository orders) {
        this.orders = orders;
    }

    @Transactional
    public OrderResponse approve(OrderId id, Version expectedVersion) {
        Order order = orders.findById(id.value()).orElseThrow(OrderNotFound::new);
        order.approve(expectedVersion);
        return OrderResponse.from(order);
    }
}
```

考虑 Spring 代理语义：

- 同类调用不激活不同的 `@Transactional` 传播模式
- private 方法不是事务入口点
- 受检异常回滚与不受检异常回滚不同，除非配置
- `readOnly = true` 是优化提示，而非授权或不可变性边界
- 数据库事务内的长外部调用增加锁时间和失败耦合

当确实需要独立提交时，将 `REQUIRES_NEW` 工作移至另一个代理 bean。不要将其用作日志快捷方式而不定义父回滚后什么存活。

外部支付、消息、email 和 HTTP 调用不是数据库事务的一部分。当外部副作用必须跟随已提交的写入时，定义 outbox、post-commit 事件、补偿行为或显式重试边界。

## Repository 查询形态

对读密集的列表/详情路径使用投影。保持 fetch plan 用例特定：

- 对稳定的关联集使用 `@EntityGraph`
- 对有界的详情查询使用 fetch join
- 对表格、导出和摘要使用 DTO 投影
- 仅在测量后且访问模式支持时使用批量获取

不要将集合 fetch join 与可分页查询组合而不证明计数和行语义。不要全局将关联切换为 eager 以隐藏 `LazyInitializationException`。慎重使用 Open Session in View；新 API 不应依赖视图时懒加载。

对于 `@Modifying` 操作，决定是否需要 `clearAutomatically` 或 `flushAutomatically`。批量 JPQL 绕过托管实体状态、回调和乐观锁，除非查询显式强制版本行为。

## Specification 与过滤器

Specification 应组合稳定谓词，不接受来自客户端的任意属性名。

```java
static Specification<OrderEntity> hasStatus(OrderStatus status) {
    return status == null ? null : (root, query, cb) -> cb.equal(root.get("status"), status);
}

static Specification<OrderEntity> submittedAfter(Instant since) {
    return since == null ? null : (root, query, cb) ->
        cb.greaterThanOrEqualTo(root.get("submittedAt"), since);
}
```

对关系谓词使用显式 join，仅在 join 多重性需要时应用 `distinct`。保持过滤、排序和分页字段名与已接受的 API 和实际实体/投影字段对齐。

## Spring Data 审计与事件

仅当 actor 和时间语义被接受时才启用 Spring Data 审计。通过应用面向的 identity 和 clock 边界提供 `AuditorAware` 和时间，这些边界也适用于 HTTP 请求、job、迁移和系统操作。审计列和可空性必须与迁移保持对齐。

Repository `save` 事件和 `@DomainEvents` 在 repository 生命周期周围运行；它们不使外部消息与数据库提交原子化。对有界的进程内后续使用事务感知监听器，对外部投递使用已接受的 outbox/持久机制。不要从实体回调发布远程副作用。

在保留原始回滚行为之后，在应用边界转换乐观锁和已知约束失败。JPA identity、关系、fetch 和 provider 映射语义仍在 Java 持久化参考中。

## 迁移集成

当选择 Flyway 或 Liquibase 时，迁移拥有 schema 演进。Hibernate auto-DDL 不得静默修改生产 schema。

Boot 启动必须建立一致的顺序：

1. datasource 属性绑定
2. 迁移工具到达所选 provider
3. 迁移应用或验证
4. JPA 映射初始化
5. 可选 Hibernate schema 验证比较兼容类型

通过所选 SQL 参考保持迁移 SQL provider 特定。不要将 PostgreSQL `BIGSERIAL`、MySQL 特定 DDL 或 H2 语法复制到 provider 中立参考中。

当一次迁移无法安全执行所有步骤时，分离 schema 创建、数据回填、约束激活和清理。为部分应用或非事务性迁移定义前向修复。

## 并发与完整性

使用数据库约束作为最终完整性边界。写入前的 `existsBy...` 检查改善错误消息但不能防止并发重复。

从已接受的数据架构中选择乐观锁、悲观锁、序列化、幂等或领域拒绝。测试应证明所选冲突行为。不要捕获并忽略 `DataIntegrityViolationException`；翻译已知约束并保留意外失败。

## Verification Focus

有用的数据证据包括：

- 具有真实持久化行的 repository 基数和查询行为
- ID、enum、时间戳、版本、默认值和关系的写入/读取往返
- 事务回滚和 post-commit 副作用边界
- 重复、not-found、陈旧版本和约束失败行为
- 列表投影、过滤、排序、分页和计数正确性
- 针对所选 provider 的迁移启动和映射验证
- N+1 修正的查询计数或 fetch plan 证据

## 不安全默认

- 将实体作为 API DTO 返回。
- 在 controller 或 private helper 方法上放 `@Transactional`。
- 通过同类自调用调用 `REQUIRES_NEW`。
- 在无界外部调用期间保持事务打开。
- 将 H2 兼容性视为其他生产 provider 的证明。
- 启用 `ddl-auto=update` 作为迁移策略。
- 没有任务拥有理由地添加缓存、原生查询或 eager 关系。

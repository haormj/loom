# Entity Framework Core 持久化

仅当已选择 Entity Framework Core 且任务拥有持久化时才应用此参考。已接受的数据架构和数据库 provider 决定映射和迁移行为；ASP.NET Core 并不意味着必须使用 EF Core 或 SQL Server。

## DbContext 边界

使用应用已选择的 provider 和生命周期注册 `DbContext`。Scoped context 通常表示一个工作单元；它不是线程安全的，不得存储在 singleton 中或跨任务并发使用。

将 provider 设置和连接配置保持在组合根中。在启动时验证必需的连接/选项，不记录凭证。对已接受的背景/并发 scope 使用 `IDbContextFactory<T>` 来获取独立的 context，而非绕过正常的请求 scope。

不要在迁移拥有的生产数据库中调用 `EnsureCreated`。保持 design-time factory/setup 与运行时 provider 和迁移一致。

## 模型配置

使用 `IEntityTypeConfiguration<T>` 或仓库已有的配置风格。在正确性依赖的方面显式指定键、生成值、必需/可选字段、长度、精度/标度、Unicode/collation、索引、备用键、并发 token 和删除行为。

```csharp
public sealed class OrderConfiguration : IEntityTypeConfiguration<Order>
{
    public void Configure(EntityTypeBuilder<Order> builder)
    {
        builder.HasKey(x => x.Id);
        builder.Property(x => x.Status).HasConversion<string>().HasMaxLength(32);
        builder.Property(x => x.Total).HasPrecision(18, 2);
        builder.HasIndex(x => new { x.TenantId, x.Number }).IsUnique();
        builder.Property(x => x.Version).IsConcurrencyToken();
    }
}
```

根据领域生命周期和 provider 支持选择 owned/complex type、value converter、backing field 和 join entity。Converter 会更改持久化表示，但可能不保留查询翻译或比较语义。

根据归属关系设置 `DeleteBehavior`。级联不是方便的默认值。慎重处理必需关系、孤儿行为、循环和软删除过滤器。

## 查询形态

对于只读查询使用 `AsNoTracking`，除非需要 identity 解析或更新。直接投影到响应/读模型并仅选择必需的列。

仅在确实需要实体图时使用 `Include` 进行聚合加载。列表优先使用投影；当多个集合会造成笛卡尔积爆炸时使用 split query，并验证其一致性/性能权衡。

对过滤、排序和分页使用有界和确定性顺序。避免客户端求值、过滤前的 `ToList`、懒加载 N+1 行为和无界集合物化。对复杂或性能敏感的查询检查生成的 SQL。

仅在测量表明重复翻译成本有影响时才使用编译查询。Provider 索引和查询计划仍然是主要的性能边界。

## 写入、并发与事务

使用异步 EF API 并传播 `CancellationToken`。慎重地 attach/update 图；宽泛的 `Update(entity)` 可能将每个字段标记为已修改并覆盖并发更改。

数据库约束是最终的唯一性/完整性边界。通过 provider 感知的适配器翻译已知的 `DbUpdateException` 场景，不要在各 service 中解析消息文本。

当陈旧写入有影响时，使用 row-version/并发 token 或显式的状态/版本谓词。捕获 `DbUpdateConcurrencyException`，决定 reload/merge/reject 行为，并映射已接受的冲突响应。

`SaveChanges` 对其批次是事务性的。当多次保存/context 或协调操作需要一个数据库原子边界时，使用显式事务。将网络/消息/email 工作保持在事务之外；使用已接受的 outbox 进行持久发布。

## 迁移与数据演进

从预期模型生成迁移，检查每个操作，并将迁移历史保持在源代码管理中。对兼容性敏感的变更使用展开/回填/切换/收缩步骤，而非一次性破坏性迁移。

大批量回填、非空添加、索引创建和 provider 特定的在线行为需要有界的运维策略。当部署流程可能重试时，数据迁移必须是确定性的和重启安全的。

除非运行时契约明确协调，否则不要从每个应用副本自动应用迁移。当声明了升级行为时，验证从干净的已选 provider 数据库创建以及从代表性先前 schema 升级两种路径。

## Provider 保真度

SQL Server、PostgreSQL、MySQL 和 SQLite 在生成值、小数、日期/时间、JSON、collation、索引、计算列、迁移、锁和并发方面存在差异。对 provider 特定的声明使用所选的 provider。

SQLite 仅当它是已接受的生产 provider 或被测试的行为与 provider 无关时才适用。EF InMemory provider 不能证明关系约束、事务、查询翻译或迁移行为。

## 验证

- 证明映射、默认值、生成值、enum/value object、关系和小数/时间戳的创建/更新/回读。
- 断言唯一性/check/delete 和乐观并发结果。
- 验证投影、过滤、确定性排序、分页和相关的生成 SQL/查询计数。
- 执行事务回滚和不存在部分副作用。
- 当发生变更时，将迁移应用到干净的已选 provider 数据库并测试代表性的升级路径。
- 当拥有背景/并发使用时，确认取消和 context 生命周期行为。

## 交付证据

标识 EF 配置/查询/迁移以及证明它的已选 provider 断言。仅凭通过的 mock、InMemory 测试、生成的迁移文件或成功启动不能证明关系完整性、查询翻译、并发或升级安全。

## 不安全默认

- 因为存在 ASP.NET Core 就选择 EF Core。
- 在生产中同时使用 `EnsureCreated` 和迁移。
- `DbContext` 跨线程或 singleton service 共享。
- 从 HTTP 响应返回实体。
- 为列表端点使用 `Include` 加载完整图。
- 对 detached 的客户端形态对象应用 `Update`。
- 对 provider 特定行为声称 SQLite/InMemory 证据。
- 没有兼容性/回填规划的破坏性迁移。

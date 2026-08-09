# Django 模型、迁移与 ORM

Django 模型定义持久化 identity、约束、关系和查询行为。迁移拥有 schema 演进；serializer、view 和 admin 界面是消费者，而非 model/data 完整性的替代品。

## 何时使用

对 Django 模型字段、约束、索引、关系、manager/queryset、事务、迁移操作、批量写入和 ORM 查询性能使用此参考。仅涉及 DRF 传输的变更不需要它。

## Implementation Focus

### 模型与字段语义

从已接受的数据模型中选择字段类型、长度、精度、可空性、`blank`、默认值、choices 和数据库约束。`null` 控制存储而 `blank` 控制验证；不要在字符串上不加区分地同时使用两者。

当值需外部持久化时，通过 `TextChoices`/`IntegerChoices` 使用稳定的 enum 值。对类货币值使用 `DecimalField`，并使用时区感知的 Django datetime 行为。保持服务端生成/默认值与迁移和 API 输出一致。

关系需要显式的生命周期语义：

- 仅当子记录不能脱离所有者存在时使用 `CASCADE`
- 当必须阻止删除时使用 `PROTECT`/`RESTRICT`
- 仅在可空存储且已接受孤儿含义时使用 `SET_NULL`
- 当多对多关系有属性或生命周期时使用显式 through model

设置有用的 `related_name` 值，避免意外的反向名称冲突。保持 `__str__` 有界且没有懒加载关系查询。

### 约束与索引

数据库约束是最终的完整性边界。在所选 provider 支持的地方使用 `UniqueConstraint`、`CheckConstraint` 和条件约束。Pre-save 验证或 `.exists()` 可以改善反馈，但不能防止并发写入。

为实际的过滤、join、排序和唯一性路径添加索引。优先使用匹配常见查询前缀的复合顺序。在没有证据的情况下，不要索引每个字段或复制唯一性/外键已提供的索引。

### Manager 与 QuerySet

将可复用的领域查询概念放在自定义 `QuerySet` 方法中，通过 `as_manager()` 或类型化 manager 暴露。Queryset 保持懒加载；在所属边界慎重地求值。

```python
class OrderQuerySet(models.QuerySet["Order"]):
    def visible_to(self, actor: User) -> "OrderQuerySet":
        if actor.is_staff:
            return self
        return self.filter(requester=actor)

    def for_list(self) -> "OrderQuerySet":
        return self.select_related("requester").only(
            "id", "status", "requested_at", "requester__username"
        )
```

保持 tenant/归属范围限定可组合，并在查找之前应用。不要在未限定范围的类 queryset 上调用 `.all()` 并依赖 serializer/view 后续过滤。

### 加载与查询形态

对单值外键/一对一路径使用 `select_related`，对集合和反向关系使用 `prefetch_related`/`Prefetch`。将加载与 list/detail serializer 匹配，而非应用一个巨大的全局 prefetch。

当 `values`、`values_list`、annotation、子查询、`Exists`、`F` 和 `Q` 能高效表达有界的读/更新时使用它们。如果延迟字段后续被访问，`only`/`defer` 可能产生隐藏的后续查询。

在评估大型 queryset 之前分页并使用确定性排序。将查询计数改进视为基于证据的变更，而非假设。

### 事务与并发

在多行不变量和状态转换周围使用 `transaction.atomic()`。将网络调用和缓慢的外部工作保持在数据库事务之外。当已接受悲观锁时使用 `select_for_update` 并配以显式顺序/scope，并定义锁争用行为。

对并发计数器和转换使用 `F()` 表达式、约束或版本/状态谓词。陈旧读取后的 `save()` 不是并发控制。

批量 `update`、`bulk_create` 和 `bulk_update` 绕过 `save()`、模型验证和许多 signal。仅当跳过的生命周期行为是有意为之时才使用它们。

### 迁移纪律

生成并检查迁移，仅在所需操作无法正确表示时才编辑。对大型或兼容性敏感的变更使用独立的 schema、数据回填、约束激活和清理迁移。

不要在数据迁移中直接运行应用模型代码；使用 `apps.get_model` 获取历史模型。保持反向行为和 provider 能力显式。切勿用手动数据库更改或 `--fake` 作为常规修复来替代迁移。

## Verification Focus

- 测试变更字段的 create/update/回读，包括默认值、enum、小数、时间戳和关系。
- 证明唯一性/check/delete 约束和并发冲突行为。
- 当拥有性能时，验证自定义 queryset 范围限定、结果正确性、排序、分页和查询计数。
- 执行事务回滚和任何已接受的锁定/状态谓词。
- 当 schema 变更时运行迁移检查并将干净的测试数据库迁移。
- 对 provider 特定的约束、索引、JSON、锁或 SQL 行为使用所选 provider。

## Evidence Focus

标识 model/query/migration 边界以及证明它的持久化或查询断言。仅凭 serializer 验证、内存中的模型实例或生成的迁移文件不能证明数据库完整性或升级行为。

## 不安全默认

- 每个可选字符串都复制 `null=True` 和 `blank=True`。
- 没有生命周期归属的 `CASCADE`。
- tenant 拥有数据的类级未限定范围 queryset。
- 没有 loader 规划的无界关系序列化。
- `transaction.atomic()` 中的外部调用。
- 假设批量操作会执行 `save()` 或 signal。
- 数据迁移导入当前应用模型。
- 用 fake 迁移隐藏的手动 schema 漂移。

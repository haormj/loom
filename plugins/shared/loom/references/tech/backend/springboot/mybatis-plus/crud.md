# MyBatis-Plus Mapper 与 Service CRUD

## 何时使用

当任务实现或变更 MyBatis-Plus Mapper、Service、列表、详情、批量、分页或流查询行为时使用此参考。

## Implementation Focus

- 当仓库有 Service/application 层时，将 controller 和传输适配器排除在直接 Mapper 调用之外。
- `BaseMapper` 是持久化原语；`IService` 和 `ServiceImpl` 是可选的项目约定，非强制样板。
- 将授权、租户 scope、审计、事务和 domain 规则保留在已建立的 application/service 边界中。

## 读取与写入

- 用 `Page`/`IPage`、显式限制或任务拥有的结果大小约束限定列表查询。不要对用户控制的集合使用无界 `selectList`。
- 仅在具有显式批量大小、事务边界、重复行为和失败处理时才使用 `saveBatch` 和批量更新。
- 根据已接受的逻辑删除契约处理 `removeById`；物理删除需要显式业务理由。
- 仅在具有已定义连接、事务、资源关闭和取消边界时才流式处理大结果。

## Verification Focus

证明基数、not-found 和重复行为、分页总数、批量回滚或部分失败语义、逻辑删除、授权/租户条件和需要时的写后读行为。

## 失败边界

- 在应用边界转换已知的约束和乐观锁失败而不隐藏原始回滚行为。
- 除非已接受设计定义 outbox 或补偿边界，否则将外部调用保持在数据库事务之外。
- 不要让通过的 Mapper 单元测试替代 provider 特定 SQL 或迁移证据。
- 当任务可被执行或运行时工作流重试时保持批量操作幂等。
- 为每个分页或流式集合保留确定性排序。

## Evidence Focus

记录拥有的 Mapper 或 Service 方法、查询边界、事务边界以及分页、失败、授权和重试行为的证据。

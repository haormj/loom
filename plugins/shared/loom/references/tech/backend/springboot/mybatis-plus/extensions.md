# MyBatis-Plus 扩展与高影响功能

## 何时使用

仅当任务拥有生成器、自定义 ID 策略、SQL injector、ActiveRecord 或 Db Kit 使用、多数据源或迁移敏感扩展时才使用此参考。

## Implementation Focus

将每个扩展视为显式的仓库范围或模块本地契约。优先使用满足已接受架构的最小 scope。

## 生成器与 ID

- 在使用 `FastAutoGenerator` 或 `AutoGenerator` 之前确认包、模块、表前缀、父类、命名、XML 输出和覆盖策略。
- 生成的文件需要审查注解、字段类型、删除/版本/填充行为、索引和权限入口点。
- `IdType.AUTO`、`ASSIGN_ID` 和自定义 `IdentifierGenerator` 具有不同的数据库、分布、时钟、序列化和历史数据契约。

## 全局扩展

- 自定义 SQL injector、基础 Mapper 方法、TypeHandler、插件或 ID 生成器影响使用该基础的每个模块。优先使用本地解决方案。
- ActiveRecord、`Db` 和 `SimpleQuery` 是小规模或已有仓库模式的可选工具；它们不得绕过 Service 事务、权限、租户、审计或缓存规则。

## DDL 与多数据源

- 自动 DDL 不是迁移策略。使用已接受的 Flyway/Liquibase 或仓库迁移流程配以回滚和历史数据审查。
- 对于多数据源，验证每个 `SqlSessionFactory` 具有所需的 Mapper 路径、TypeHandler、插件和事务管理器。
- 跨数据源事务、读/写路由和插件顺序是高影响架构决策，非仅 Mapper 变更。

## Verification Focus

审查生成差异、ID 唯一性、扩展 scope、迁移启动、每数据源注册、事务行为和回滚或恢复边界。

## Evidence Focus

标识生成或自定义扩展文件、受影响的模块和数据源、所选 ID 契约、迁移证据以及回滚或恢复结果。

## 非选择规则

- 除非任务拥有其中一个高影响扩展，否则不要为普通实体或 CRUD 工作选择此参考。
- 不要仅为减少本地样板或绕过已有 Service 边界而引入扩展。
- 将部署和密钥管理决策保留在 RuntimeDelivery 和 Deploy 契约中。

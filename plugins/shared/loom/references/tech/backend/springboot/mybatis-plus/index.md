# MyBatis-Plus 参考路由

## 何时使用

此 profile 仅在已接受的 TechnicalBaseline 选择 MyBatis-Plus 时适用，或已有项目扫描具有高置信度 MyBatis-Plus 证据如 `com.baomidou.mybatisplus`、`mybatis-plus-boot-starter`、`BaseMapper` 或 `MybatisPlusInterceptor`。

不要将其应用于 MyBatis-Flex、纯 MyBatis、JPA、Hibernate、Spring Data 或任意名为 `Mapper` 的类。

## 按归属路由

| 任务拥有的能力 | 阅读 |
|---|---|
| Starter、mapper 扫描、全局配置 | `configuration.md` |
| 实体映射、ID、逻辑删除、乐观锁、enum/JSON 字段、审计填充 | `mapping.md` |
| Mapper、Service、CRUD、批量、分页、流查询 | `crud.md` |
| 查询或更新 wrapper 和部分更新 | `wrappers.md` |
| 分页、租户、锁、动态表、防攻击 interceptor | `plugins.md` |
| SQL 片段、用户控制排序、注入风险 | `security.md` |
| 生成器、ID 生成器、SQL injector、ActiveRecord、Db Kit、多数据源、DDL | `extensions.md` |

通用 Spring Boot 运行时、Web、安全、日志和测试规则保留在已有的 Spring Boot 参考中。SQL dialect 行为保留在所选的 `tech/code/sql` 参考中。

## Implementation Focus

- 遵循已接受的后端和数据访问选择；不要在执行期间选择另一个 ORM。
- 复用仓库已有的 Mapper、Service、事务、迁移、权限、租户和审计约定。
- 保持高影响扩展任务拥有且显式。不要将生成器、全局 interceptor、自动 DDL 或多数据源设置作为附带清理添加。
- 在已有的代码质量证据中记录所选的参考文件和受影响的持久化行为。

## Verification Focus

证明任务拥有的映射、查询/更新语义、事务边界、租户或权限边界、provider 行为和失败路径。仅编译通过不证明 SQL、interceptor 或持久化行为。

## Evidence Focus

记录为何选择此 profile、加载了哪些子参考以及每个验证结果覆盖哪些任务拥有的持久化行为。

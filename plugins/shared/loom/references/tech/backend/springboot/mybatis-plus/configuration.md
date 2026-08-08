# MyBatis-Plus 配置

## 何时使用

当已接受的 Spring Boot 持久化设计选择 MyBatis-Plus 且任务拥有依赖、mapper 扫描、session-factory 或 interceptor 配置时使用此参考。

## Implementation Focus

保持配置变更与仓库所选的 Spring Boot 和数据库 provider 对齐。不要将 mapper 配置任务扩展为 ORM 或迁移变更。

## 依赖与扫描

- 使用与已接受的 Spring Boot 主版本和仓库构建文件兼容的 starter 和 MyBatis-Plus 版本。
- 复用已有的依赖管理和包布局。不要同时添加 Boot 2 和 Boot 3 starter。
- 在更改配置之前确认 `@MapperScan`、mapper 接口、XML 位置和活跃的 `SqlSessionFactory`。
- 当仓库约定如此时，mapper 必须扩展项目所选的基础抽象，通常是 `BaseMapper<Entity>`。
- 在没有任务拥有的迁移理由的情况下，不要在同一模块中引入第二种 Mapper 或 Service 风格。

## Interceptor 注册

在使用该能力的每个 `SqlSessionFactory` 上注册 `MybatisPlusInterceptor`。将 provider 特定设置（如分页数据库类型）保留在所选持久化配置中。

组合分页、租户、数据权限、动态表、乐观锁、非法 SQL 和防攻击 interceptor 时验证 interceptor 顺序。不要假设在一个工厂上注册的插件保护另一个。

## 安全默认

- 在仓库已使用类型化配置的地方通过类型化配置绑定相关设置。
- 将凭证和生产端点排除在已提交配置之外。
- 不要启用自动 schema 变更作为已接受迁移工具的替代。
- 不要将 mapper 扫描或插件注册视为权限或租户边界。

## Verification Focus

检查依赖解析、mapper 发现、XML namespace/path 对齐、interceptor 注册、provider 配置和活跃 profile 的启动行为。

## Evidence Focus

记录所选的 starter、扫描根、session factory、活跃插件以及证明配置有效的聚焦启动或持久化检查。

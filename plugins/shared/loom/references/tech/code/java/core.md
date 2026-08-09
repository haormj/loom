# Java 核心实现质量

本文件不是 Java 风格指南；它告诉 agent 如何在 Loom 任务中做出 Java 实现决策。

## When To Use

- 任务变更了 Java 领域、服务、DTO、验证、异常、配置或控制器相邻代码。
- 项目是常规 Spring MVC、Spring Boot 服务代码，或不是特定响应式、仅持久化或仅安全的 Java 后端逻辑。
- 优先使用仓库约定。仅当现有项目已支持或任务显式拥有该基础时才引入 record、sealed class、builder、Lombok、MapStruct 或包分层。

## Implementation Focus

- 保持控制器代码面向传输。控制器应解析路径/查询/主体、触发验证、调用服务/用例并映射结果。将业务状态转换、资格检查和跨实体不变式放在服务/领域代码中。
- 不要通过 API 响应直接暴露 JPA 实体。在传输边界使用请求/响应 DTO 或 record，特别是当实体具有延迟关联、内部标志、审计字段或仅持久化标识符时。
- 在最早的拥有边界放置输入验证。对简单形态约束使用 Jakarta Bean Validation，对需要 repository 读取或状态检查的业务规则使用显式服务/领域验证。
- 使用项目现有的错误契约建模业务失败。如果不存在且此任务拥有 API 行为，使用小型异常层次加 `ProblemDetail`/错误响应映射；不要抛出带有面向用户消息的通用 `RuntimeException`。
- 仅当构建工具和源码兼容性已允许时使用 Java 21 特性。为不可变 DTO 优先使用 record，为封闭领域状态优先使用 sealed 层次，但不要将可变的框架绑定实体重写为 record。
- 通过 `application.yml`、环境变量或类型化配置属性外部化运行时值。不要在服务中硬编码 URL、端口、凭据、功能标志、日期截止或文件系统路径。
- 保持映射逻辑足够显式以便检查。对于小型 DTO，手写映射即可；对于重复映射，遵循仓库的 mapper 约定而非引入新的 mapper 库。
- 保留事务所有权。变更业务状态的服务方法应拥有事务边界或调用现有的事务用例；不要在控制器代码中分散写入。
- 优先使用构造函数注入和 final 依赖。不要添加字段注入或静态服务查找。
- 当添加 ID 或状态字符串时，在项目已使用它们的地方使用领域特定类型/枚举；避免为状态机或业务类别使用自由格式字符串。
- 使用专业的生产包根。优先使用现有的 `src/main` 包根，然后是 Gradle `group` 或 Maven `groupId`，然后是确认的组织/产品命名空间。如果都不存在，从仓库或确认的项目名派生 `app.<project_slug>`；仅当无法派生稳定 slug 时才使用 `app.generated`。
- 不要在 `com.example`、`org.example`、`net.example`、`io.example`、`com.company`、`org.company`、`com.demo`、`org.demo`、`com.sample` 或 `org.sample` 等占位符根下创建生产源码包。

## Verification Focus

- 运行仓库的 Java 构建/测试命令：根据项目选择 `./gradlew test`、`./gradlew check`、`./mvnw test` 或 `./mvnw verify`。
- 为新业务规则添加服务/领域测试，包括至少一个业务阻止或验证失败路径。
- 如果控制器可见行为变更，添加控制器/API 测试或运行时探针以证明状态码、响应体和错误形态。
- 如果添加了配置，在可行时验证默认本地值和一个覆盖路径。
- 如果变更使用 Java 21 语言特性，编译步骤必须证明配置的 source/target 支持它们。
- 对于新的 Java 源码根，验证包声明与选择的基础包和目录布局匹配。

## Evidence Focus

- 在证据总结中，说明保持清晰的 Java 边界，如 `controller -> service -> repository`、DTO/实体分离、验证所有权或异常映射。
- 创建新的 Java 文件时，说明选择的基础包来源：现有包根、构建 group 元数据、确认的命名空间或回退 `app.<project_slug>`。

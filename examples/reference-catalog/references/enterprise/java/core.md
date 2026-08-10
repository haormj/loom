# Acme 企业 Java 核心实现质量

本文件是企业示例参考,演示如何用 `replace_item_entry` 将 vendor 的 `java/core`
改写为指向企业内部参考文件。它替代默认的 `tech/code/java/core.md`。

## When To Use

- 任务变更了 Java 领域、服务、DTO、验证、异常、配置或控制器相邻代码。
- 项目使用 Acme 企业 Java 规范(基于 Spring Boot,但不用 Spring Security/Reactive)。
- 优先使用仓库约定与 Acme 内部编码规范。

## Implementation Focus

- 保持控制器代码面向传输。控制器应解析路径/查询/主体、触发验证、调用 service 并映射结果。
- 使用 Acme 标准异常层次 `AcmeBusinessException` + `ProblemDetail` 映射;不要抛出通用 `RuntimeException`。
- DTO 优先使用 record;实体保留 JPA 注解但不在 API 响应中直接暴露。
- 通过 `application.yml` 或环境变量外部化运行时值;不要硬编码 URL、端口、凭据。
- 使用 Acme 标准包根 `com.acme.<product>.<layer>`;不要使用占位符根。
- 事务边界由 service 层拥有;不要在控制器中分散写入。
- 依赖注入使用构造函数 + final;不使用字段注入。

## Verification Focus

- 运行 `./gradlew test` 或 `./mvnw verify` 证明变更可编译且测试通过。
- 为新业务规则添加 service 测试,包括至少一个 `AcmeBusinessException` 路径。
- 如果控制器可见行为变更,添加控制器/API 测试证明状态码与错误形态。
- 验证包声明符合 Acme 标准包根约定。

## Evidence Focus

- 在证据总结中,说明遵循 Acme Java 边界:`controller -> service -> repository`、DTO/实体分离、`AcmeBusinessException` 映射。
- 创建新 Java 文件时,说明使用 `com.acme.<product>.<layer>` 包根。

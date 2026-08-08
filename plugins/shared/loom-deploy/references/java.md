# Java 部署参考

当实现或修复 Java 家族项目的 loom deploy 支持时，使用本参考文档。

## 扫描器信号

- `pom.xml` 或 `mvnw` 标识 Maven 项目。
- `build.gradle`、`build.gradle.kts`、`settings.gradle`、`settings.gradle.kts` 或 `gradlew` 标识 Gradle 项目。
- `org.springframework.boot` 或 `spring-boot` 依赖/插件信号标识 Spring Boot。
- `io.quarkus` 信号标识 Quarkus。
- `io.micronaut` 信号标识 Micronaut。
- Java 版本信号可能出现在 Maven 属性中，如 `java.version`、`maven.compiler.release` 或 `maven.compiler.target`；Gradle 信号包括 `sourceCompatibility`、`targetCompatibility` 和 toolchain `languageVersion`。
- `application.properties` 中的 `server.port`，或 `server: port:` 风格的 YAML，应成为运行时端口。Java web 默认端口为 8080。

## 模板规则

- 使用多阶段 Dockerfile。
- 存在项目 wrapper 时优先使用：
  - Maven：`./mvnw -DskipTests package`，否则 `mvn -DskipTests package`。
  - Gradle：`./gradlew build -x test`，否则 `gradle build -x test`。
- 从包含 wrapper/build 文件的目录运行 wrapper 命令。如果服务根为 `service/`，Dockerfile 必须在调用 `./gradlew` 或 `./mvnw` 之前 `WORKDIR /app/service`。
- 使用受维护的 Eclipse Temurin 镜像：
  - Maven builder：`maven:3-eclipse-temurin-<major>`。
  - Gradle builder：`gradle:8-jdk<major>`。
  - Runtime：`eclipse-temurin:<major>-jre`。
- 项目未声明版本时，默认 Java 主版本为 21。
- 从 `target` 或 `build/libs` 复制第一个可运行 jar，排除 `*-plain.jar`、`*-sources.jar` 和 `*-javadoc.jar`。
- 在生成的 Compose/runtime 环境中为 Spring Boot 兼容性同时设置 `PORT` 和 `SERVER_PORT`。
- 如果前端构建输出必须由 Spring Boot 提供，在打包 jar 之前将该输出复制到 `src/main/resources/static` 或生成的构建暂存区；不要在 runtime 镜像中运行独立的前端 dev server。

## 依赖服务

- 从 JDBC URL、`postgresql`、`org.postgresql`、Flyway/Liquibase 迁移配置或 Spring datasource 设置检测 Postgres。
- 从 JDBC URL、`mysql`、`mariadb` 或驱动依赖检测 MySQL/MariaDB。
- 从 `spring-data-redis`、`lettuce` 或 `jedis` 检测 Redis。
- 从 `mongodb` 或 Spring Data MongoDB 检测 MongoDB。
- 从 `amqp`、`spring-rabbit` 或 RabbitMQ 配置检测 RabbitMQ。
- 从客户端依赖或 endpoint 变量检测 Elasticsearch/OpenSearch。

## 持久化与迁移

- 使用 Compose 依赖服务的 JDBC URL 必须使用服务 DNS 名，而非 `localhost`。
- 文件数据库 URL（如 SQLite、H2 file、HSQLDB file 和 Derby file）必须使用 `DeploymentSpec.storageFacts` 中选择的可写 `containerPath`；不要假设固定目录。
- 当检测到 Flyway 或 Liquibase 时，将迁移工具视为本地部署的 schema 拥有者。已知会误读文件数据库类型亲和性的框架 schema 验证应使用安全的本地生成 env 覆盖来禁用或降级，而非导致容器启动失败。
- 不要为每个 Java 应用假设 SQLite。仅当仓库配置或生成的 env 明确指向文件数据库时才使用此路径。

## 修复说明

- 如果构建找不到 wrapper 脚本，回退到 builder 镜像中已安装的 Maven/Gradle 命令。
- 如果找不到最终 jar，检查构建输出目录并在选择应用 jar 之前排除 classifier jar。
- 如果 Spring Boot 容器启动但 healthcheck 失败，验证 `SERVER_PORT`、`server.address`、profile 特定配置，以及应用是否需要数据库迁移或密钥。
- 如果 Gradle 构建因 daemon 或缓存问题失败，在更改应用代码之前禁用 daemon 或使用全新的生成镜像重试。
- 如果 Java 构建上下文遗漏了同级前端或共享模块，同时修复 Compose 构建上下文和 Dockerfile 复制路径。

## 扫描器信号到部署事实

在生成文件之前，将 Java 扫描器证据转换为部署事实：

- Maven/Gradle build 文件路径成为服务根和 manifest ref。
- Wrapper 脚本仅当它们位于构建上下文内时才成为首选构建命令事实。
- Spring Boot、Quarkus、Micronaut、servlet 容器或 CLI 信号决定服务是否暴露 HTTP。
- Java 版本属性/toolchain 选择 builder/runtime 镜像主版本。
- `server.port`、profile 配置、Actuator 配置和文档成为运行时端口和 healthcheck 候选。
- Flyway/Liquibase、datasource 配置、JDBC URL 和驱动依赖成为持久化/依赖事实。
- 同级/根目录下的前端资产仅当构建/打包路径可以包含它们时才成为后端服务前端事实。

## 生成的资产预期

生成的 Java 资产应显示：

- 带有构建和 JRE runtime 阶段的多阶段 Dockerfile。
- `WORKDIR` 与包含 `pom.xml` 或 `build.gradle*` 的目录对齐。
- 构建上下文足够宽，以容纳 wrapper 脚本、build 文件、同级模块和前端资产（当拓扑需要时）。
- 可运行 jar 选择，排除 `*-plain.jar`、sources 和 javadoc artifact。
- Compose env 包含 `PORT` 和框架特定端口变量（如检测到 Spring Boot 时的 `SERVER_PORT`）。
- 文件数据库 URL 指向挂载的可写容器路径；服务数据库使用 Compose DNS 名。
- 迁移感知的本地默认值避免在迁移工具拥有 schema 创建时因 schema 验证差异而阻塞启动。

## 修复边界

在以下情况下修复生成的 Java 部署资产：

- 上下文/workdir 无法看到 wrapper 脚本或 build 文件。
- 构建使用了 Docker 构建上下文之外的 wrapper 路径。
- Runtime 选择了不可运行的 classifier jar。
- 容器端口/env 与 Java runtime 端口不匹配。
- 依赖 URL 在容器内使用了 `localhost`。
- 生成的本地文件数据库路径未挂载或不可写。

在部署资产修复期间不要修改应用 entity 映射、迁移、profile 或源配置，除非 MCP 操作路由到执行修复。

# 环境诊断参考

当实现或修复与环境变量、密钥、框架配置或生成的 Compose `environment` 相关的 loom deploy 行为时，使用本参考文档。

## 扫描器规则

从以下来源记录变量名：

- `.env.example`、`.env.sample`、`.env.local.example`、`.env.template` 和 `.env.dist`
- 仅本地 `.env`、`.env.local`、`.env.development` 和 `.env.production` 的名称
- 源代码引用，如 `process.env.X`、`import.meta.env.X`、`os.getenv("X")`、`System.getenv("X")`、`Environment.GetEnvironmentVariable("X")`、`getenv("X")` 和 `ENV["X"]`
- 框架必需变量，如 Laravel `APP_KEY`、Rails `SECRET_KEY_BASE`、Django `SECRET_KEY` 和 NextAuth `NEXTAUTH_SECRET`

不要读取、打印、复制或注入真实的本地 `.env` 值。本地 `.env` 文件仅证明变量名在开发者机器上存在。

## 必需 vs 可选

当 loom 生成明显的运行时默认值时将其视为可选，如 `PORT`、`NODE_ENV`、`RAILS_ENV`、`RACK_ENV`、`SERVER_PORT` 和 `ASPNETCORE_URLS`。

将公共前端 env 名称（如 `NEXT_PUBLIC_*`、`VITE_*` 和 `PUBLIC_*`）视为已引用但非启动必需，除非日志证明并非如此。

当密钥、令牌、密码、key、JWT/session/cookie 变量和连接 URL 被示例或源代码引用时视为必需，除非 loom 已生成安全的本地默认值。

## 生成的默认值

生成的 Compose 可以包含：

- 运行时默认值，如 `PORT`
- 依赖服务连接值，如 `DATABASE_URL`、`REDIS_URL`、`MONGODB_URL` 和相关服务 URL
- 仅本地占位符，用于启动本地预览所需的常见框架密钥
- 容器安全的文件数据库 URL，当项目已指向本地文件数据库（如 SQLite、H2 file、HSQLDB file 或 Derby file）时
- 框架覆盖变量，使生成的本地容器与生成的运行时一致，如 `SERVER_PORT`、`ASPNETCORE_URLS` 或安全的本地 profile 标志

生成的占位符不是生产密钥。它们的存在仅为了使本地部署可诊断和可运行。

## 环境事实流

扫描器证据在资产生成之前成为部署事实：

- Env 示例名成为 `environment.expectedNames`。
- 源代码 env 引用成为 `environment.referencedNames`。
- 已知安全的本地默认值成为生成的 Compose 值。
- 真实密钥名保持为诊断，不得从本地文件填充。
- 依赖事实决定连接 URL 形态和服务名。
- 文件数据库事实决定可写容器路径和卷挂载。

生成的 Compose 应只包含由这些事实支持的值。如果某个变量在事实中不存在且所选运行时模板不要求它，不要发明它。

## 文件数据库与本地状态

对于本地文件数据库，容器路径必须在 `DeploymentSpec.storageFacts` 选择的可写挂载目录内。Compose 应创建该事实命名的卷。不要将容器指向仅存在于开发者机器上的主机相对路径。

对于 Spring Boot 加 JPA/Flyway/Liquibase 风格技术栈，不要假设 Hibernate schema 验证对所有本地文件数据库是权威的。如果生成的部署正在提供容器化的文件数据库 URL 且迁移工具拥有 schema 创建，优先使用安全本地覆盖来防止 schema 验证在应用启动前因 SQLite/H2 类型亲和性而失败。

当生成依赖服务时，应用 URL 必须使用 Compose 服务名（如 `postgres`、`mysql` 或 `redis`），而非 `localhost`。面向浏览器的前端 env 可以使用公共代理路径；容器到容器的 env 必须使用服务 DNS 名。

文件数据库处理并非仅针对 SQLite：

- 文件数据库路径被容器化到 `DeploymentSpec.storageFacts` 声明的路径；该路径不是通用的 `/app/data` 约定。挂载匹配的命名卷时保留 URL 前缀和查询/选项。
- H2 file、HSQLDB file、Derby、LiteFS 支持的 SQLite 和类似的本地文件存储仍然需要可写挂载路径。
- 如果应用配置命名了主机路径如 `./data/app.db`，将其转换为容器路径并在父目录挂载卷。
- 如果存在迁移，让迁移工具为本地部署初始化 schema，除非仓库配置明确禁用了它。

## 服务依赖 URL

从服务事实生成依赖 URL：

- Postgres：host `postgres`，port `5432`，生成的本地 user/password/database。
- MySQL/MariaDB：host `mysql` 或 `mariadb`，port `3306`，生成的本地 user/password/database。
- Redis：host `redis`，port `6379`。
- MongoDB：host `mongo` 或 `mongodb`，port `27017`。
- RabbitMQ：host `rabbitmq`，端口保持内部，除非明确为公共。
- MinIO/S3-compatible：endpoint 使用 Compose 服务 DNS 名和内部端口。

框架特定的变量名可以包装同一服务 URL。检测到时使用框架期望的配置名，但保持底层 host/port 与 Compose 一致。

## 框架本地安全默认值

安全本地默认值可以解除本地预览阻塞，而无需假装是生产配置：

- Spring Boot：`SERVER_PORT`，生成时的本地 datasource URL/driver，以及容器化本地启动所需的迁移/JPA 标志。
- Django：`SECRET_KEY`、`DEBUG=1`，本地容器访问的 allowed hosts，以及生成时的数据库 URL。
- Rails：`SECRET_KEY_BASE`、本地数据库 URL 和可写的 storage/log 路径。
- Laravel：`APP_KEY`、`APP_ENV=local`、`APP_DEBUG=true`、storage/cache 路径和生成的 DB/Redis URL。
- ASP.NET Core：`ASPNETCORE_URLS`、`ASPNETCORE_ENVIRONMENT=Development` 和来自生成依赖的连接字符串。
- NextAuth/Auth.js：仅当应用需要它来启动时，使用本地 `NEXTAUTH_SECRET` 或等效值。

不要为未检测到的框架添加框架默认值。

## 修复指导

当 `environment.missing` 非空时，在编辑 Dockerfile/Compose 之前检查它。如果缺失的变量可以安全地为本地部署生成，仅将其添加到生成的 Compose 中。如果它是真实凭证，向用户请求安全的本地值或解释阻塞原因。

如果日志提到缺失 env、缺失密钥、无效配置、app key、secret key base、database URL、auth secret、JWT secret 或凭证，将日志与 `DeploymentSpec.environment` 比较，更新生成的部署文件或请求用户提供值。

如果日志提到缺失数据表、待执行迁移、schema drift、Prisma 迁移错误、Django/Rails/Laravel 迁移错误、Flyway 或 Liquibase，将日志与 `DeploymentSpec.bootstrap` 比较。将 bootstrap 命令视为仅诊断指导；运行前需询问。

不要将每个启动错误都变成用户确认。如果失败可以在生成的 Compose/Dockerfile 中用安全本地默认值修复且受影响文件可编辑，则修复生成的部署资产。仅在需要真实凭证、破坏性状态更改或编辑受保护的用户资产时才询问用户。
## 前端 API 环境

前端 API 环境变量不是全局部署默认值。Loom 仅注入仓库源代码实际使用的环境键，且仅当源代码级请求构造证明该键提供了相对后缀的公共 base 时。当源代码已经发送完整的已接受接口路径时，注入值为空，因此诸如 `/api` 的 fallback 不能产生 `/api/api/...`。检测到的请求构造的冲突或未证明绑定在镜像生成前被阻止；没有可检测 API 请求的前端仍然可部署而无需注入。

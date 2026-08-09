# Docker Compose 部署参考

当实现或修复生成的 Compose 文件或围绕现有 Dockerfile 的包装时，使用本参考文档。

## 生成规则

- 根据 DeploymentSpec 源模型生成服务，而非硬编码的单服务假设。
- 为应用服务使用源模型服务 id。对于单服务，`app` 是可接受的。对于前端/后端形态，保持独立的前端和后端服务 id，并让公共入口服务拥有预览 URL。
- 将每个服务的 `build.context` 设为该服务/提供者的 DeploymentSpec 构建上下文，将 `build.dockerfile` 设为相对于该上下文的 Dockerfile 路径。不要将 Dockerfile 指向遗漏其 package/构建文件的上下文。
- 只发布 `DeploymentSpec.runtime.ports` 中 `internalOnly=false` 的公共运行时端口。
- 依赖服务应使用 Compose 内部网络和 `expose`，而非主机 `ports`，以避免本地冲突。
- 为有状态依赖（如 Postgres、MySQL、MongoDB、Redis、MinIO、RabbitMQ 和 Elasticsearch）使用命名卷。
- 仅为本地开发默认值生成环境变量。不要生成真实密钥。
- 真实的本地 `.env` 值不得复制到生成的 Compose 中。使用环境诊断仅记录变量名。
- 对于常见框架启动密钥（如 Laravel `APP_KEY`、Rails `SECRET_KEY_BASE`、Django `SECRET_KEY` 或 NextAuth `NEXTAUTH_SECRET`），安全的本地占位符是可接受的；它们不是生产凭证。
- 优先使用 map 风格的 `environment` 值，以便修复时易于阅读和修补。
- 使用 `depends_on` 控制依赖顺序。仅为定义了 healthcheck 且本地 Docker Compose 版本支持的服务添加健康条件。
- 为生成的长期运行服务使用 `restart: unless-stopped`。

## 拓扑感知 Compose 契约

Compose 必须实现 Loom 已准备的拓扑。不要从文件名或最近的 Docker 错误推断不同的形态。

公共入口：

- 由 `topology.publicEntryServiceId` 指定的服务拥有主机端口发布和预览 URL。
- 静态/前端网关发布面向浏览器的端口并在内部路由 API 路径。
- 后端服务应用直接发布后端端口，不需要代理路由。
- API-only 服务可以发布 API/健康端口，无需假装提供 SPA。

内部后端：

- 前端网关后方的后端/API 服务使用 `expose`，而非主机 `ports`，除非运行时端口明确为公共。
- 面向浏览器的前端配置指向公共代理路径，如 `/api`。
- 容器到容器的配置指向 Compose 服务 DNS，如 `http://backend:8080`。
- API 代理位置必须由生成的网关配置表示，而不仅是环境变量。

依赖：

- 依赖服务使用稳定的 Compose 服务名、有状态时使用命名卷、`expose` 和生成本地凭证。
- 应用连接 URL 使用依赖服务 DNS 名。容器内的 `localhost` 指向同一容器，对于依赖几乎总是错误的。
- `depends_on` 表示启动顺序。仅在依赖服务声明了 healthcheck 时添加健康条件。

多端口：

- 当 `DeploymentSpec.runtime.ports` 声明了多个应用端口时，多端口是有效的。
- 每个公共应用端口使用分配的 `hostPort` 发布一次。
- 即使依赖有知名主机端口，也保持依赖端口为内部。
- Healthcheck、预览 URL、API 路由和最终部署结果必须引用相同的已分配公共端口方案。

## 源模型形态

单服务项目：

- 一个应用服务拥有 build/start/runtime 配置。
- 发布 `runtime.ports` 中解析出的主机端口。
- 如果项目没有 HTTP 服务器，省略预览 URL 期望，而不是伪造一个。

前端加后端项目：

- 优先使用公共前端/静态服务和内部后端 API 服务。
- 仅当生成的栈包含已验证的代理配置时，才通过公共入口服务路由 API 路径（该代理配置需在 SPA fallback 之前得到证明）。
- 后端端口保持内部，除非运行时契约明确标记为公共。
- 当存在代理时，前端运行时环境应指向代理路径，而非部署后在浏览器内会失败的主机后端 URL。

后端服务前端加 API 项目：

- 一个后端服务拥有公共预览和 API 路径。
- 前端构建输出必须在打包或运行时启动前复制到后端 artifact/static 目录中。
- 除非源模型有独立的前端服务，否则不要生成前端网关代理。

现有 Dockerfile 包装：

- 用 Compose 包装现有 Dockerfile，但保持 Dockerfile 受保护。
- 将 `build.context` 与 Dockerfile 的目录假设匹配。应用本地 Dockerfile 通常期望自己的目录；生成的 workspace Dockerfile 可能需要 workspace 根目录。
- 不要仅因为包装更容易就向用户 Compose 文件中添加服务。

## 生成的资产预期

对于生成的提供者，有效的 Compose 文件应使以下关系可见：

- 每个 `sourceModel.services[].serviceId` 都有一个 Compose 服务。
- 每个应用服务有 `build.context`、`build.dockerfile`、环境块和预期的容器端口接线。
- 公共入口服务是唯一拥有预览主机端口发布的服务，除非另一个公共运行时端口是明确的。
- 内部服务在应用环境变量中使用服务 DNS 名，内部端口使用 `expose`。
- 依赖服务有生成本地凭证、有状态时的卷，且没有意外的主机端口发布。
- 网关/代理服务挂载或复制生成的代理配置，并在 SPA fallback 之前路由 API 路径。

## 端口方案

将 `DeploymentSpec.runtime.ports` 视为主机/容器端口发布的唯一真实来源：

- `hostPort` 是 Loom 选择的实际可用本地端口。在 Compose `ports` 和最终预览详情中使用它。
- `preferredHostPort` 仅是起始偏好。当 `hostPort` 不同时不要硬编码它。
- `containerPort` 必须与容器内的应用进程和 Dockerfile `EXPOSE` 匹配。
- `purpose` 表示端口是预览、api、service 还是 dependency。
- `internalOnly=true` 表示不发布到主机。使用服务 DNS 名和 `expose` 进行内部通信。

当存在多个公共应用端口时，每个显式公共运行时端口发布一次。不要通过发布依赖端口来解决应用连接问题；改为修复内部服务 URL。

## 现有资产

- 根级 `compose.yaml`、`compose.yml`、`docker-compose.yaml` 和 `docker-compose.yml` 受保护。
- 根级 `Dockerfile` 受保护；生成的 Compose 可以包装它，但未经批准不得编辑。
- 如果存在用户拥有的 Compose 文件，验证并报告它。不要自动将生成的服务合并到其中。
- 在报告状态、日志或健康之前分析现有 Compose 服务。优先选择名为 `app`、`web`、`api`、`server`、`backend`、`frontend`、`www`、`site` 或 `gateway` 的类应用服务，特别是当它们有 `build`、已发布的 HTTP 端口和 `depends_on` 时。
- 避免将类依赖服务（如 Postgres、MySQL、Redis、MongoDB、RabbitMQ、Elasticsearch、MinIO、Kafka、localstack 或邮件服务）选为主应用，即使它们发布了主机端口。
- 当选定的服务有已发布端口时，使用该主机/容器端口对作为预览 URL 和 healthcheck。如果不存在已发布端口，保持诊断明确，而不是猜测可达 URL。

## 健康与日志

- 应用健康探测属于 Loom 验证，除非已接受的运行时事实包含明确的安全健康路径。
- 不要猜测 `/` 或探测业务端点作为 healthcheck。只使用 `DeploymentSpec.runtimeContract.healthPath`、扫描器确认的安全探测或已接受的框架健康端点。
- 尊重已记录在 `DeploymentSpec`、`sourceModel.services[].healthcheckPath` 和拓扑验证中的 healthcheck 路径。不要在 Compose 修复期间发明无关的探测。
- 当不存在安全健康路径时，省略 Compose healthcheck，让预览/API 验证提供证据。
- 日志解析应针对现有 Compose 的选定应用服务，并在报告预览 URL 之前识别致命启动失败。
- 如果应用没有 HTTP 服务器，Compose 仍然可以 build/start 它，但部署结果不应发明 HTTP 预览 URL。

## 存储事实

- 文件数据库和有状态依赖必须从 `DeploymentSpec.storageFacts` 挂载。
- 存储事实命名提供者、拥有服务、卷、容器路径和适用的环境键。
- 不要将 `/app/data` 作为通用约定，也不要通过搜索生成的环境值来决定持久性。

## 修复线索

- `docker compose config` 失败通常涉及无效 YAML、错误的 env 形态、缺失文件、不支持的健康条件语法或错误的构建路径。
- 启动失败通常涉及错误的容器命令、缺失依赖 env、需要更多启动时间的依赖服务或端口不匹配。
- 端口发布失败通常意味着选定的主机端口已被占用；修复生成的 Compose，而非应用源。
- 构建上下文失败通常显示 `file not found`、缺失 lockfile、缺失包装脚本、缺失构建文件或缺失源目录。在更改 Dockerfile 命令之前，比较 Compose `build.context`、`build.dockerfile`、`DeploymentSpec.files` 和 `sourceModel.services[].root`。

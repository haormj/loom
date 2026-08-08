# 部署稳定性矩阵

当 Loom 选择 `deploy.matrix` 时使用本参考文档。它定义了生成的 spec、源模型、拓扑、Compose、Dockerfile、验证和修复必须达成一致的部署矩阵。

## 矩阵维度

每个 `loom.deployPrepare` 结果应可通过这些维度解释：

- 拓扑类别：`single_service_app`、`api_only_single_service`、`static_site`、`backend_served_frontend_api`、`frontend_gateway_backend_api`、`multi_service`、`existing_compose` 或 `existing_dockerfile_wrapper`。
- 运行时家族：Node、Java、Python、Go、.NET、PHP、Ruby、Static 或 Unknown。
- 仓库布局：根应用、同根 fullstack、拆分前端/后端、workspace 应用或现有资产。
- 提供者策略：生成的资产、现有 Compose 或现有 Dockerfile 包装。
- 端口方案：公共预览/API 端口、内部应用端口和依赖端口。
- 状态和依赖模型：无依赖、文件数据库卷、SQL/Redis 等依赖服务或受保护的外部服务。

不要将矩阵折叠为单一“一个容器服务一切”假设，除非事实表明同一运行时进程确实服务每个所需界面。

## 拓扑预期

`api_only_single_service`：

- 一个应用服务。
- API 探测路径可直接针对公共服务验证。
- 不需要 HTTP 代理路由。
- 不得发明前端/static 预览假设。

`single_service_app`：

- 一个公共应用服务暴露 HTTP 预览/根路径。
- 不生成额外的 API 代理或 static 网关。
- Compose、Dockerfile、healthcheck 和端口方案都指向同一服务/容器端口。

`static_site`：

- 一个公共 static 服务。
- 除非源模型包含后端/API 服务，否则不验证 API 路由。
- 使用 Nginx 或等效的 static server；不要将前端开发服务器作为已部署运行时运行。

`backend_served_frontend_api`：

- 一个后端/runtime 服务同时拥有预览流量和 API 路径。
- Static 资产在打包或运行时启动前构建/复制到后端 artifact 中。
- 不需要前端网关代理。

`frontend_gateway_backend_api`：

- 公共入口服务是前端/static 网关。
- 后端 API 服务为内部。
- API 路径必须在 SPA fallback 之前由公共入口代理。
- 面向浏览器的 API env 指向公共代理路径，而非 `localhost:<backend-port>`。

`multi_service`：

- 存在多个可部署服务，但不一定是前端/API 对。
- 公共和内部端口必须在 `DeploymentSpec.runtime.ports` 中明确。
- Compose 服务 id 必须与 `sourceModel.services[].serviceId` 匹配。

`existing_compose`：

- 用户 Compose 受保护。
- Loom 可以检查、验证和报告选定服务。
- Loom 不得在 prepare 或 repair 期间重写用户 Compose 文件。

`existing_dockerfile_wrapper`：

- 用户 Dockerfile 受保护。
- Loom 可以生成 Compose 包装。
- 包装构建上下文必须与 Dockerfile 的假设匹配。

## 端口矩阵

使用 `DeploymentSpec.runtime.ports` 作为唯一的主机/容器端口方案。

- `hostPort` 是 Loom 选择的实际可用本地端口。
- `preferredHostPort` 在分配后仅用于诊断。
- `containerPort` 必须与运行时进程和 Dockerfile `EXPOSE` 匹配。
- `internalOnly=true` 服务不发布到主机。
- 依赖服务使用 Compose DNS 名和 `expose`；不要通过发布依赖端口来使应用代码工作。
- 当源模型需要多个公共端口时允许，但每个必须有明确用途。

## 依赖矩阵

生成的依赖服务是本地部署便利设施，非生产基础设施。

- 文件数据库需要 `DeploymentSpec.storageFacts` 提供的可写 `containerPath` 和 `volumeName`；该目录是技术栈和仓库派生的。
- SQL/Redis/Mongo 等服务使用稳定的 Compose 服务名。
- 应用连接 URL 使用服务 DNS 名，而非 `localhost`。
- 除非 Loom 能提供安全本地占位符，否则真实凭证是阻塞项。

## 修复边界

当生成的文件与事实不一致时，修复可以修补生成的资产。修复不得通过发明不同的拓扑来补偿错误的矩阵。

- 事实/源/拓扑不匹配：修复 MCP 生成逻辑或与事实对齐的生成资产。
- 生成的 Dockerfile/Compose 不匹配：仅修复生成资产。
- 应用 build/start/runtime 失败：路由到部署执行修复。
- 受保护的现有资产不匹配：根据提供者策略报告受保护资产阻塞项或生成 fallback。

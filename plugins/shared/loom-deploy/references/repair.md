# 部署修复参考

当当前 Loom MCP 部署操作请求部署修复时，使用本参考文档。

## 失败类型

- `compose_config`：修复 Compose 语法、构建上下文路径、服务名、端口映射、env 形态或文件引用。
- `image_build`：修复生成的 Dockerfile 安装/构建命令、包管理器处理、被忽略的文件或构建上下文。
- `container_start`：修复运行时命令、暴露端口、host 绑定、缺失构建 artifact 或 production server 命令。
- `healthcheck`：修复 HTTP healthcheck 路径/候选、应用监听地址、启动时序、暴露端口或应用命令。
- 缺失环境诊断位于 `environment.missing`；在假设 Dockerfile、端口或 healthcheck 问题之前检查它们。
- Bootstrap 诊断位于 `bootstrap.tasks`；将 Prisma、Django、Rails、Laravel、Flyway 和 Liquibase 迁移命令视为建议性，除非用户明确批准运行它们。
- 如果用户批准 bootstrap 执行，使用 `loom.deployBootstrap` 携带 `kind` 和 `confirm: true`，而非手动运行迁移命令。
- 失败诊断位于 `diagnostics`；使用诊断 `code`、`evidence` 和 `suggestedAction` 来优先修复。这些诊断可能显示缺失原生包、缺失模块、端口冲突、依赖连接/认证失败、待执行迁移、缺失 env 或权限问题。
- `logs`：在编辑文件之前验证 Compose 项目/服务仍然存在。
- `docker_unavailable`：不要编辑文件。要求用户启动 Docker Desktop/Docker daemon，验证 `docker version` 从同一终端/会话可工作，并在 Docker 在 agent 聊天外可用但在其中不可用时启用完全本地访问或 Docker 命令权限。
- `registry_network`：不要编辑文件；Docker 无法到达或认证镜像 registry。要求用户重试、预拉取被阻止的镜像、配置 Docker registry 镜像/代理，或修复 registry 凭证/网络访问。
- `build_command_failed`、`start_command_failed`、`http_probe_failed`、`preview_not_verified`：如果当前 MCP 操作报告 `repairRoute=execution_repair`，不要编辑部署资产。执行返回的修复操作，写入其结果，使用返回的提交工具提交，然后通过返回的部署操作重试。
- `unknown`：在编辑之前从 stdout/stderr 分类。

## 修复决策树

对每次部署修复按此顺序执行：

1. 检查失败是否在生成资产之外。
   - Docker daemon 不可用、registry/network/auth 失败、缺失真实凭证和受保护用户资产不匹配是阻塞项或用户操作项。
   - 不要为这些失败编辑 Dockerfile/Compose。
2. 检查事实/源/拓扑一致性。
   - 比较 `factsRef`、`sourceModelRef`、`topologyRef` 和 `DeploymentSpec.runtime.ports`。
   - 如果拓扑为 `frontend_gateway_backend_api`，生成资产必须包含公共网关和 API 代理路由。
   - 如果拓扑为 `backend_served_frontend_api`，生成资产必须将前端输出打包到后端中，且不得发明代理需求。
   - 如果 spec 事实相互矛盾，MCP 生成器是错误的。不要通过在修复中发明不同拓扑来隐藏它。
3. 检查生成资产闭包。
   - Compose 服务 id 必须与 `sourceModel.services[].serviceId` 匹配。
   - Compose 构建上下文、Dockerfile 路径、Dockerfile `WORKDIR` 和 `COPY` 源必须是有效的路径闭包。
   - 运行时端口、`EXPOSE`、host 绑定和 healthcheck 路径必须与端口方案匹配。
   - 环境值必须来自环境/依赖事实。
4. 分类失败阶段。
   - Compose config 失败：修补 Compose 结构、路径、env 形态或不支持的语法。
   - Image build 失败：修补生成的 Dockerfile、ignore 文件、依赖安装或构建上下文。
   - Container start 失败：修补生成的命令、env、依赖 URL、绑定 host 或运行时 artifact 选择。
   - Healthcheck/proxy 失败：修补 healthcheck 候选、公共网关配置、路由顺序、绑定 host、启动时序或拓扑一致的端口接线。
5. 当日志证明应用代码/构建脚本是失败界面且 MCP 操作路由到 `execution_repair` 时，升级到执行修复。
   - 在该路由中，不要编辑部署资产。
   - 执行返回的合成任务，提交结果，然后重试返回的部署步骤。

此决策树防止盲目的 Dockerfile 编辑。Dockerfile 修复仅在源模型和 Compose 构建上下文已检查后才正确。

## 平台特定的原生依赖失败

- 如果日志提到 `@next/swc-linux-*`、`@tailwindcss/oxide-linux-*`、`tailwindcss-oxide.linux-*.node`、`lightningcss.linux-*.node`、`sharp`、`esbuild`、`rollup-*` 或类似的原生可选包，将 OS/libc/CPU 视为修复的一部分。
- 除非项目已知可在 Alpine 上工作，否则优先为 Next.js/Tailwind 应用使用 glibc 镜像（如 `node:22-slim` 或项目检测到的 `node:<major>-slim`）。
- 如果包 lockfile 在 macOS 上生成且只包含 `darwin-*` 可选包，优先在用户批准下将所需的 Linux 可选依赖添加到项目 lockfile。当修复范围限于生成资产时，修补生成的 Dockerfile 安装步骤使 Linux 包在镜像内安装。
- 除非用户明确想要仅 dev 部署，否则不要通过将主机 `node_modules` bind-mount 到 Linux 容器来解决原生模块失败。主机 `node_modules` 通常是平台特定的。

## Java 构建失败

- 如果 Maven 或 Gradle wrapper 脚本缺失或不可执行，在编辑应用代码之前使用 builder 镜像已安装的 `mvn` 或 `gradle` 命令。
- 如果找不到可运行 jar，检查 `target` 或 `build/libs` 并避免选择 `*-plain.jar`、`*-sources.jar` 或 `*-javadoc.jar`。
- 如果 Spring Boot 在错误端口启动，验证生成的 `PORT`、`SERVER_PORT` 和任何项目 `server.port` 设置。

## .NET 构建失败

- 如果 `dotnet publish` 成功但 runtime 找不到 DLL，检查 `.csproj` assembly 名和已发布的输出，然后仅更新生成的启动命令。
- 如果 restore 因私有 NuGet feed 失败，请求凭证或安全的 `NuGet.Config`；不要将密钥烘焙到生成的部署文件中。
- 如果 ASP.NET Core 启动但 healthcheck 失败，验证 `ASPNETCORE_URLS`、HTTPS 重定向、应用是否在生成的容器端口上监听，以及是否应添加框架特定的健康路径。

## PHP 构建失败

- 如果 Composer install 因扩展缺失失败，在编辑应用代码之前更新生成的 Dockerfile 扩展安装块。
- 如果 Laravel 启动后返回 500，检查日志和 `environment.missing` 中缺失的 `APP_KEY`、storage/cache 权限、数据库连接错误或待执行迁移。
- 如果 Laravel 项目因前端资产被检测为 Node，将 `composer.json` 和 `artisan` 视为比 `package.json` 更高优先级的技术栈信号。

## Ruby 构建失败

- 如果 Bundler 因原生扩展失败，在编辑应用代码之前更新生成的 OS 包安装。
- 如果 Rails 启动后返回 500，检查日志和 `environment.missing` 中缺失的 `SECRET_KEY_BASE`、storage 权限、数据库连接错误或待执行迁移。
- 如果 Rails 项目因前端资产被检测为 Node，将 `Gemfile` 和 Rails 配置视为比 `package.json` 更高优先级的技术栈信号。

## 编辑规则

- 仅编辑 `editableFiles` 中列出的文件。
- `source-model.json`、`topology.json` 和 `facts.json` 是 MCP 生成的快照。当 Loom 暴露 `modelRepairRef` 时，将受控更正放入 `model-repair.json`，使用返回的 base fingerprint、`status=accepted` 和仅更正的 `sourceModel` 或 `topology`；Loom 在仅源模型更改时重新生成依赖拓扑，然后从该更正重新计算事实和生成资产。
- 如果 `editableFiles` 为空（因为失败路由到部署源执行修复），允许的编辑边界来自合成执行请求，而非部署修复。
- 将 `protectedFiles` 视为只读，除非用户明确批准编辑。
- 除非用户批准且当前修复操作无法在部署文件中解决，否则不要编辑应用源代码、package 脚本或环境文件。
- 不要自动运行迁移/bootstrap 命令。如果诊断指向缺失数据表或待执行迁移，解释 `bootstrap.tasks` 中的命令并请求批准。
- 不要读取、打印或将真实的本地 `.env` 值烘焙到生成的部署文件中。仅使用变量名和安全的本地占位符。
- 保持生成的文件位置在 `.loom/deployment/specs/generated/` 下。
- 不要使用修复来重写提供者策略。如果选定的提供者是生成的，修复生成资产。如果未强制的现有提供者不合适，Loom 应在修复之前回退到生成的。如果用户强制了现有提供者，清晰报告受保护资产问题。

## 生成优先修复姿态

修复是有界的 fallback，而非主模板引擎。在修补之前，将失败与以下内容比较：

- `sourceModelRef`：服务根、运行时类型、工作目录和包管理器
- `topologyRef`：公共入口服务、API 代理路径和验证路径
- `generatedFileRefs`：agent 可编辑的实际文件
- `DeploymentSpec.runtime.ports`：实际主机/容器端口分配

如果生成的模板结构上错误，以最小持久方式修复生成资产，并保持修复与源模型对齐。避免场景特定的修复，如硬编码某个框架的端口、某个文件夹名或某个数据库，除非源模型或诊断证明了该确切技术栈。

仅当下一个操作需要真实凭证、破坏性 bootstrap/迁移执行、更改受保护的用户 Compose/Dockerfile 资产或无法从仓库证据推断的决策时，才询问用户。

## 受保护资产边界

除非用户强制生成提供者或明确批准编辑，否则现有 Compose 和 Dockerfile 资产为用户拥有。

- 对于 `compose-existing`，检查并报告用户 Compose 拓扑。不要向其中注入生成的代理、依赖或 healthcheck 服务。
- 对于 `dockerfile-existing`，围绕受保护的 Dockerfile 生成或修复 Compose 包装和环境。当 Dockerfile 的假设仅是不便时不要更改它。
- 对于生成的提供者，`.loom/deployment/specs/generated/` 下的生成 Dockerfile/Compose/nginx 资产在返回时可编辑。生成的模型快照为只读；当源事实或拓扑错误时使用受控的 model-repair 文件。
- 如果受保护资产无法满足已接受的运行时契约，报告受保护资产阻塞项和确切所需的更改。仅当提供者策略允许时，Loom 才可回退到生成资产。

## 重试规则

- 对于全新的部署请求，使用 `loom.deployRun`；它准备缺失的 spec、构建、启动、验证、报告状态，并在完整流程无法完成时返回下一个修复操作。
- 每次修复编辑后，调用返回的 `retryTool`。对于资产修复，这是 `loom.deployUp`。
- `loom.deployUp` 使用当前的 `.loom/deployment/specs/local.json` 和生成资产重试。它不得重新生成 Dockerfile、Compose、nginx 或 dockerignore 文件。
- 不要将 `loom.deployRun` 作为修复重试调用。`deployRun` 是高级入口点，而修复重试是针对当前 spec 的执行步骤。
- 如果成功，对同一 `projectRoot` 调用 `loom.deployStatus`。
- 如果失败，对同一 `projectRoot` 再次调用 `loom.deployRepair` 并使用新请求。
- 同一失败签名的默认 `maxAttempts` 为 2；更改的失败签名创建新的有界修复评估。
- 默认 Docker Compose build/start 超时为 10 分钟，因为首次依赖安装在真实项目上可能很慢。
- 当 `attempts >= maxAttempts` 或下一个修复需要受保护文件时停止。

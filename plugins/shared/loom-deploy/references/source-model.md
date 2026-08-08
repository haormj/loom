# 部署源模型参考

当 Loom 选择 `deploy.source-model` 时使用本参考文档。它解释仓库证据如何成为可部署服务。

## 源模型契约

`DeploymentSourceModel` 由 Loom 生成，是部署资产生成的权威。

必需闭包：

- `services[].serviceId` 成为 Compose 服务 id 和 Dockerfile 后缀。
- `services[].role` 决定公共前端、后端 API 或单应用行为。
- `services[].root` 是仓库相对的源根。
- `services[].workingDirectory` 是当其与构建上下文不同时的容器内命令目录。
- `services[].runtimeKind` 选择技术栈生成器。
- `services[].packageManager`、`hasLockfile`、`manifestRefs` 和 `lockfileRefs` 驱动依赖安装命令。
- `services[].buildCommand`、`startCommand`、`outputDirectory`、`port` 和 `healthcheckPath` 是生成的服务事实。它们必须与 Dockerfile、Compose 和验证一致；运行时命令权威来自 `runtimeContract.commands.development`、`verification` 和 `deployment`。
- `manifestRefs` 和 `lockfileRefs` 必须只包含仓库扫描器确认的文件。绝不发出通配符或不存在的常规 manifest。
- healthcheck 路径是可选的。缺失意味着 Loom 从外部验证预览/API 探测；不得用 `/` 替换。
- `storageFacts` 是文件数据库和依赖卷挂载的唯一来源。不要从环境字符串或固定容器目录推断持久性。

Agent 不应从日志中重新发现此模型。如果模型错误，使用返回的 `modelRepairRef` 并说明哪个源事实不一致。不要直接编辑 `source-model.json` 或用无关拓扑替换它；Loom 验证 base fingerprint，在接受的模型修复后重新计算依赖事实并重新生成资产。

## 布局规则

根应用：

- 源根为 `.`。
- 构建上下文可以是仓库/应用根。
- Dockerfile 复制路径相对于根。

拆分前端/后端：

- 前端根通常为 `web`、`frontend`、`client` 或 `ui`。
- 后端根通常为 `service`、`backend`、`api` 或 `server`。
- 除非证明了后端服务 static 拓扑，否则前端和后端必须有独立的服务记录。

Workspace 应用：

- `apps/*`、`services/*` 或 `packages/*` 下的根需要 workspace 根作为构建上下文（当需要根 lockfile/workspace 清单时）。
- Dockerfile 必须在 build/start 命令之前切换到应用工作目录。
- 构建上下文、Dockerfile 路径、`COPY` 源和 `WORKDIR` 必须形成一个有效的三角。

同根 fullstack：

- 一个运行时项目同时包含后端和前端资产。
- 前端输出必须在运行时启动前复制到后端 artifact/static 目录。
- 除非源模型有独立的前端服务，否则不要创建前端网关。

## 运行时家族预期

Node：

- 从 lockfile 和 `packageManager` 元数据检测包管理器。
- 使用 production preview/server 命令而非 dev server，除非不存在 production 脚本。
- 在生成的镜像中保留 Linux 可选依赖处理。

Java：

- Maven/Gradle wrapper 或 build 文件决定包管理器。
- Spring Boot、Quarkus 和 Micronaut 需要运行时端口/env 一致性。
- 可运行 jar 发现必须避免 `*-plain.jar`、sources 和 javadoc jar。

Python：

- `requirements.txt`、`pyproject.toml`、`uv.lock`、`poetry.lock`、`manage.py`、`main.py`、`app.py` 或 `server.py` 标识应用根。
- 框架 entrypoint 必须绑定到 `0.0.0.0`。

Go：

- `go.mod` 标识模块根。
- 构建输出应为复制到 slim runtime 镜像中的单一运行时二进制。

.NET：

- `*.csproj`、`*.sln`、`global.json` 和 ASP.NET 包信号选择 SDK/runtime 镜像。
- 已发布的 DLL 选择不得假设固定的 assembly 名，除非检测到。

PHP：

- `composer.json`、`artisan` 和 `public/index.php` 标识 Composer/Laravel 风格应用。
- Runtime 应服务正确的 document root 且不需要主机 PHP。

Ruby：

- `Gemfile`、`config.ru` 和 Rails 配置标识 Bundler/Rails/Rack 应用。
- `tmp`、`log` 和 `storage` 等运行时目录必须为 Rails 风格应用存在。

Static：

- 现有 static 输出（如 `dist`、`build`、`public`、`out`、`_site` 或根 `index.html`）可直接服务。
- 不要为纯 static 应用发明 API 服务。

## 预检清单

启动前，验证应捕获：

- Compose 中缺失服务 id。
- 构建上下文中缺失服务根。
- 构建上下文中缺失 manifest 或 lockfile ref。
- Dockerfile `COPY` 源在构建上下文之外。
- Dockerfile `WORKDIR` 与服务根不对齐。
- Dockerfile `EXPOSE` 与 `sourceModel.port` 不对齐。
- 生成的 `.dockerignore` 排除了所需的 manifest 或 lockfile。

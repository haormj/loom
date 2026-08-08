# Workspace / Monorepo 部署指导

当 `loom.deployPrepare` 或 `loom.deployRun` 接收指向 monorepo 根而非单个应用目录的 `projectRoot` 时，使用本参考文档。

## 检测

将以下视为 workspace 根标记：

- `pnpm-workspace.yaml`
- 带有 `workspaces` 或 `workspaces.packages` 的 `package.json`
- `turbo.json`
- `nx.json`
- `lerna.json`
- `rush.json`

如果根已经有 Compose 文件、Dockerfile 或直接可部署的技术栈，使用根。否则，搜索可能的应用目录，如 `apps/*`、`packages/*`、`services/*`、`sites/*`、`web`、`frontend`、`backend` 和 `api`。

按显式部署资产优先排序候选，然后是可运行的框架/start 命令信号，然后是常见应用目录名。将选定路径和候选分数保存在 `DeploymentSpec.workspace` 中，以便 agent 可以解释或修复选择。

## 应用路径与构建上下文矩阵

在生成 Compose 或 Dockerfile 之前使用此矩阵：

- 根应用：应用路径 `.`，源根 `.`，构建上下文 `.`，生成的 Dockerfile 路径相对于根。
- 应用本地子目录：应用路径如 `service`、`backend`、`web` 或 `frontend`；源根是该目录；构建上下文是应用路径，除非需要祖先 lockfile/workspace 清单。
- 拆分前端/后端：应用路径是独立的源根；生成的 Compose 有独立服务，除非拓扑证明后端服务前端。
- 同根 fullstack：一个应用路径包含后端和前端构建输入；构建上下文保持在该根，Dockerfile 阶段将前端构建与后端 runtime 分离。
- Workspace package：应用路径在 `apps/*`、`packages/*` 或 `services/*` 下；当需要根 lockfile/workspace 清单时，构建上下文是 workspace 根；Dockerfile `WORKDIR` 在 build/start 之前切换到 package。
- 现有 Dockerfile：构建上下文遵循 Dockerfile 自身的假设；包装 Compose 不得选择使现有 `COPY` 路径无效的上下文。
- 现有 Compose：Compose 文件拥有服务上下文选择；Loom 在 prepare 期间报告它们而非替换。

选定的应用路径不总是构建上下文。构建上下文是包含生成的 Dockerfile 必须复制的所有文件的最小目录。

## 显式应用路径

`DeployToolInput.appPath` 覆盖自动 workspace 选择。它必须保持在 `projectRoot` 内并指向现有目录。

当仓库有多个可部署目标（如 `apps/web`、`apps/admin` 和 `services/api`）时使用显式应用路径。loom 仍在根 `.loom` 下存储一个当前本地部署；选择不同应用会重写当前生成的部署 spec/资产。

## 构建上下文

对于复用的应用本地 Dockerfile 和 Compose 文件，将构建上下文保持在选定的应用路径。用户编写的 Dockerfile 通常假设自己的目录作为上下文。

对于生成的 Node workspace Dockerfile，优先使用 workspace 根作为构建上下文，以便根 lockfile 和 workspace 清单对 npm/pnpm/yarn/bun 可用。将 `detectedStack.workingDirectory` 设为选定的应用路径，并使 Dockerfile 在运行应用 build/start 脚本之前切换到该目录。

对于生成的非 Node 技术栈，从源模型选择上下文：

- 所有构建文件在一个根下的应用本地服务 -> 应用根上下文
- 一个镜像将前端 static 资产复制到后端的前端/后端组合 -> 仓库或公共祖先上下文
- 多服务生成 Compose -> 每个服务可以使用不同的 Dockerfile 和 workdir，但每个 Dockerfile 路径必须从其 Compose 构建上下文有效

## 包管理器

当扫描选定的 Node 应用时，包管理器检测可以使用祖先目录中的 lockfile。这对于应用不携带自身 lockfile 的 pnpm/npm/yarn/bun monorepo 很重要。

对于 pnpm workspace，在安装之前复制 `pnpm-workspace.yaml` 和根 lockfile。没有它，`pnpm install --frozen-lockfile` 可能失败或安装不完整的 workspace 图。

## 修复说明

当 monorepo 部署失败时，首先检查这些字段：

- `workspace.appPath`
- `workspace.buildContextPath`
- `files.buildContextPath`
- `files.dockerfilePath`
- `detectedStack.workingDirectory`

常见修复是更正 Compose `build.context`、相对于该上下文的 Dockerfile 路径，或在 install/build/start 命令之前使用的 Dockerfile `WORKDIR`。

如果构建命令仅在从子目录运行时本地工作，将该子目录编码为生成 Dockerfile 中的 `WORKDIR` 或显式 `cd`。除非项目已有该应用的根级构建脚本，否则不要将 workspace 展平为一个根命令。

## 源根修复边界

当 workspace 部署失败时：

- Docker 构建中缺失 manifest 意味着构建上下文或 `COPY` 路径错误。
- 缺失 wrapper/build 脚本意味着 `WORKDIR` 错误或服务根被错误识别。
- 缺失同级 package/module 意味着上下文对于 workspace 依赖图太窄。
- 错误的公共服务意味着拓扑/源模型选择错误，而非 Compose 重试细节。

修复生成资产以匹配选定的源模型。如果源模型选择了错误的应用路径，报告该事实而非用宽泛的仓库复制来补偿。

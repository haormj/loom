# Dockerfile 部署参考

当实现或修复生成的 Dockerfile 或 Dockerfile 专用 ignore 文件时，使用本参考文档。

## 生成规则

- 当技术栈有构建步骤或编译 artifact 时，优先使用多阶段构建。
- 将生成的文件保存在 `.loom/deployment/specs/generated/` 下；不要覆盖用户拥有的根 `Dockerfile`。
- 为每个可部署应用服务生成 Dockerfile。除非源模型明确指出一个运行时进程同时服务两者，否则不要在一个镜像中合并无关的前端/后端命令。
- 将 `COPY`、依赖安装、构建和启动命令与 `sourceModel.services[].root`、`workingDirectory`、包管理器和构建上下文对齐。在 `.loom/deployment/specs/generated/` 旁生成的 Dockerfile 仍然针对 Compose 构建上下文运行。
- 使用技术栈参考已选择的显式、受维护的基础镜像，如 `node:22-slim`、项目检测到的 `node:<major>-slim`、`python:3.12-slim`、`golang:1.23-alpine`、Eclipse Temurin Java 镜像、Microsoft .NET 镜像、官方 PHP 镜像或官方 Ruby 镜像。
- 选择基础镜像时考虑平台。Debian/Ubuntu/slim 镜像使用 glibc；Alpine 使用 musl。原生可选依赖必须匹配容器 OS/libc/CPU，而非主机机器。
- 在源文件之前复制依赖清单，以便依赖安装层缓存良好。
- 在技术栈支持时使用感知 lockfile 的安装。
- 设置确定性的 `WORKDIR`，通常为 `/app`。
- 将运行时服务器绑定到 `0.0.0.0`，绝不绑定到 `127.0.0.1`。
- 只暴露检测到的容器端口。Compose 负责主机端口发布。
- 不要将 `.env`、本地数据库、缓存、`node_modules`、virtualenv、构建输出或 VCS 元数据复制到镜像中，除非项目明确要求。
- 当通过后端镜像提供已构建的前端时，只将构建好的静态输出复制到后端的 static 资源目录，并保持后端启动命令作为单一运行时进程。

## 源根目录、构建上下文、Workdir 与 COPY 闭包

在更改 Dockerfile 之前，验证路径三角：

- Compose `build.context`
- Compose `build.dockerfile`
- Dockerfile `WORKDIR` 和每个 `COPY` 源

Docker 构建上下文控制哪些文件可以被复制。如果 `COPY frontend/package.json` 命令在 `build.context: ./service` 下运行，即使文件在仓库中存在也会失败。修复上下文或复制路径；不要用宽泛的 `COPY . .` 掩盖它，除非那是预期的上下文。

对于生成的 workspace 感知 Dockerfile，先复制根 lockfile 和 workspace 清单，再复制服务清单，然后是源。对于应用本地生成的 Dockerfile，保持路径相对于该应用根。

使用此闭包矩阵：

- 应用本地服务：`build.context` 是服务根，Dockerfile 路径相对于该根，`WORKDIR` 通常为 `/app`，`COPY` 源相对于服务根。
- Workspace 服务：`build.context` 是 workspace 根，Dockerfile 路径相对于 workspace 根，`WORKDIR` 从 `/workspace` 或 `/app` 开始，先复制根 workspace 清单，然后在 build/start 之前切换到应用子目录。
- 拆分前端/后端：每个生成的服务使用其自己的清单所需的上下文。网关/前端服务不得复制后端文件，生成的代理配置除外。后端服务不得复制前端源，除非拓扑指明后端服务前端。
- 后端服务前端：使用包含前端和后端根的公共祖先上下文。在 builder 阶段构建前端资产，然后在后端打包或最终运行时之前将构建输出复制到后端 static/资源位置。
- 现有 Dockerfile 包装：保持用户 Dockerfile 似乎期望的构建上下文。不要在包装生成期间重写用户 Dockerfile 假设。

生成的 Dockerfile 可以使用 `/app`、`/workspace` 或 `/src`，但选择必须一致：

- `/app` 用于应用本地单服务和大多数运行时镜像。
- `/workspace` 当根清单和子应用目录需要在依赖安装期间共存时。
- `/src` 仅当生成的文件一致地从 `/src` 复制、构建和发布时；不要将 `/src` 复制路径与 `/app` 运行时路径混用。

每个依赖安装命令必须在包含其消费的依赖清单的目录中运行。每个构建命令必须在拥有应用构建脚本/项目文件的目录中运行。

## Ignore 文件

- 优先在生成的 Dockerfile 旁使用 Dockerfile 专用的 ignore 文件，例如 `Dockerfile.dockerignore`。
- 包含大型和敏感的本地状态：
  - `.git`
  - `.loom`
  - `.env`
  - `.env.*`
  - `node_modules`
  - `.next`
  - `.turbo`
  - `.vercel`
  - `out`
  - `.venv`
  - `venv`
  - `__pycache__`
  - `dist`
  - `build`
  - `coverage`
  - `.DS_Store`
- 保持 ignore 规则保守。默认不要忽略 lockfile、依赖清单、源目录、迁移、公共资产或框架配置文件。

## 运行时规则

- 优先使用扫描器检测到的运行时命令，然后是技术栈参考默认值。
- 生成的 Dockerfile 应对编码 agent 可理解，且在 build/start 失败后易于修补。
- 仅在不破坏常见框架行为或不需要项目特定的文件所有权更改时，才添加非 root 运行时用户。
- 除非技术栈有可靠的 HTTP 端点，否则避免 Dockerfile `HEALTHCHECK`。Compose 验证可以外部探测健康。
- 不要将密钥烘焙到 `ARG`、`ENV` 或复制的文件中。

## 修复线索

- 安装失败通常指向缺失 lockfile 处理、缺失系统包、错误的包管理器或构建上下文 ignore。
- 缺失原生模块（如 `@next/swc-linux-*` 或 `lightningcss.linux-*.node`）通常意味着 lockfile 或安装步骤只包含主机平台的可选依赖。通过安装匹配镜像的 Linux glibc/musl 包来修复，或切换镜像系列。
- 启动失败通常指向错误的命令、缺失构建 artifact、错误的绑定主机或错误的端口。
- 对于编译型技术栈，区分构建阶段失败和运行时阶段缺失二进制/文件。
- 如果构建找不到本地存在的项目文件，在更改包管理器命令之前检查 Compose 构建上下文和 `.dockerignore`。
- 如果生成的 Dockerfile 反复因缺失文件而失败，首先修复源根目录/上下文/workdir/COPY 闭包。不要不断添加临时 `COPY` 语句，直到 Dockerfile 变成整个仓库的镜像。

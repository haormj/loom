# Node 部署参考

当实现或修复 Node 家族项目的 loom deploy 支持时，使用本参考文档。

## 扫描器信号

- `package.json` 标识 Node 项目。
- Lockfile 选择包管理器：
  - `pnpm-lock.yaml` -> pnpm
  - `yarn.lock` -> yarn
  - `bun.lock` 或 `bun.lockb` -> bun
  - `package-lock.json` 或无识别的 lockfile -> npm
- `scripts.build` 成为构建命令。
- `scripts.start` 优先作为运行时命令。
- `next.config.*` 中的 Next.js `output: "standalone"` 应优先使用 `node .next/standalone/server.js` 作为运行时命令。
- 带有 `scripts.preview` 的 Vite 应以 host 绑定运行 preview，例如 `npm run preview -- --host 0.0.0.0`。

## 框架提示

- `next` 依赖 -> Next.js，默认端口 3000。
- `vite` 依赖或脚本 -> Vite，默认 preview 端口 4173。
- `astro` 依赖 -> Astro，默认端口 4321。
- `express`、`fastify`、`koa` 或 `hono` 依赖 -> Node server，默认端口 3000。
- 无 start/preview 脚本 -> Node CLI/library 项目。生成一个构建并说明未检测到 start 脚本的容器，而非假装提供 HTTP。

## 模板规则

- 从 `package.json` `engines.node`、`package.json` `volta.node`、`.nvmrc`、`.node-version` 或 `.tool-versions` 检测到时，使用项目声明的 Node 主版本。
- 如果不存在项目 Node 版本信号，默认为 `node:22-slim`。
- 为 Node 和 Next.js 项目使用 Linux/glibc slim 镜像，如 `node:<major>-slim`。除非有项目特定原因，否则 Next/Tailwind/CSS pipeline 避免使用 Alpine，因为原生可选依赖在 glibc 和 musl 之间通常不同。
- 对于 Bun 项目，优先使用 `oven/bun:1` 而非在 Node 镜像中安装 Bun。
- 仅当存在 package lockfile 时使用 `npm ci`；否则使用 `npm install`。
- 对于 pnpm/yarn，启用 Corepack。修复工作需要锁定包管理器版本时，尊重 `packageManager` 版本元数据。
- 对于 Next.js standalone 输出，在 production build 之后运行 `.next/standalone/server.js`，而非在配置声明 standalone 输出时调用 package `start` 脚本。
- 在生成的 Dockerfile 旁使用 Dockerfile 专用 ignore 文件，如 `Dockerfile.dockerignore`。
- 默认不要将 `.env` 文件复制到构建上下文。
- 生成的 ignore 文件应排除 `.next`、`.turbo`、`.vercel`、`out`、`dist`、`build` 和 `node_modules`，以使主机平台构建 artifact 不泄漏到 Linux 镜像中。
- 对于 Vite/SPA preview，运行绑定到 `0.0.0.0` 的 production preview 或 static server；除非项目仅有 dev 脚本，否则不要将开发服务器作为已部署运行时使用。
- 对于前端/后端部署，决定前端由 static 服务、框架服务器提供，还是复制到后端镜像。当浏览器将访问公共前端端口时，不要让前端 API 调用指向 `localhost:<backend-port>`。

## 构建上下文与 Workspace

- 对于应用本地 Node 项目，上下文可以是应用根，所有 `COPY` 路径应相对于应用根。
- 对于 Node workspace，上下文通常应为 workspace 根，以便 lockfile 和 workspace 清单可用。Dockerfile 然后应在 build/start 之前将 `WORKDIR` 设为应用 package。
- 使用 pnpm 时，在安装之前复制 `pnpm-workspace.yaml`、根 `package.json` 和 `pnpm-lock.yaml`。使用 npm/yarn/bun workspace 时，复制根 lockfile 和解析 workspace 图所需的 package 清单。

## 平台感知

- 容器在 Linux 上运行，而非开发者主机 OS。将 OS、CPU 和 libc 视为部署输入。
- 在 Apple Silicon 上，Docker 通常构建 Linux arm64 镜像。原生可选包可能需要 Debian/Ubuntu/slim 镜像的 `linux-arm64-gnu` 或 Alpine 的 `linux-arm64-musl`。
- 常见的 Next/Tailwind 原生可选包包括 `@next/swc-*`、`@tailwindcss/oxide-*` 和 `lightningcss-*`。
- 如果在 macOS 上生成的 lockfile 只包含 `darwin-*` 可选包，容器构建可能因缺失 Linux 原生模块而失败。优先修复项目 lockfile 或生成的 Dockerfile 安装步骤，使 Linux 可选依赖在容器内安装。
- 如果日志提到缺失 `lightningcss.linux-*.node`，优先使用 glibc 镜像（如 `node:<major>-slim`）并确保存在 `lightningcss-linux-<arch>-gnu`。如果使用 Alpine，确保存在 `*-musl` 变体。
- 如果日志提到缺失 `@tailwindcss/oxide-linux-*` 或 `tailwindcss-oxide.linux-*.node`，确保 slim/glibc 镜像有匹配的 `@tailwindcss/oxide-linux-<arch>-gnu` 包。如果使用 Alpine，确保有 `*-musl` 变体。
- 如果日志提到缺失 `@next/swc-linux-*`，确保镜像中安装了对应的 Next SWC 可选包，避免依赖 Next 从 npm 运行时下载。

## 现有 Compose 复用

- 优先使用根级 Compose 文件而非生成模板。
- 在 `loom.deployPrepare` 期间不要覆盖现有 Compose 文件。
- 从第一个简单的已发布端口映射（如 `8080:80`）推断预览 URL；使用主机侧作为本地预览端口。

## 依赖服务

- 从 `pg`、`postgres`、`postgresql`、`prisma` 或 `drizzle-orm` 信号检测 Postgres。
- 从 `redis`、`ioredis`、`bullmq` 或相关队列信号检测 Redis。
- 从 `mysql`、`mysql2` 或 `mariadb` 信号检测 MySQL。
- 从 `mongodb` 或 `mongoose` 检测 MongoDB。
- 从 `rabbitmq`、`amqplib` 或 `amqp` 检测 RabbitMQ。
- 从 `elasticsearch`、`@elastic/elasticsearch` 或 `opensearch` 检测 Elasticsearch/OpenSearch。
- 从 `minio` 或 S3 endpoint 信号检测 MinIO/S3-compatible 存储。
- 生成的 Compose 应仅为生成的部署添加依赖服务，而非覆盖现有 Compose 文件。
- 依赖服务应使用 Compose 内部网络和 `expose`，而非主机 `ports`，以避免本地端口冲突。
- 如果只检测到一个 SQL 服务，分配 `DATABASE_URL`。
- 如果同时检测到 Postgres 和 MySQL，避免歧义的 `DATABASE_URL`；改为分配 `POSTGRES_URL` 和 `MYSQL_URL`。

## 扫描器信号到部署事实

在生成文件之前，将 Node 扫描器证据转换为部署事实：

- `package.json` 路径成为服务根和 manifest ref。
- Lockfile 和 `packageManager` 元数据成为包管理器事实和安装命令选择。
- `scripts.build`、`scripts.start`、`scripts.preview`、框架配置和文档成为 build/start/runtime 命令候选。
- 框架信号成为运行时家族详情：Next.js、Vite/static preview、Astro、Remix、Express/Fastify/Koa/Hono、Bun 或 static export。
- 输出目录信号（如 `dist`、`build`、`out`、`.next` 和 `.next/standalone`）成为生成的资产预期。
- 纯前端信号加上另一个服务中的后端 API 信号应产生 `frontend_gateway_backend_api`，而非单个应用容器。
- 无前端界面的服务端 Node/API 信号应产生 `single_service_app` 或 `api_only_single_service`。
- 依赖 package/env 信号成为依赖服务事实和生成的环境 URL。

## 生成的资产预期

生成的 Node 资产应显示：

- Compose 服务 id 与源模型服务 id 匹配。
- 构建上下文包含安装所需的 package 清单、lockfile 和 workspace 清单。
- Dockerfile 安装步骤与包管理器和 lockfile 匹配。
- 仅当项目有构建步骤或框架输出要求时才有构建命令。
- 运行时命令绑定到 `0.0.0.0` 和选定的容器端口。
- 当前端资产与后端 API 分开提供时的 static 或 gateway 服务。
- 当存在前端网关时，面向浏览器代码的 API base env 指向公共代理路径。
- 生成的镜像中的 Next/Tailwind/CSS pipeline 的 Linux 可选依赖处理，而非主机 `node_modules`。

## 修复边界

在以下情况下修复生成的 Node 部署资产：

- 构建上下文遗漏了 workspace 根 lockfile 或 package 清单。
- 安装命令与 lockfile/包管理器事实不匹配。
- 当 production preview/server 可用时运行时命令使用了 dev server。
- 服务器绑定到 `127.0.0.1` 或错误端口。
- 前端代码指向 `localhost:<api-port>` 而非 gateway 路由。
- 原生可选包在 Linux 容器内失败。

除非 MCP 操作路由到执行修复或用户批准源代码更改，否则不要编辑应用源代码或 package 脚本。

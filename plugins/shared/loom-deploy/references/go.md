# Go 部署参考

当实现或修复 Go 项目的 loom deploy 支持时，使用本参考文档。

## 扫描器信号

- `go.mod` 标识 Go 项目。
- `go.sum` 表示依赖校验和已存在。
- 框架提示：
  - `github.com/gin-gonic/gin` -> Gin。
  - `github.com/labstack/echo` -> Echo。
  - `github.com/gofiber/fiber` -> Fiber。
- 端口检测从项目元数据和 env 示例中读取简单的 `PORT=9090` 或 `port: 9090` 信号；否则默认为 8080。

## 模板规则

- 使用多阶段 Dockerfile。
- 使用 `golang:1.23-alpine` 构建。
- 从 `alpine:3.20` 运行。
- 构建命令：`CGO_ENABLED=0 GOOS=linux go build -o /out/server .`。
- 运行时命令：`/app/server`。

## 修复说明

- 常见失败包括模块下载错误、需要 CGO/系统库的包、入口点在 `./cmd/<name>` 下的多命令仓库，或未绑定到 `0.0.0.0` 的应用。
- 除非用户批准应用源或模块更改，否则将修复保持在生成的部署文件中。

## 扫描器信号到部署事实

在生成文件之前，将 Go 扫描器证据转换为部署事实：

- `go.mod` 路径成为模块根、服务根和 manifest ref。
- `go.sum` 成为源复制前使用的 lock/checksum ref。
- 根目录或 `cmd/*` 下的 `main` 包成为入口点候选。
- 框架/导入信号（如 Gin、Echo、Fiber、Chi、net/http 或 gRPC）决定服务是否为 HTTP/API。
- Port/env 示例、`os.Getenv("PORT")`、router listen 调用和文档成为运行时端口事实。
- SQL/Redis/Mongo/RabbitMQ/cloud SDK 导入和 env 名成为依赖服务事实。
- 非 HTTP 命令包成为命令式部署事实，不带发明的预览路由。

## 生成的资产预期

生成的 Go 资产应显示：

- 在源复制前进行模块下载的多阶段构建。
- 构建命令针对选定的 `main` 包，例如 `.` 或 `./cmd/server`。
- 静态二进制输出复制到 slim runtime 镜像中。
- 仅当依赖事实不需要 CGO 时使用 `CGO_ENABLED=0`。如果需要 CGO，使用具有匹配系统库的 runtime/build 镜像。
- 运行时命令执行生成的二进制并暴露选定的容器端口。
- Compose env 仅在应用读取它或框架默认需要时包含 `PORT`。
- 依赖 URL 使用 Compose 服务 DNS 名。

## 修复边界

在以下情况下修复生成的 Go 部署资产：

- 构建目标指向错误的 `main` 包。
- `CGO_ENABLED=0` 与所需的 CGO/原生依赖冲突。
- Dockerfile 上下文遗漏了 `go.mod`、`go.sum` 或内部包。
- Runtime 镜像缺少所需的证书、时区数据或原生库。
- 应用通过生成的 command/env 在 localhost 或错误端口上监听。

在部署资产修复期间不要更改 Go 源代码、模块路径或生成代码，除非 MCP 操作路由到执行修复。

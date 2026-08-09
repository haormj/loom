# .NET 部署参考

当实现或修复 .NET 项目的 loom deploy 支持时，使用本参考文档。

## 扫描器信号

- 根级 `*.csproj` 或 `*.sln` 标识 .NET 项目。
- `Microsoft.NET.Sdk.Web`、`Microsoft.AspNetCore` 或 ASP.NET app builder 代码信号标识 ASP.NET Core。
- `TargetFramework` 或 `TargetFrameworks` 选择 .NET 主版本，例如 `net8.0` -> `8`。
- `global.json` SDK 版本可作为运行时主版本信号的回退。
- `ASPNETCORE_URLS`、launch settings 或通用 `PORT` 信号可标识运行时端口。ASP.NET Core 默认本地容器端口为 8080。

## 模板规则

- 使用多阶段 Dockerfile。
- 使用 `mcr.microsoft.com/dotnet/sdk:<major>` 构建。
- 使用 `mcr.microsoft.com/dotnet/aspnet:<major>` 运行 ASP.NET Core 项目。
- 使用 `mcr.microsoft.com/dotnet/runtime:<major>` 运行非 web .NET 项目。
- 使用 `dotnet restore`，然后 `dotnet publish -c Release -o /app/publish --no-restore`。
- 为生成的 Compose/runtime 环境设置 `ASPNETCORE_URLS=http://0.0.0.0:<port>` 和 `PORT=<port>`。
- 使用 `dotnet /app/<ProjectName>.dll` 运行已发布的 assembly。

## 依赖服务

- 从 `Npgsql`、`postgres` 或 postgres 连接字符串检测 Postgres。
- 从 `MySqlConnector`、`mysql` 或 MariaDB 连接字符串检测 MySQL/MariaDB。
- 从 `StackExchange.Redis` 或 Redis 连接字符串检测 Redis。
- 从 `MongoDB.Driver` 检测 MongoDB。
- 从 `RabbitMQ.Client` 检测 RabbitMQ。
- 从 Elastic 客户端包检测 Elasticsearch/OpenSearch。

## 修复说明

- 如果 publish 成功但 runtime 找不到 DLL，检查项目文件名和已发布的输出；启动命令应与 assembly 名匹配。
- 如果 healthcheck 失败，验证 `ASPNETCORE_URLS`、app `urls` 配置，以及 HTTPS 重定向是否强制了仅 HTTPS 端点。
- 如果 restore 因私有 feed 失败，请求 NuGet 凭证或项目特定的 `NuGet.Config`，而非将密钥烘焙到生成的文件中。
- 如果项目只有 `.sln` 和多个 web 项目，编码 agent 应在编辑生成的部署文件之前检查解决方案并选择预期的启动项目。

## 扫描器信号到部署事实

在生成文件之前，将 .NET 扫描器证据转换为部署事实：

- `*.csproj` 路径成为服务根和 manifest ref。当涉及多个项目时，`.sln` 成为 workspace 上下文。
- `Microsoft.NET.Sdk.Web`、ASP.NET 包、minimal API builder 代码、controllers 或 `UseRouting` 决定 HTTP/API 服务事实。
- `TargetFramework`、`TargetFrameworks` 和 `global.json` 选择 SDK/runtime 镜像主版本。
- `launchSettings.json`、`ASPNETCORE_URLS`、`urls`、Kestrel 配置和文档成为运行时端口事实。
- EF Core 包/迁移、连接字符串、Redis/Mongo/RabbitMQ 包和 env 示例成为依赖服务事实。
- 非 web worker/service 项目成为命令式部署事实，不带虚假预览路由。

## 生成的资产预期

生成的 .NET 资产应显示：

- 按项目类型选择的 SDK 构建阶段和 ASP.NET/runtime 最终阶段。
- 针对选定项目文件的 `dotnet restore` 和 `dotnet publish`，而非随机解决方案成员。
- 运行时命令指向选定项目的已发布 assembly 名。
- web 服务的 `ASPNETCORE_URLS=http://0.0.0.0:<port>` 和 `PORT=<port>`。
- Compose 依赖映射到使用服务 DNS 名的连接字符串。
- HTTPS 重定向不会使本地 HTTP 预览不可达。

## 修复边界

在以下情况下修复生成的 .NET 部署资产：

- restore/publish 使用了错误的项目路径。
- 运行时命令引用了错误的 DLL。
- SDK/runtime 镜像主版本与 target framework 不匹配。
- `ASPNETCORE_URLS` 或端口接线使 healthcheck 不可达。
- 生成的连接字符串指向 localhost 或遗漏了生成的依赖凭证。

在部署资产修复期间不要编辑 C# 源代码、项目文件、迁移或 NuGet feed，除非 MCP 操作路由到执行修复或提供了凭证。

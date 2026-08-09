# Static 部署参考

当实现或修复 static 站点部署支持时，使用本参考文档。

## 扫描器信号

- Static 输出目录：`dist`、`build`、`public`、`out`、`_site`。
- 纯 static 入口文件：`index.html`、`404.html`，无 server entrypoint 的 static 资产。
- 带有 Vite/Astro/Next export 的 Node 项目，当 `package.json` 有 build/preview 脚本时，可能仍先使用 Node 参考。

## 模板规则

- 对于已构建的 static 资产，使用 Nginx runtime 镜像并将检测到的输出目录复制到默认 web 根。
- 如果检测到构建步骤，先使用技术栈特定的构建阶段，然后将输出目录复制到 Nginx。
- 默认容器端口为 `80`。
- 仅当框架或项目信号客户端路由时才添加 SPA fallback。不要为纯 static 文档强制 fallback。

## 修复说明

- 常见失败包括缺失构建输出、错误的输出目录、生成的 Nginx 配置未复制，或项目在 static 服务前需要构建命令。
- 如果项目是没有生成资产的 library/文档源，报告缺失构建 artifact 而非盲目服务源树。

## 扫描器信号到部署事实

在生成文件之前，将 static 扫描器证据转换为部署事实：

- 根 `index.html` 或检测到的输出目录成为 static 源/输出事实。
- 框架/export 信号决定 static 服务前是否需要构建步骤。
- 客户端路由信号决定是否允许 SPA fallback。
- 同一仓库中的 API/后端信号应创建前端网关/后端 API 拓扑，而非纯 static 拓扑。
- 没有构建输出的纯文档/library 源成为缺失构建 artifact 诊断，而非可部署的 static 事实。

## 生成的资产预期

生成的 static 资产应显示：

- Nginx 或等效的 static runtime 服务于检测到的输出目录。
- 仅当扫描器事实证明有构建命令/输出目录时才有构建阶段。
- 仅当信号客户端路由时才有 SPA fallback。
- 仅当拓扑包含内部后端/API 服务时才有 API 代理配置。
- 容器端口 `80`，主机发布由 `DeploymentSpec.runtime.ports` 控制。

## 修复边界

在以下情况下修复生成的 static 部署资产：

- 复制的输出目录与扫描器事实不匹配。
- 构建阶段输出路径错误。
- Nginx 配置缺失、放错位置或 API 代理路由在 SPA fallback 之后。
- 纯 static 拓扑错误地包含 API 验证路径。

不要从源文件创建虚假的 static 输出以使容器启动。如果不存在构建输出或构建命令，报告缺失的可部署 artifact。

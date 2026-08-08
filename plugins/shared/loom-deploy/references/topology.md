# 部署拓扑参考

当 Loom 选择 `deploy.topology` 时使用本参考文档。它解释公共入口、代理路由、验证路径和网关行为。

## 权威

`DeploymentTopology` 从 Loom 部署事实生成。Compose 和 Nginx 必须实现它；agent 不应在修复期间用发明的拓扑替换它。

字段：

- `publicEntryServiceId`：拥有预览 URL 和已发布主机端口的服务。
- `routes`：公共路由规则，如 static SPA 服务或 HTTP 代理路径。
- `validation.previewPaths`：针对公共入口 URL 探测的路径。
- `validation.apiProbes`：仅从已接受 API 契约中的读取安全接口派生的安全 `GET`/`HEAD` 探测。路由契约仍包含每个声明的方法；写接口绝不作为部署探测执行。

## 前端网关 + 后端 API

当拓扑类别为 `frontend_gateway_backend_api` 时：

- 公共入口服务是前端/static 网关。
- 后端服务保持内部。
- 除非另一个公共端口明确存在，否则 Compose 仅发布前端网关主机端口。
- Nginx 或等效网关必须在 SPA fallback 之前定义 API 代理位置。
- 当 base 路径为 `/api` 时，`location /api/` 和精确的 `/api` 路由都需要。
- `proxy_pass` 必须指向后端 Compose 服务名和容器端口。
- SPA fallback 位于 API 代理位置之后。

## 后端服务前端 + API

当拓扑类别为 `backend_served_frontend_api` 时：

- 一个服务同时拥有预览和 API 验证。
- 不需要 HTTP 代理路由。
- 前端资产必须在打包或运行时启动前复制到后端的 static 输出中。
- API 验证路径直接针对后端公共端口探测。

## API-only 单服务

当拓扑类别为 `api_only_single_service` 时：

- API 路径是直接验证路径。
- 缺失代理路由不是错误。
- 如果应用暴露 HTTP，预览 URL 可以是 health/root 路径。

## 单服务应用

当拓扑类别为 `single_service_app` 时：

- 一个应用服务拥有公共预览 URL。
- 不需要 HTTP 代理路由。
- 验证直接针对应用公共端口探测预览/健康路径。

## Static 站点

当拓扑类别为 `static_site` 时：

- 公共入口服务 static 内容。
- API 路径必须为空。
- 仅当框架/源证据信号客户端路由时才添加 SPA fallback。

## 验证规则

生成的资产应在以下情况预检失败：

- 拓扑引用了源模型中不存在的服务 id。
- 前端网关拓扑缺少 HTTP 代理路由。
- Nginx 代理路由出现在 SPA fallback 之后。
- API 验证路径返回 HTML fallback。
- `DeploymentSpec.runtime.ports` 中公共入口服务没有公共端口。

当生成的网关文件与拓扑矛盾时，修复应修复生成的网关文件。如果拓扑本身与源事实矛盾，MCP 生成器是错误的，应被修复而非通过资产编辑隐藏。

## API 契约边界

部署消费由已接受的 Architecture artifact 引用的项目级当前 API 契约。它不从诸如 `/api` 的字符串推断公共 API 前缀，也不允许生成的前端环境变量重新定义接口路径。在生成资产写入之前，Loom 检查每个声明的接口路径是否适合公共暴露 base，并单独派生安全读取探测。未解决或冲突的绑定在诊断中阻止生成部署资产，并附带源文件和契约引用。

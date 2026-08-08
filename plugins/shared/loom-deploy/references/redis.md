# Redis 部署参考

## 何时使用

仅在已接受的 RuntimeDependency 投影包含 Redis 依赖时加载本参考文档。已接受的 TechnicalBaseline 能力对于 Redis 是否存在以及为何需要是权威的。

## 运行时事实

- Redis 服务身份来自规范依赖投影。
- 默认内部服务名为 `redis`，默认容器端口为 `6379`。
- 应用连接 URL 使用 Compose DNS，如 `redis://redis:6379`。
- 除非已接受的运行时端口方案明确标记一个为公共，否则 Redis 不得接收主机端口。
- 代码探测可以报告 Redis 候选，但不能覆盖已接受的 RuntimeDelivery 契约。

## 能力投影

- 仅缓存临时能力：不需要数据卷；应用 fallback 行为仍然是架构契约的一部分。
- 任何持久化能力：创建命名卷并挂载到生成的 Redis 数据路径。
- 必需 Redis 能力：生成 `redis-cli ping` healthcheck 并使依赖应用服务等待 `service_healthy`。
- Queue 和 Stream 能力必须保留已接受的持久化和重试边界；不要将它们变成无界的通用缓存。
- 当能力需要独立隔离或不兼容的 eviction 和 durability 策略时，需要独立的依赖 id。

## 生成的资产预期

- Compose 为每个已接受的依赖投影包含一个服务。
- Redis 使用固定的镜像 tag、内部 `expose` 且无意外的主机 `ports` 条目。
- 持久化 Redis 输出在顶级卷声明和服务挂载中都包含命名卷。
- Healthcheck、`depends_on`、服务名和连接 URL 引用同一 Redis 服务。
- 应用服务的生成 Dockerfile 不复制 `.env`、`.loom`、本地数据库或 Redis 凭证。

## Verification Focus

- `docker compose config` 解析 Redis 服务、卷、healthcheck 和依赖条件。
- 应用通过 Compose 服务名而非容器本地 `localhost` 连接。
- Cache、Session、Queue 和 Stream 能力投影与已接受的运行时事实匹配。
- 缺失的 Redis 候选不会从仓库关键词证据中创建服务。

## 修复边界

当源模型和已接受的运行时事实正确时，修复生成的 Compose 和生成的部署资产。不要通过编辑应用源代码或添加已接受契约中不存在的提供者来修复 Redis 拓扑。

如果已接受的能力模型错误或不完整，停止资产修复并请求上游 TechnicalBaseline 或 RuntimeDelivery 更正。不要让 Agent 从 Docker 错误中重建能力语义。

## 超出范围

Sentinel、Cluster、复制、生产备份、监控、TLS 和 registry 操作不由本地 Loom 部署路径生成。

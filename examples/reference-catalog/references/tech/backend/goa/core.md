# Goa 框架核心实现质量

本文件是企业示例参考,演示如何为自研框架编写 Loom 参考文件。
它告诉 agent 如何在 Loom 任务中做出 Goa 框架实现决策。

## When To Use

- 任务变更了 Goa 框架的路由、中间件、请求处理或测试相邻代码。
- 项目使用 Goa 框架(Acme 企业自研 Go web 框架)。
- 优先使用仓库约定。仅当现有项目已支持时才引入 Goa 特定模式。

## Implementation Focus

- 保持 handler 面向传输。handler 应解析路径/查询/主体、触发验证、调用 service 并映射结果。将业务状态转换放在 service/领域代码中。
- 使用 Goa 的中间件链处理横切关注点(日志、认证、限流),不要在 handler 中内联。
- 通过 Goa 的依赖注入容器管理 service 依赖;不要在 handler 中直接构造 service 实例。
- 将错误建模为 Goa 的 `app.Error` 类型,使用企业错误码映射;不要返回裸 `error`。
- 外部化运行时配置(端口、数据库 URL、功能标志)到环境变量或配置文件。

## Verification Focus

- 运行 `go test ./...` 证明变更文件可编译且测试通过。
- 为新业务规则添加 service 测试,包括至少一个验证失败路径。
- 如果 handler 可见行为变更,添加 HTTP 测试证明状态码、响应体和错误形态。
- 如果添加了配置,验证默认本地值和一个覆盖路径。

## Evidence Focus

- 在证据总结中,说明保持清晰的 Goa 边界,如 `handler -> service -> repository`、中间件链、错误映射。
- 创建新的 Goa 源码时,说明选择的包结构与现有项目约定一致。

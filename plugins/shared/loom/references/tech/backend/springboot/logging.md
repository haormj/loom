# Spring Boot 日志

仅当任务拥有 Spring Boot 日志基础设施时才使用此参考。仅发出任务拥有事件的任务使用已配置的 logger，不替换 provider 或全局配置。

## Provider 决策

1. 当仓库已有的 SLF4J provider 和配置满足已接受契约时保留它们。
2. 对于全新 Spring Boot 应用，使用 Boot starter 的 SLF4J 和 Logback 默认。
3. 仅当已接受的基线或仓库已选择 Log4j2 或其他 provider 时才使用它。慎重排除被替代的 provider；切勿留下多个 SLF4J 绑定。

应用代码依赖 SLF4J API。不要从业务 service 调用 Logback 类。仅当 Lombok 已是已建立的仓库依赖时才可接受 Lombok `@Slf4j`。

## 配置归属

将环境中立的级别和事件形态保留在 `application.yml`/`application.properties` 或仓库已有的 `logback-spring.xml` 中。环境覆盖拥有目的地和级别变更。不要在源码中硬编码生产路径、凭证或调试级别。

使用来自 `tech/code/observability.md` 的稳定结构化字段。仅当需要结构化 JSON 且没有已有 provider 提供时才添加 JSON 编码器依赖。当人类可读的本地输出是仓库约定时保留它。

## 关联与边界

在 filter 或已接受的 observation/tracing 集成中建立一次请求关联。仅将安全标识符放入 MDC 并可靠清除。对于 Reactor、执行器、定时工作和消息，使用支持的上下文传播；不要复制任意 ThreadLocal。

业务代码仅记录其拥有的关键转换、依赖结果、重试终止、async 结果和意外错误。Controller、service、repository 和异常 handler 不得都记录同一失败。

## 异步与文件输出

仅当已接受的可运维性需求要求时才使用异步 appender。限定队列，定义丢弃/阻塞行为，保留高严重性事件，并在优雅关闭期间刷新。

当需要应用拥有的文件日志时，配置具有显式目录、时间和/或大小触发器、压缩、历史和总大小上限的轮转策略。当文件归属未被接受时保持仅 console 输出。不要要求 Deploy 生成 Logback 设置或发明日志卷。

## 运行时失败策略

- 日志不得成为请求处理、持久化或消息消费的隐藏正确性依赖。
- 如果 appender、编码器或目的地不可用，遵循已接受的过载策略并保持业务结果显式。
- 在应用队列限制或丢弃规则时保留安全、恢复和事件诊断所需的错误事件。
- 保持 logger 配置失败在启动期间可见而不泄漏密钥或环境凭证。

## 结构化事件契约

- 使用稳定的事件名和有界字段，如 operation、outcome、error code、dependency、duration 和 correlation id。
- 不要使用原始请求 URL、任意异常消息、用户输入或无界标识符作为 metric 或日志维度。
- 在序列化之前脱敏授权 header、cookie、token、密码、连接字符串和敏感 payload。
- 保持事件 schema 与仓库的 API 错误和 tracing 契约对齐，使一个操作可以跨边界跟踪。

## 选择边界

任务必须说明它是否拥有 provider 选择、事件埋点、异步缓冲、文件输出或仅验证。日志参考不授权对不相关业务模块、部署拓扑或外部 collector 的变更。

在任务证据中记录该归属决策，保持未拥有的日志行为不变。

## Verification Focus

- 启动应用并断言恰好一个 SLF4J provider 激活。
- 捕获一个拥有的事件并验证级别、稳定字段、关联和脱敏。
- 证明一个意外失败在所选边界记录一次。
- 当拥有异步输出时，执行队列压力和关闭刷新行为。
- 当拥有文件输出时，验证轮转、压缩、保留、总大小限制和不可用目录。

## 不安全默认

- 在 Boot 默认 Logback 绑定旁添加第二个 provider。
- 记录请求 body、token、凭证或原始 provider 响应。
- 在每层放捕获/记录/重新抛出块。
- 没有总大小限制的无界异步 appender 或滚动文件。
- 将 Actuator 暴露视为应用日志行为的替代。

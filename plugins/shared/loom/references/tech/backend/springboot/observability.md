# Spring Boot 可观测性

本参考负责 Spring Boot 健康、metric 和 tracing 机制。跨栈事件行为在 `tech/code/observability.md` 中；SLF4J provider 配置、日志配置、异步 appender 和滚动文件在选择该参考时在 `tech/backend/springboot/logging.md` 中。

## Actuator 暴露

仅暴露运行时契约所需的端点。Health 和 info 常见；metric 和 Prometheus 需要实际的 collector/检查边界。Environment、config 属性、bean、mapping、heap dump、logger 和 thread dump 是敏感管理表面，不是公共默认。

通过独立端口/网络或显式安全策略保护管理端点。不要对未认证的健康响应使用 `show-details: always`。

## Liveness 与 Readiness

保持探针含义区分：

- **liveness**：进程可以继续；失败导致重启
- **readiness**：实例可以为其所需能力接收流量
- **能力/依赖状态**：可选或有范围的能力可能不可用而不杀死整个进程

不要在每个健康探针中放入缓慢的外部业务调用。依赖健康指标必须有界、适当缓存或轻量，并与依赖在启动时还是按能力必需保持一致。

不要为可恢复的下游中断将 liveness 标记为 down；重启循环可能放大中断。

## Metric

Metric 需要稳定的名称、单位和有界 tag。好的 tag 包括结果类、操作、依赖或稳定错误类别。切勿用用户 id、订单 id、包含标识符的 URL、异常消息、原始查询或任意 tenant 打 tag，除非基数显式有界。

对延迟使用 timer，对发生次数使用 counter，对当前有界状态使用 gauge，仅在服务于已接受的 NFR 或运维问题时使用分布配置。

## Tracing

使用 Micrometer Observation/Tracing 或仓库已建立的机制。在支持的 HTTP、消息、Reactor 和异步边界保留 trace 上下文。不要在每个方法周围手动创建 span。

采样概率是环境决策，而非硬编码 `1.0` 生产默认。避免敏感 span tag 和高基数业务标识符。

## Domain 与失败信号

当架构 NFR/风险需要时，为关键状态转换和终态失败发出稳定的可观测信号。遥测不是真值来源；业务审计记录在需要时属于持久 domain 存储。

## 归属与失败策略

任务必须确定它是否拥有端点暴露、探针行为、metric、tracing 或仅证据收集。可选的遥测 exporter 失败不得导致业务请求失败；必需的 readiness 依赖必须遵循已接受的启动和恢复策略。

保持 Actuator 配置环境特定，将管理暴露与公共 API 表面分开审查。

## Verification Focus

有用的可观测性证据包括：

- 精确的 Actuator 端点和访问策略
- 必需和可选依赖中断的 liveness/readiness 行为
- 安全的健康详情暴露
- 变更边界上的 Spring 请求/trace 上下文传播
- 有界 metric tag 和预期的增量/时序
- 当拥有时的异步/响应式/客户端调用的 trace 传播
- 遥测导出可选时没有 collector 的启动

## 不安全默认

- 暴露所有 Actuator 端点。
- 公共端点上 `show-details: always`。
- 在每次 liveness 探针期间调用缓慢外部服务。
- 记录请求/响应 body、token 或凭证。
- 用无界 ID 或异常消息为 metric 打 tag。
- 硬编码全量 trace 采样。
- 将重启视为每个依赖中断的恢复。

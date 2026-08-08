# Java Reactor 与响应式数据基础

此参考拥有 Reactor 组合、非阻塞边界、背压、取消、响应式上下文和 R2DBC 事务语义。Spring WebFlux 控制器、WebClient 配置、安全过滤器和 web 测试属于 Spring Boot 参考。

## When To Use

当 Java 实现工作拥有 Reactor pipeline、响应式流、R2DBC 事务、取消/资源清理、背压、调度器边界或响应式上下文传播时使用此参考。Spring Boot API 任务仅在已接受的技术栈包含 WebFlux/响应式技术或任务显式拥有响应式实现时才选择它。

不要用于普通 Spring MVC、JPA、同步 HTTP 客户端或仅异步业务措辞。框架传输和客户端配置保留在选中的 Spring Boot 参考中。

## Implementation Focus

### Non-Blocking Boundary

保持响应式链端到端非阻塞。不要在事件循环线程上调用 `.block()`、同步 HTTP 客户端、JPA repository、文件系统 API 或阻塞 SDK。

当不可避免的阻塞适配器存在时，在一个显式边界上将其隔离在有界调度器上，并考虑并发、排队、取消和关闭。不要将 `subscribeOn` 作为通用修复散布。

### Operator Semantics

- `map`：同步转换
- `flatMap`：无顺序保证的异步组合
- `concatMap`：有序异步组合
- `flatMapSequential`：并发工作但有序结果
- `zip`：组合独立 publisher
- `switchIfEmpty`：显式缺失/未找到分支
- `then`：当先前值被有意丢弃时的完成

避免嵌套订阅。应用代码应将 publisher 返回给拥有框架。手动 `subscribe()` 隐藏生命周期、取消和失败。

### Error Semantics

使用 `onErrorMap` 转换错误，`onErrorResume` 仅用于已接受的回退。不要将失败转换为空的成功 publisher。在操作边界放置重试并分类安全失败；没有去重契约永远不要重试非幂等写入。

使用 `doOn...` 进行观察，而非业务变更。清理属于 `usingWhen`、`doFinally` 或具有定义的成功/错误/取消行为的资源特定操作符。

### Backpressure And Bounds

无界 `Flux` 结果需要分页、流式、速率限制或已证明的有界源。定义缓冲和溢出行为。避免将任意大的流收集到内存中。

为扇出工作有意选择并发和预取。更多并发可能使下游服务过载并破坏排序假设。

### Context And Threading

响应式执行可能在线程之间移动。不要在没有受支持的上下文传播的情况下依赖 ThreadLocal 请求、事务、安全或日志状态。通过框架支持的集成将不可变关联/安全数据放入 Reactor Context。

### R2DBC And Transactions

为响应式持久化使用响应式 repository/驱动。响应式事务必须通过受支持的操作符或事务代理包装订阅；命令式事务假设不会自动应用。

不要跨并发操作符传递可变实体。显式定义写入顺序和冲突行为。

### R2DBC Mapping Boundary

一致使用选中的 R2DBC 映射模型：表和标识符注解、列名、可空性、转换器、生成键行为和迁移 SQL 必须一致。不要将 JPA 注解或延迟关联假设混入 R2DBC 实体。当仓库约定支持时不可变 record 有用，但映射便利不得削弱领域不变式。

将响应式 repository 和数据库客户端保持在应用拥有的边界之后。repository 方法应暴露操作的基数和空结果语义；当调用者需要有界页面或摘要时不应返回无界流。

### Reactive Client Boundary

`WebClient` 组合仅在此处的 Reactor 边界：返回 publisher、在解码主体之前映射提供者状态、在操作边界应用超时并保留取消。客户端构造、base URL、凭据、连接池、载荷限制和提供者错误契约属于选中的框架/集成参考。对于 Spring Boot，使用专用的外部服务集成参考。

仅重试集成/弹性契约声明为重试安全的操作。响应式重试必须有尝试预算且不得重复非幂等写入。不要使用 `.block()` 将响应式客户端桥接到请求路径。

### Cancellation

取消是正常的终止信号。确保资源关闭并避免在调用者取消后继续不可见地产生副作用，除非操作是有意持久的且解耦的。

## Verification Focus

使用 `StepVerifier` 和虚拟时间验证：

- 成功、空和错误分支
- 排序和并发
- 超时/重试分类
- 背压/有界收集行为
- 取消和清理
- 响应式事务提交/回滚
- 与选中提供者的 R2DBC 映射和迁移往返
- 客户端状态/错误映射、超时、取消和重试尝试边界
- 当检测工具存在时无阻塞调用

## Evidence Focus

对 publisher 信号使用 `StepVerifier` 或等效的基于订阅者的断言。证据应识别成功、空、失败、取消、排序或重试路径并断言终止信号和副作用，而非仅发射的值。

对于非阻塞声明，在可用时包含特定边界的检查或检测器结果。对于 R2DBC、重试和资源清理，使用演练订阅、事务/资源生命周期和选中提供者或适配器的集成证据。

## Unsafe Defaults

- 响应式请求路径中的 `.block()`。
- 用于业务副作用的 `subscribe()`。
- 在 `map`/`flatMap` 中隐藏阻塞工作。
- 返回空/伪造成功的 `onErrorResume`。
- 无界 `collectList`、缓冲区或扇出并发。
- 假设 ThreadLocal 或命令式事务状态传播。
- 将 JPA 映射/生命周期假设混入 R2DBC 模型。
- 没有幂等或去重契约的情况下重试非幂等客户端写入。

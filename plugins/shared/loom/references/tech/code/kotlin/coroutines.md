# Kotlin 协程质量

## When To Use

- 任务变更了 suspend 函数、协程作用域、调度器、Flow、StateFlow、SharedFlow、channel、后台工作、取消、重试或异步集成。
- 当正确性依赖于结构化并发、生命周期所有权、背压或异步错误传播时使用此参考。
- 如果变更的 Kotlin 代码是同步的且不在异步框架路径中，不要因为此参考可用就引入协程。

## Implementation Focus

- 使用结构化并发。不要在生产代码中使用 `GlobalScope`；工作应由请求、服务、ViewModel、应用或显式管理的作用域拥有。
- 按工作类型选择调度器：CPU 在 `Default`，阻塞 I/O 在 `IO`，UI 状态更新在 main/lifecycle 作用域。不要在默认 suspend 函数中隐藏阻塞调用。
- 保留取消。重新抛出 `CancellationException`；不要捕获 `Exception` 并将取消转换为正常失败状态。
- 当所有子项必须成功时使用 `coroutineScope`，当独立子项可以失败而不取消兄弟项时使用 `supervisorScope`。使此失败策略可见。
- 仅当等待结果且并行有益时使用 `async`。对不返回值的生命周期拥有工作使用 `launch` 并具有显式错误处理。
- 对 Flow，区分冷数据流、`StateFlow` 当前状态和 `SharedFlow` 事件。不要对一次性事件使用 `StateFlow` 或对需要初始值的状态使用 `SharedFlow`。
- 有意使用 Flow 操作符：`debounce` 用于用户输入，`distinctUntilChanged` 用于抑制重复，`flatMapLatest` 用于可替换请求，`buffer` 或 `conflate` 仅在丢弃/解耦可接受时使用。
- 当输入可能增长时用 channel、缓冲区或信号量绑定生产者-消费者工作。不要从列表或流启动无界异步工作。
- 在关闭、组件销毁或 ViewModel 清除时拥有后台作用域取消。不要让 job 泄漏超过其生命周期。
- 将重试限制在安全/幂等操作并记录可重试异常类型。

## Decision Rules

- 将作用域放在可以取消工作的所有者处：请求工作用请求作用域，UI 工作用 ViewModel 或屏幕作用域，仅当工作必须超过单个请求或屏幕时才用应用拥有的作用域。
- 在阻塞边界保持调度器选择。调用阻塞驱动程序的 repository 应隔离该调用；调用者不应猜测通用 `suspend` 函数需要哪个调度器。
- 对具有当前值的可观察状态使用 `StateFlow`，对事件或广播使用 `SharedFlow`，当每个收集器拥有收集时使用冷 `Flow`。当事件可能丢失时记录重放和缓冲策略。
- 仅在说明哪些工作或值可能被取消、延迟或丢弃之后才选择 `flatMapLatest`、`buffer` 或 `conflate`。不要在具有通用名称的辅助函数中隐藏此策略。
- 仅重试幂等操作或受幂等键保护的操作。绑定尝试和延迟，并通过拥有的状态或结果类型呈现终止错误。

## Verification Focus

- 对协程代码使用 `runTest` 或仓库协程测试设置。
- 当 Flow 行为变更时使用 Turbine 或等效工具进行 Flow 发射顺序、完成、取消和错误断言。
- 在涉及处测试取消、超时、首个错误/部分成功策略、调度器敏感边界和生命周期清理。
- 确认没有引入 `GlobalScope`、生产路径中的 `runBlocking`、吞掉的取消或无界并行。
- 对于生命周期变更，证明父作用域被取消且子 job 不超过请求、屏幕、服务或关闭所有者的生命周期。

## Evidence Focus

- 在证据总结中，说明协程决策：作用域所有权、调度器、取消、supervisor 策略、Flow 类型、缓冲/背压、重试策略或生命周期清理。

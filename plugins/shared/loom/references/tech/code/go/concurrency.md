# Go 并发与 Goroutine 所有权

## When To Use

仅当任务显式拥有 goroutine、channel、worker 池、pipeline、队列、定时器、速率限制、共享状态、异步作业或关闭行为时才使用此参考。

## Implementation Focus

### Lifecycle First

每个 goroutine 都有命名所有者、启动点、停止/取消信号、错误/结果路径和等待/汇合完成。不要在构造函数中启动隐藏的后台工作而不返回 close/stop 所有者。

不要仅为使阻塞 API 看起来异步而启动 goroutine；在真实边界保留背压和取消。

对共享取消/错误传播的兄弟任务使用 `errgroup.WithContext` 或仓库等效方案。根据工作负载/资源用 `SetLimit`、信号量或 worker 池绑定并发。

### Context And Shutdown

在发送、接收、等待、重试、速率限制或定时器工作时 select `ctx.Done()`。阻塞的 channel 操作不得阻止关闭。

定义优雅关闭顺序：停止接收、信号取消、按策略排空或丢弃排队工作、停止生产者、由所有者关闭输出、带截止时间等待、释放资源。

在取消/进程重启之间保留部分/已提交的工作和幂等性。Context 取消不会自动回滚外部效果。

### Channel Ownership

发送/生产所有者在所有发送完成后关闭 channel；接收者不关闭共享输入。有多个生产者的 channel 需要一个协调器在生产者完成后关闭。

关闭 channel 以表示不再有值，而不是当 context 是已建立边界时作为通用取消广播。永远不要在关闭后发送/关闭 channel。

选择缓冲区容量作为显式有界交接/背压。大缓冲区可能隐藏慢消费者并增加内存/过期工作；零缓冲区强制汇合。

处理关闭接收（`v, ok`/range）、nil channel、放弃的消费者和早期 pipeline 错误，使 fan-in/fan-out goroutine 不会泄漏。

### Worker Pools And Pipelines

定义作业标识、队列边界、满/关闭时的入队行为、排序/公平性、重试/幂等性、每作业超时、panic/错误处理、结果所有权和排空/取消策略。

仅在每个 pipeline 阶段的所有 worker 停止后才关闭该阶段的输出；使用 WaitGroup/协调器。传播第一个或所有相关错误，而不是使用因无人接收而阻塞的错误 channel。

不要在 goroutine 之间共享循环变量或可变请求缓冲区而不进行每次迭代拷贝/所有权（包括版本相关的 range 语义）。

### Mutexes, Atomics, And Once

为多字段不变式使用 mutex 并保持临界区小，不调用阻塞/可重入外部代码。记录哪个 mutex 保护哪个状态。

`RWMutex` 仅在测量的读密集争用时有用；写者饥饿/开销可能更糟。永远不要复制已使用的包含 mutex/atomic 的结构体。

使用 `sync.Once`/`OnceValue` 进行一次性初始化，具有清晰的失败/panic 语义。初始化错误可能需要显式状态而非 Once。

对简单的已验证协议/计数器/标志使用 `sync/atomic`，并对内存模型有理解。原子字段不会使复合不变式变为原子。

### Timers, Tickers, And Retries

不再拥有时停止 ticker 和 timer。为支持的 Go 版本重用/重置定时器并具有已记录的停止/排空语义；当分配/资源重要时避免在长循环中反复使用 `time.After`。

重试需要有界的尝试/时间预算、context、退避/抖动、失败分类和幂等操作。不要重试永久验证/认证失败或跨层倍增重试。

速率限制器和信号量在每条路径上释放许可并暴露取消/过载行为。

### Panic And Recovery

任何 goroutine 中的 panic 都可能终止进程。仅在刻意的进程/作业隔离边界恢复，记录安全上下文/堆栈，转换为失败，保留清理；不要用 recover 忽略已损坏的不变式。

### Race And Leak Safety

Map 和普通变量在并发读/写时需要同步。Channel 传输在双方都保留引用后不保护对象。

避免永远阻塞在发送、接收、锁、无 context 的网络调用或 wait group 上的 goroutine。捕获 goroutine profile 仅用于诊断，而非作为常规产品输出。

## Verification Focus

- 测试成功、首个错误、取消、超时、满/关闭队列、放弃的消费者、重复停止和有待处理工作的关闭。
- 使用确定性门/channel/假时钟而非仅 sleep 的排序断言。
- 为变更的共享状态路径运行聚焦的 `go test -race` 和重复/压力用例。
- 断言并发/队列/速率边界以及停止后没有工作。
- 检查 goroutine/资源清理，在失败时使用截止时间和诊断。

## Evidence Focus

说明 goroutine/channel 所有者、绑定/背压、状态同步、取消/关闭策略和确定性/竞争证明。WaitGroup 或一次干净运行不建立泄漏/竞争安全。

## Unsafe Defaults

- 启动 goroutine 而没有停止/错误/等待所有者。
- 使用缓冲 channel 隐藏阻塞或无界工作。
- 接收者/多个发送者不安全地关闭 channel。
- 在阻塞发送/接收/定时器/重试期间忽略 Context。
- 仅 sleep 的并发测试。
- 在没有不变式或争用分析的情况下选择 Atomic/RWMutex。
- 重试层倍增非幂等效果。

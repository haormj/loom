# Spring Boot 异步处理

仅当任务拥有真实的异步边界时才使用 Spring 异步执行。`@Async` 改变执行和失败语义；它不是使阻塞请求看起来非阻塞的性能注解。

## 边界选择

选择匹配持久性和耦合的机制：

| 需求 | 合适边界 |
|---|---|
| 与一个应用实例绑定的短、非持久工作 | 受管 `TaskExecutor` 和 `@Async` |
| 必须在重启后存活的持久工作 | 队列、job 存储或持久化工作记录 |
| 定时对账 | 带显式重叠/幂等策略的 Spring 调度器 |
| 响应式 I/O 组合 | Reactor 管道，而非响应式代码周围的 `@Async` |

不要对已接受契约说不得丢失的业务工作使用内存执行器。

## 代理与调用

`@Async` 由 Spring 代理应用；同类自调用、private 方法和用 `new` 创建的对象不跨越异步代理。

```java
@Service
final class NotificationDispatcher {
    private final NotificationSender sender;

    NotificationDispatcher(NotificationSender sender) {
        this.sender = sender;
    }

    @Async("notificationExecutor")
    public CompletableFuture<DispatchResult> dispatch(NotificationCommand command) {
        return CompletableFuture.completedFuture(sender.send(command));
    }
}
```

当同步 service 触发异步工作时，将异步入口点保持在单独的 bean 中。使返回类型反映调用者是否可以观察完成：`CompletableFuture<T>` 用于可观察完成，`void` 仅用于显式 fire-and-forget 且带异常 handler 和独立失败信号的工作。

## 执行器归属

为拥有的工作负载定义命名执行器。配置：

- 基于工作负载类型的核心/最大并发
- 有界队列容量
- 线程命名
- 拒绝行为
- 用于支持上下文传播的任务装饰
- 优雅关闭和等待超时

无界队列隐藏过载直到内存压力。`CallerRunsPolicy` 改变延迟和执行上下文；仅当调用者上的背压可接受时使用它。丢弃工作需要显式可观测失败和重试/恢复路径。

不要手动复制安全、请求、locale 或日志 ThreadLocal。使用支持的任务装饰器/上下文传播，仅传播工作所需的数据。切勿在请求完成后依赖请求范围 bean。

## 事务与副作用

异步方法在另一个线程中运行，不继承调用者的事务。跨边界传递托管 JPA 实体面临 detached 状态和陈旧数据的风险。传递不可变标识符/命令并在异步操作内加载所需状态。

当异步工作依赖已提交数据时在提交之后触发。根据可靠性需求使用事务同步、带 after-commit 监听器的 domain 事件或持久 outbox。进程失败时 after-commit 内存回调仍会丢失。

为重试或重复调度定义幂等。当用户或操作员需要观察完成时记录状态/进度。

## 失败与取消

- 异常完成返回的 future；不要将失败转换为静默成功。
- 仅对 `void` 方法配置 `AsyncUncaughtExceptionHandler`。
- 区分可重试依赖失败和无效业务输入。
- 在底层工作支持的地方保留中断/取消信号。
- 在关闭期间停止接受新任务并限定进行中工作的等待。

## Verification Focus

有用的异步证据包括：

- 调用跨越 Spring 代理和执行器线程的证明
- 有界队列/拒绝行为
- 成功和异常完成
- 数据库支持工作的 after-commit 排序
- 重复效果保护
- 上下文/关联传播而无请求范围泄漏
- 排队和运行中工作的关闭行为

使用 future、latch、Awaitility、虚拟时间或完成记录。不要通过任意 sleep 断言异步行为。

## 不安全默认

- 对同类方法调用添加 `@Async`。
- 意外使用公共池或无界执行器。
- 将 JPA 实体传入另一个线程。
- 假设调用者事务跨越异步边界。
- 没有失败可见性的 fire-and-forget 工作。
- 对必需持久工作使用内存异步执行。

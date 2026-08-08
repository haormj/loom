# Swift 并发质量

本文件适用于 Swift async/await 和并发隔离。

## When To Use

- 任务变更了异步函数、actor、`@MainActor` 代码、任务组、`async let`、`Task`、取消、`AsyncSequence`、续体、Sendable 类型或并发相关的 UI/服务器行为。
- 当正确性依赖于结构化并发、线程安全、取消或跨越 actor/执行器边界时使用此参考。
- 如果触及的 Swift 代码是同步的且没有并发边界，不要引入异步。

## Implementation Focus

- 优先使用原生异步 API 和结构化并发。不要仅因为存在旧回调示例就将异步 API 包装在续体中。
- 对跨任务的可变共享状态使用 actor。将同步锁和 `@unchecked Sendable` 作为最后手段边界并具有清晰的不变式。
- 当 UI 面向的 view model 和 UI 更新方法变更 UI 状态时用 `@MainActor` 标记。当非 UI 工作可以独立运行时将其保持在 main actor 之外。
- 对小型固定独立操作集使用 `async let`，对动态扇出使用任务组。当调用者依赖顺序时显式保留结果排序。
- 每个长时间运行的 `Task` 需要所有权、取消和清理。避免分离任务，除非有意打破结构化并发。
- 在循环、流和长等待中检查取消。传播 `CancellationError` 或将其转换为仓库期望的用户/系统行为。
- 对于 `AsyncSequence`，用 `onTermination` 或等效生命周期处理定义终止和资源清理。
- 续体必须在每条成功、失败、取消和提前返回分支上恰好恢复一次。除非仓库有测量理由不这样做否则优先使用检查续体。
- 安全时使跨任务数据 `Sendable`，并修复编译器警告而非隐藏它们。

## Decision Rules

- 对小型固定子操作集使用结构化 `async let`，对动态扇出使用任务组。说明首个失败是否取消兄弟项、部分结果是否有效以及结果顺序如何保留。
- 对可变共享状态使用 actor 并保持 actor 方法小。将 UI 状态和 view-model 变更标记 `@MainActor`；不要无理由地将网络、解析或重计算移到 main actor。
- 为每个长生命 `Task` 赋予所有者和取消路径。分离任务需要显式生命周期和 Sendable 边界，因为它们逃逸正常结构化并发。
- 将续体桥接视为一次恢复证明：对成功、失败、取消和提前返回恰好恢复一次。在可用处优先使用原生异步 API。
- 使 `AsyncSequence` 终止和资源清理显式，并在清理后重新传播取消。完成的流、超时和取消是不同的结果。
- 将 Sendable 警告视为边界设计反馈。修复数据所有权或 actor 隔离而非全局抑制警告。

## Verification Focus

- 运行 `swift build`/目标构建和仓库的异步测试。
- 测试任务涉及的成功、失败、取消、超时或流终止、actor 隔离变更和 main-actor UI 更新路径。
- 在项目启用时用并发警告或警告即错误编译。
- 对于任务组或扇出，验证部分失败和排序行为。

## Evidence Focus

- 在证据总结中，说明并发决策：actor 隔离、MainActor 边界、async let/任务组、任务所有权、取消、AsyncSequence 清理、续体安全或 Sendable 证明。

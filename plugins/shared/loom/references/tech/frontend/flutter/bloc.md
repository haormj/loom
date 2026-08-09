# Flutter 与 Bloc/Cubit 的共享状态

仅当 TechnicalBaseline 选择 Bloc 且任务拥有共享客户端状态时应用 Bloc/Cubit。不要为本地 widget 状态或在 Riverpod-only 架构中引入它。

## Cubit 还是 Bloc

对事件源和并发直接的命令到状态工作流使用 Cubit。当显式事件语义、事件 transformer/并发、多生产者或可审计转换建模增加真实价值时使用 Bloc。

事件精确描述用户/系统事实和命令；状态是不可变 UI 工作流快照。避免 `UpdateEverything`、每个字段一个状态和省略目标且依赖可变选定记录的事件载荷。

```dart
sealed class ApprovalEvent {
  const ApprovalEvent();
}

final class ApprovalRequested extends ApprovalEvent {
  const ApprovalRequested(this.orderId, this.expectedVersion);
  final String orderId;
  final int expectedVersion;
}
```

使用仓库的相等性/不可变生成约定（Equatable、Freezed、sealed record/class）。替换集合/model 而非原地修改。

## 状态模型

在相关处表示 initial/loading/empty/ready/refreshing/submitting/success 和类型化的验证/冲突/禁止/离线/不可用失败。仅在已接受时在刷新/变更期间保留可用的先前数据。

当需要稳定视图模型时将领域/持久化实体排除在 widget 状态之外。不要在状态/bloc 中存储 `BuildContext`、widget、controller、snack bar 或导航对象。

状态转换应在正确时间清除过期失败/pending 字段并在写入后协调服务端返回的 ID/版本/状态。

## 事件并发与异步工作

从产品行为选择事件 transformer：restartable/latest 用于可替换搜索，droppable 用于重复提交预防，sequential 用于有序写入，concurrent 用于独立有界工作。

不要在未 await/跟踪的情况下启动必需副作用。将 repository 错误处理为类型化状态并保持 bloc 事件流存活。在支持处传播取消。

将外部/网络/存储操作保留在注入的 repository/用例中。一个 bloc/cubit 操作拥有排序和部分失败行为；widget 不应协调第二份副本。

## 配置与生命周期

在匹配状态生命周期的最窄稳定 app/功能/路由/标签边界提供 bloc。仅对处置由别处拥有的现有实例使用 `BlocProvider.value`；用 `.value` 创建实例可能泄漏它。

不要在 `build()` 中或重复在列表行中创建 bloc。关闭手动拥有的 bloc/stream 并避免无应用级生命周期需求的全局单例功能 bloc。

## Widget 集成与副作用

使用 `BlocBuilder` 进行渲染，`BlocListener` 进行一次性导航/对话框/snack/分析，`BlocConsumer` 仅在同一子树真正同时拥有两者时使用。

使用 `context.watch`/选择器进行渲染，在回调中使用 `context.read`。`buildWhen`/`listenWhen` 可缩小工作但不能修复单一庞大的状态模型。

监听器必须在有意义的转换上触发一次。导航和对话框属于监听器/路由编排，永远不在 reducer/事件/状态或构建期间。

## Verification

- 测试初始状态、命令/事件、精确发出序列和 repository 交互。
- 覆盖任务所属的加载/空/刷新、验证/冲突/禁止/离线、提交成功/失败、重复预防和有序/并发行为。
- 验证不可变更新和返回的记录/版本协调。
- 测试 provider 生命周期/释放和路由/标签重建后无重复 bloc 实例。
- 验证 builder 渲染任务所属状态和监听器为显示目标仅触发一次。
- 在适用时分析/构建生成的不可变状态代码。

## 交付证据

标识 Cubit-vs-Bloc 选择、事件/命令、状态序列、并发和证明它的监听器断言。单个 `blocTest` happy path 或 widget provider 存在不能证明目标标识、错误恢复、生命周期、并发或副作用去重。

## 不安全默认行为

- 在无已选择技术栈和共享状态所有权的情况下加载/引入 Bloc。
- 仅为统一性对琐碎本地状态选择 Bloc。
- 稍后读取可变选定状态的模糊事件。
- bloc/状态中的可变状态集合或持有的 `BuildContext`。
- 在构建期间创建或通过 `BlocProvider.value` 泄漏的 bloc 实例。
- 在构建期间发出或由监听器重复的导航/snack/对话框操作。

# Flutter 与 Riverpod 的共享状态

仅当 TechnicalBaseline 选择 Riverpod 且任务拥有共享客户端状态时应用 Riverpod。本地临时 widget 状态不需要 provider，Bloc 选择的项目不应接收 Riverpod 模式。

## Provider 选择

选择匹配生命周期和命令的最小 provider：

| 需求 | Riverpod 边界 |
|---|---|
| 无状态依赖/派生值 | `Provider` |
| 只读有限异步数据 | `FutureProvider` |
| 长寿命流 | `StreamProvider` |
| 带命令的同步状态 | `NotifierProvider` |
| 带命令的异步工作流 | `AsyncNotifierProvider` |
| 参数化稳定标识 | `.family` with immutable/equatable key |

不要将 `StateProvider` 用作非结构化功能 store，或当表单/widget 拥有草稿时为每个文本字段创建 provider。

## Notifier 与状态契约

保持状态不可变并显式建模工作流结果。Notifier 方法表示产品命令并协调 repository/port，不持有 `BuildContext`。

```dart
@riverpod
class OrderDetail extends _$OrderDetail {
  @override
  Future<OrderViewModel> build(String orderId) =>
      ref.watch(orderRepositoryProvider).load(orderId);

  Future<void> approve(int expectedVersion) async {
    final current = await future;
    state = const AsyncLoading<OrderViewModel>().copyWithPrevious(state);
    state = await AsyncValue.guard(() =>
      ref.read(orderRepositoryProvider).approve(current.id, expectedVersion));
  }
}
```

精确的生成 API 取决于所选 Riverpod 版本。仅在 UX/产品行为接受过期内容时在变更期间保留先前数据。

不要修改被监听的列表/map/model 实例。使用 copy/update 方法并协调服务端返回的标识/版本/状态。

## 读取、监听与副作用

使用 `ref.watch` 进行渲染/派生依赖，在事件命令中使用 `ref.read`。在稳定的 widget/provider 边界使用 `ref.listen` 进行一次性导航/对话框/snack/分析行为。

永远不要在回调中调用 `watch`、在构建期间分派命令或在 widget 重建时创建重复监听器。监听器必须区分初始/加载/数据/错误转换并避免重复副作用。

当 widget 需要一个稳定字段时使用 `select` 缩小重建。不要每次选择新分配的集合/视图模型并期望减少重建。

## 生命周期、Family 与失效

对应在不被观察时结束的路由/参数状态使用 auto-dispose；keepAlive 仅配以显式新鲜度/缓存/失效策略。无界 family 键可能保留内存/网络状态。

Family 参数必须是稳定值键，而非可变对象或整个 DTO。在变更、标识/租户变更、登出或已接受过期事件后失效/刷新。

避免循环 provider 依赖和隐藏的全局 provider 容器。使用一个应用 `ProviderScope` 加所有权所需的有意嵌套 override/scope。

## AsyncValue 与错误

渲染 `AsyncValue` 的 loading/data/error，同时在功能状态/视图模型中保留空、验证、冲突、禁止、离线/不可用、过期和意外错误之间的区别。

刷新、初始加载和变更是不同的 UX 状态。不要在后台刷新期间擦除可用数据或在错误后使提交控件保持禁用。

取消/释放必须在支持处到达 repository/client。不要从 notifier 为必需业务效果启动未跟踪的 future。

## 依赖与生成代码

通过 provider 注入 repository、时钟、存储、权限和平台 adapter，在测试中 override 它们。Provider 不应读取分散的环境/全局单例。

当注解/生成被选择时，通过仓库的 build_runner 策略保持源和生成文件同步。不要手动编辑生成的 provider。

## Verification

- 测试任务所属的 provider/notifier 初始、加载、数据、空、验证/冲突/禁止/离线、刷新、变更和恢复转换。
- 验证不可变更新和服务端回读/版本协调。
- 测试 family 键/生命周期、auto-dispose、失效、登出/租户清除和无重复请求/监听器。
- 仅在声称性能时验证 `select`/scoping 限制重建。
- 用 fake override 基础设施并断言命令命中显示目标。
- 当注解或生成 provider 变更时重新生成/分析。

## 交付证据

标识 provider/notifier 命令、生命周期和证明它的状态转换断言。`ConsumerWidget` 渲染一个值或生成文件存在不能证明失效、释放、命令失败、不可变性或副作用去重。

## 不安全默认行为

- 在无已选择技术栈和共享状态所有权的情况下加载/引入 Riverpod。
- `StateProvider` 用作通用功能 store。
- 原地更新的可变列表/model 状态。
- 回调中的 `watch` 或构建期间创建的命令/监听器。
- 以可变/完整 DTO 对象为键且无限 keep-alive 的 family。
- 异步错误折叠为通用空/错误文本。
- 生成文件手动编辑或留过期。

# Kotlin Compose 质量

## When To Use

- 任务变更了 Jetpack Compose UI、Compose Multiplatform UI、ViewModel、状态持有者、导航、Material 主题使用、effect、列表、动画或 UI 测试。
- 当声明式 UI 状态、重组、生命周期、可访问性或移动/桌面 UI 行为影响正确性时使用此参考。
- 如果 UI 不是 Compose，改用相关的前端/UI 参考。

## Implementation Focus

- 将状态提升到需要协调它的最低稳定所有者。当需要父级/ViewModel 所有时保持 composable 大部分无状态，仅对局部 UI 状态使用本地 `remember`。
- 将屏幕状态作为显式模型暴露，优先为 sealed 或 data-state 基础，覆盖与工作流相关的加载、空、错误、成功、编辑、保存和禁用状态。
- 在 Android 上在 composable 中收集 flow 时使用生命周期感知收集，如 `collectAsStateWithLifecycle`。
- 将副作用保持在 `LaunchedEffect`、`DisposableEffect` 或其他 Compose effect API 中并使用正确的键。不要直接从 composable 主体触发网络调用。
- 当行可能变更、更新或重排序时在 lazy list 中使用稳定项键。避免大型集合的无界 eager 渲染。
- 仅对应在重建后存活的小型可序列化 UI 状态使用 `rememberSaveable`；不要在保存状态中存储 repository、客户端或大对象。
- 保持 Material 主题、排版、间距和组件风格与应用一致。不要为每个屏幕创建一次性颜色和布局。
- 通过 `DisposableEffect` 或生命周期所有者销毁监听器、回调、传感器、互操作句柄和其他外部资源。
- 避免 UI 渲染中的 `!!`。使 null/缺失状态显式，使屏幕能渲染适当的加载、空或不可用状态。
- 将预览视为设计辅助，而非验证。运行时状态、导航和验证仍需测试或应用冒烟覆盖。

## Decision Rules

- 当工作流具有这些状态时定义区分加载、空、内容、验证失败、保存中和可恢复错误的屏幕状态。不要用独立标志表示互斥状态。
- 将状态提升到协调多个 composable 的最窄所有者。保持瞬态输入局部，业务状态在 ViewModel/状态持有者中，并将事件作为回调或稳定接口向下传递。
- 按使工作失效的标识为 `LaunchedEffect` 设键。变更的查询、路由 ID 或认证用户应取消旧加载；`Unit` 仅对真正的屏幕生命周期 effect 正确。
- 为小型用户可编辑值使用 `rememberSaveable`，而非 repository、客户端、大列表或必须从真相来源重新加载的值。
- 保持导航参数类型化并在边界验证。不要让 composable 静默将缺失参数视为有效空标识符。
- 在引入屏幕本地颜色、间距或排版之前应用主题令牌和共享组件。在一个状态下看起来正确的预览不是交互状态完整的证据。

## Verification Focus

- 为变更的 UI 模块运行配置的 Compose/Android/桌面构建或测试任务。
- 测试任务涉及的加载、空、错误、成功、验证、禁用和保存状态的状态渲染。
- 对于 ViewModel/Flow 支持的屏幕，使用协程测试支持测试状态转换和取消。
- 对于列表和 effect，当输入变更或屏幕销毁时验证稳定键和清理行为。
- 当这些路径是任务的一部分时，通过真实导航/状态所有者演练屏幕至少一个加载到内容和一个失败/重试路径。

## Evidence Focus

- 在证据总结中，说明 Compose 决策：状态提升、屏幕状态模型、生命周期收集、副作用键、lazy list 键、主题一致性、清理或 UI 状态验证。

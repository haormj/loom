# Blazor 组件与托管交付

## When To Use

仅当选中的 Blazor 技术栈中任务拥有组件、路由、表单、交互渲染模式、circuit/WASM 状态、JS 互操作、SignalR、流式处理、虚拟化或 Blazor 生命周期行为时才使用此参考。

## Implementation Focus

### Hosting And Render Mode

标识 Server、WebAssembly、Web App 静态 SSR、交互 Server、交互 WASM 或自动渲染模式，以及哪些组件/路由跨模式。状态、DI 生命周期、API 访问、密钥、延迟、重连和浏览器能力各不相同。

仅 Server 的服务/密钥不能进入 WASM。预渲染组件可能执行两次初始化；使数据加载/幂等副作用和持久组件状态显式。

不要在不考虑序列化、包可用性、下载大小、认证和回退的情况下按组件选择渲染模式。

### Component Contracts

对值使用参数，对父级意图使用 `EventCallback<T>`，在回调中使用稳定 ID。`[EditorRequired]` 改善工具但不强制运行时存在。

避免直接变更参数/级联对象。将可编辑表单/视图模型与 API/领域/EF 记录分开并在参数变更后定义重置/重置。

保持级联值窄、稳定且仅在标识永不变时 `IsFixed`。大型可变应用状态级联可能重新渲染宽泛树并泄露 circuit/用户状态。

### Lifecycle And Async Work

对稳定的首次加载使用初始化，对路由/参数依赖加载使用参数设置后，仅对 DOM/JS 可用性使用渲染后。守卫首次渲染而不隐藏所需的后续参数行为。

在参数/导航/销毁变更时取消/排序异步工作，使旧结果不能覆盖当前记录。当外部回调更新组件状态时调用 `InvokeAsync`。

避免不必要的 `StateHasChanged`、渲染循环和渲染期间的副作用。`ShouldRender` 优化不得抑制验证/认证/数据变更。

### Forms And Validation

使用一个 `EditContext`/表单所有者并具有字段和业务验证、无效中间值、重复提交防护、后端错误和返回回读。

不要依赖禁用控件/隐藏 UI 进行授权。拒绝后保留有效输入并以可访问方式聚焦/关联验证摘要/消息。

根据托管模式和已接受的 API 边界映射防伪/认证/会话行为。

### JS Interop

在拥有的组件/服务处导入 JS 模块并创建 `DotNetObjectReference`/JS 对象句柄并异步销毁。处理导航/断开（`JSDisconnectedException`）并避免在交互渲染之前调用。

保持互操作载荷有界/可序列化；对大数据使用流式 API。验证浏览器提供的数据并永远不要通过宽泛的可调用回调暴露特权服务器方法。

### Server Circuits, WASM, And State

Blazor Server 作用域服务是 circuit 作用域的，而非一个 HTTP 请求。不要将其他用户/circuit 状态放在单例/静态字段中；绑定每 circuit 内存并处理重连/断开。

WASM 客户端状态/配置/存储是公开且不可信的。将授权和密钥保留在服务器 API 上并定义离线/网络失败行为。

对于持久/预渲染状态，按组件/用户/路由键控并防止跨用户复用。

### Routing, Auth, And Errors

验证路由/查询参数并渲染未找到/禁止/不可用状态。`AuthorizeView` 和路由 UI 是展示；后端/端点必须强制访问。

在连贯区域使用错误边界并具有恢复/导航/日志。它们不替代预期的验证/业务失败且不得暴露原始异常。

### Lists And Streaming

对大型集合使用分页或 `Virtualize` 并具有稳定项键/提供者取消/总数语义。不要将无界服务器数据渲染到 circuit/浏览器中。

流式渲染/骨架需要确定性最终状态且在预渲染/交互转换期间无重复数据加载/副作用。

## Verification Focus

- 构建/发布实际渲染模式并运行聚焦的组件/集成测试。
- 测试参数/路由变更、预渲染双重执行、加载/空/错误/表单/回读、认证拒绝、重连/断开和销毁。
- 验证 JS 模块/对象引用清理和不可用/断开行为。
- 演练 circuit/用户隔离和 WASM API 授权边界。
- 测试 Virtualize/提供者取消/标识和代表性渲染状态/可访问性。

## Evidence Focus

说明托管/渲染模式、组件/表单/生命周期所有者、状态/互操作隔离和渲染/集成断言。组件单元测试或成功的服务器渲染单独不证明预渲染、circuit、WASM、JS 或授权行为。

## Unsafe Defaults

- 为非 Blazor .NET 任务加载 Blazor 指导。
- Server/WASM/SSR 渲染模式被视为可互换。
- `[EditorRequired]` 或隐藏 UI 被视为运行时验证/授权。
- 在 Blazor Server 中假设作用域状态是每 HTTP 请求的。
- 导航/断开后保留 JS 句柄/回调。
- 无界集合在没有分页/虚拟化的情况下渲染。

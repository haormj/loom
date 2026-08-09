# JavaScript 浏览器 API 质量

## When To Use

- 任务变更了浏览器 fetch 逻辑、DOM 集成、存储、worker、service worker、observer、权限、定时器、动画、剪贴板、媒体或浏览器专有运行时代码。
- 当浏览器生命周期、用户交互、资源清理、可访问性状态或客户端持久化影响正确性时使用此参考。
- 如果代码仅在 Node 或构建工具中运行，改用 Node 参考。

## Implementation Focus

- 将 fetch 行为放在项目现有的客户端或小型边界之后：base URL、头、凭据、响应状态处理、JSON 解析、取消和面向用户的错误映射应保持一致。
- 对可被导航、卸载、搜索输入变更或重复提交取代的请求使用 `AbortController`。确保生命周期所有者实际调用 abort。
- 在框架生命周期或拥有模块拆卸中清理事件监听器、observer、interval、timeout、worker、媒体流和订阅。
- 将 `localStorage` 和 `sessionStorage` 视为小型、同步、易失败的存储。保护 JSON 解析，处理配额/安全失败，不要存储密钥，除非应用已有已接受的安全模型。
- 对较大或结构化的离线数据使用 IndexedDB 或现有客户端缓存，并在 schema 形态变更时定义版本升级行为。
- 将基于权限的 API（如剪贴板、通知、摄像头、麦克风、地理位置和文件系统访问）放在用户操作和可见的拒绝/错误状态之后。
- 避免用重度解析、渲染或计算阻塞主线程。当任务可能产生明显 UI 卡顿时使用防抖、调度、worker 或分块处理。
- 在框架应用中，不要用直接 DOM 变更绕过框架的状态/渲染模型，除非集成需要它的库；将此类集成隔离在组件或适配器之后。
- 保持浏览器特性使用与配置的目标兼容，或包含现有的 polyfill/转译路径。

### Worker And Observer Ownership

Worker、service worker、observer、媒体流和事件监听器需要显式所有者。在边界验证消息形态，报告 worker 错误，当功能不再使用时终止 worker，在拆卸期间断开 observer。捕获组件或 DOM 子树的回调不得比该所有者活得更久。

### Storage And Cache Evolution

将浏览器存储视为不可靠边界：解析、配额、隐私模式/安全错误、过期 schema 和不可用 API 是预期状态。以事务方式版本化 IndexedDB 升级并定义旧记录如何迁移或丢弃。Service-worker 缓存名需要版本控制、失效、安装/激活失败处理和显式网络/缓存策略；缓存的成功不得隐藏更新的错误或授权变更。

### Permission And Main-Thread Boundaries

从用户操作请求权限，保留拒绝和忽略状态，避免重复提示。当 CPU 密集解析或转换可能阻塞交互时，将其移至 worker或有界分块。验证所选的浏览器 API 和回退路径与仓库的目标浏览器匹配，而不是假设现代 API 到处存在。

## Verification Focus

- 为变更的浏览器代码运行浏览器导向的测试或框架构建。
- 当 fetch 逻辑变更时测试成功、网络失败、非 2xx 响应、中止/卸载和格式错误响应路径。
- 当生命周期代码变更时验证 observer、worker、定时器和事件监听器的清理。
- 对于面向 UI 的浏览器行为，冒烟测试相关视口或交互路径，而非仅依赖单元测试。
- 对于 worker、存储、service worker、权限和 observer，按适用情况验证不可用/拒绝、格式错误、升级、拆卸和过期结果路径。

## Evidence Focus

- 在证据总结中，说明浏览器决策：fetch 边界、取消、生命周期清理、存储策略、权限流程、主线程保护、DOM 集成或目标兼容性。

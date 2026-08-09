# Angular 测试

仅对拥有测试的任务使用 Angular 框架测试。选择执行变更边界的最小证明。当 MCP 分配浏览器 profile 时，Playwright/浏览器参考对渲染的多视口、真实导航和端到端业务流证据保持权威。

## 证明边界

| 声明 | 首选证明 |
|---|---|
| 纯映射器/验证器/选择器/reducer | Plain TypeScript unit test |
| Service/DI/拦截器 | TestBed/injection test |
| HTTP adapter 契约 | Angular HTTP testing backend |
| 组件渲染/交互 | TestBed component test or selected harness/library |
| 守卫/resolver/导航 | RouterTestingHarness/injection test |
| RxJS 时序/顺序 | TestScheduler/marble test |
| 完整渲染工作流/视口 | Assigned Playwright browser task |

不要对每个纯函数使用组件 TestBed，也不要声称单元 DOM 测试能证明浏览器布局、CSS、跨覆盖层焦点、深链接刷新或前端/后端部署绑定。

## TestBed 与 Standalone 组件

直接导入 standalone 组件并仅提供所需协作者。当真实 provider/pipe/指令的行为是声称的一部分时保留它们；不要浅 mock 掉失败的集成。

通过支持的 fixture/组件 API 设置 signal 输入，触发更新状态的真实 DOM 事件/service 响应。避免修改私有组件字段来制造预期视图。

使用可访问角色、标签、名称、文本或已建立的稳定 test ID。断言应覆盖可见输出、启用/禁用状态、发出意图、焦点和恢复，而非仅 CSS 实现类。

## HTTP Service 与拦截器

使用仓库的 Angular 版本兼容 HTTP 测试 provider（例如 `provideHttpClient()` 配 `provideHttpClientTesting()`）或现有模块设置。每次测试后验证未完成请求。

断言精确的方法、URL/base path、参数、header、体、凭据行为、响应映射和错误映射。仅在 service 拥有时覆盖取消/去重。

不要在 mock 中重复后端行为。返回契约形状的成功和错误响应，包括 UI 使用的验证/冲突/权限/不可用情况。

## 路由、守卫与 Resolver

使用 `RouterTestingHarness` 进行路由激活、重定向、参数、查询值、守卫、resolver 和组件输入绑定。仅当路由集成不是声称内容时对聚焦的函数守卫/resolver 使用 `TestBed.runInInjectionContext`。

测试允许和阻止导航，不仅是布尔分支。深链接刷新和服务端回退仍是浏览器/运行时证据。

## RxJS 与异步行为

当虚拟时间、取消、排序、防抖、重试或并发是风险时使用 `TestScheduler.run`/marble。对于直接的有限流，直接订阅/firstValueFrom 测试更清晰。

仅对使用兼容 Zone 管理定时器的代码使用 `fakeAsync`/`tick` 并 flush 待处理工作。不要在一个测试中混合 fake timer、真实休眠、未解决 promise 和 marble scheduler。

断言成功、错误和取消时的加载/禁用清理。当生命周期为任务所属时验证 fixture 销毁时的订阅/资源清理。

## NgRx 测试

作为纯函数测试 reducer/选择器。用受控的 action 和 service 流测试 effect，断言发出的 action 和操作符语义。为组件/facade 使用 `provideMockStore`，不作为 store 实现的唯一证明。

在初始变更检测之前覆盖选择器并在测试之间重置。避免耦合不相关功能的大型初始状态 fixture。

功能注册/键、惰性 effect、持久化状态和 router-store 集成需要超越隔离 reducer/effect 测试的集成边界。

## 组件工作流状态

仅覆盖任务所属状态但包含有意义的转换：加载到就绪/空/错误、编辑草稿到验证/提交中/成功、冲突/权限失败到恢复以及破坏性确认/取消。

对于重复列表操作，证明显示的目标 ID 经历排序、筛选、分页、刷新和模态打开/关闭后存活。验证后端字段错误关联到控件且过期错误适当清除。

## 验证与清理

首先使用仓库的 runner（Karma/Jasmine、Jest、Vitest 或其他配置目标）运行变更的测试文件/项目。当 template、公共类型、路由、provider 或共享状态变更时运行聚焦的构建/类型检查/lint。在无仓库要求时不要施加通用覆盖率百分比。

销毁 fixture、验证 HTTP 请求、恢复定时器/选择器/全局状态并关闭任何自定义资源。不稳定排序或泄漏订阅是缺陷，不是添加休眠的理由。

## 交付证据

记录测试边界、场景、命令和有意义的可见/契约断言。仅通过套件计数、私有字段断言或覆盖率数字不能证明路由集成、HTTP 契约、响应式取消、可访问性、响应式渲染或浏览器流闭环。

## 不安全默认行为

- 为仅实现任务加载 Angular 测试指导。
- 修改私有状态而非练习公共行为。
- HTTP 测试仅检查发生了一个请求。
- 直接守卫测试声称是路由行为证据。
- `fakeAsync`、marble 和真实休眠不区分地混合。
- Mock store 替代 reducer/effect 正确性测试。
- 从外部技能复制的通用覆盖率阈值。

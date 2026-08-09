# Next.js 测试

仅对拥有测试的任务使用 Next.js 框架测试。为路由/服务端/客户端/action/运行时行为选择最小证明。MCP 分配的 Playwright 参考拥有真实浏览器导航、hydration、多视口渲染和部署工作流证据。

## 证明边界

| 声明 | 适用证明 |
|---|---|
| 纯解析器/映射器/schema | TypeScript 单元测试 |
| 客户端组件行为 | selected React component test tooling |
| 服务端数据/助手 | server unit/integration test with owned ports |
| Server Action | action/application integration pattern |
| Route handler | Request/Response HTTP contract test |
| 路由文件/构建边界 | production `next build` plus focused route test |
| Hydration/导航/渲染工作流 | assigned browser test |

不要用脆弱的框架内部渲染每个 Server Component，也不要声称组件测试能证明中间件、部署 rewrite、浏览器 hydration、流式传输或服务端/客户端包。

## 客户端组件

通过仓库的 React runner 用可访问查询和真实事件测试可见行为。在所属边界提供 router/action/数据适配器，不 mock 被声称的组件/状态逻辑。

在相关处覆盖 pending、验证、冲突、禁止、不可用、禁用、成功、乐观回滚和目标标识。避免私有状态/hook 实现断言。

## 服务端组件与数据

直接测试纯服务端助手和应用/数据边界。对于服务端组件，在仓库有稳定 harness 时证明授权/限定、预期结果映射和序列化；导入/序列化契约仍需生产构建。

使用确定性请求标识、缓存状态、时间和数据。在测试之间重置/失效共享/请求缓存，使顺序不影响结果。

在端口处 mock/fake 外部服务、时钟和所选 repository。不要 mock 被测试的授权、缓存键或映射行为。

## Server Action

练习类型化输入解析、认证、所有权、业务验证/冲突、成功持久化变更、重复处理、返回可序列化状态和精确重新验证/回读。

如果框架表单/cookie/重定向行为是声称的内容，不要仅单元测试导出函数而绕过它。在 harness 中保留抛出的 redirect/notFound 语义。

## Route Handler 与中间件

构造真实 `Request` 对象并断言状态、header、内容类型、体/错误形状、cookie、auth 和方法/路径行为。在拥有时测试体/大小/格式限制。

中间件 matcher/redirect/rewrite/header 行为需要聚焦的集成/构建/运行时或浏览器探测；直接函数调用不能证明 matcher 排除或有效路径拓扑。

## App Router 边界

在可用时通过稳定仓库工具练习动态/search params、加载/错误/未找到、重定向和元数据。生产构建捕获路由文件契约、服务端/客户端导入、action 序列化和运行时不兼容。

Parallel/intercepting/深链接/返回栈行为在需要实际导航/历史/焦点时属于浏览器测试。

## 运行时与环境

对于配置/运行时任务，用有效和无效的必需 env 构建/启动，然后探测精确的 header/rewrite/健康/图像/运行时行为。确保测试不打印密钥。

当公共/服务端 env 或仅服务端依赖隔离是声称内容时检查客户端输出/导入图。仅对已测量的性能任务使用包证据。

## 验证与清理

首先运行变更的测试目标，当路由、服务端/客户端边界、action、中间件、元数据、配置或运行时变更时运行生产构建。在无仓库策略时不要为小任务发明测试 runner/覆盖率阈值。

重置 mock、缓存、环境、cookie、定时器、测试服务器、数据库 fixture 和全局 fetch 实现。避免任意休眠和测试顺序依赖。

## 交付证据

记录边界、场景、命令和有意义的 HTTP/可见/构建断言。仅通过计数或 `next build` 不能证明业务/auth 分支、缓存一致性、浏览器 hydration、导航历史、响应式 UI 或部署绑定。

## 不安全默认行为

- 仅当已接受任务拥有 Next.js 测试创建、测试修改或测试专用验证时才加载此参考。
- 客户端组件测试声称是 Server Component/运行时/中间件证明。
- Action 测试绕过 auth/事务/重新验证/回读。
- 跨测试泄漏的共享缓存/env/cookie 状态。
- 构建成功用作唯一行为证据。
- 在无实际浏览器边界的情况下断言浏览器行为。

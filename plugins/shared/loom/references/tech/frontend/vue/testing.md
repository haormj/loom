# Vue 组件、Store 与 Composable 测试

仅当任务显式拥有 Vue 测试实现时使用此参考。将 Vue Test Utils/组件/store/composable 验证保留在此处；浏览器自动化参考拥有端到端导航、真实视口/浏览器证据和产物。

## 选择公共边界

直接测试纯逻辑，通过公共 ref/命令和生命周期测试 composable，通过隔离 Pinia 测试 store，通过用户可见行为/emit/model 测试组件，通过仓库 harness 测试路由/Nuxt 集成。

使用已建立的 runner、SFC 转换、DOM 环境、Vue Test Utils/testing library、请求 mock 和 fixture。不要为一个任务添加另一个 runner 或测试栈。

测试应因产品/组件契约回归而非内部重构而失败。

## 挂载与查询

使用所需 router、Pinia、i18n、UI 库、query/client、主题、auth、注入和 Nuxt app context 挂载。仅在子渲染无关时保持浅挂载；stub 一切可能擦除被测行为。

在仓库支持处优先使用可访问角色/名称/标签/文本查询。Test ID 和 CSS class 是回退，不是语义控件的替代。

断言渲染状态、焦点/禁用行为、精确的发出/model 载荷、稳定命令目标和最终回读。

## 组件与 Model

在拥有处覆盖 prop 默认值/可空性、slot 内容/prop、emit、`v-model` 更新/修饰符、无效中间表单值、Teleport 覆盖层和异步组件失败。

根据要证明的内容有意识地附加或 stub Teleport 目标。在适当的集成层验证对话框焦点/关闭/返回行为。

避免对动态工作流使用大型快照。仅在本地约定支持时小型快照可保护稳定生成标记。

## Composable 与生命周期

当 composable 使用 inject、生命周期 hook、watcher 或清理时在组件/effect scope 内运行它们。直接调用函数可能永远不会执行真实的拥有生命周期。

变更响应式输入并断言取消/过期结果预防、卸载时清理和公共状态/操作。除非性能是契约否则不要断言私有 watcher 计数。

仅对定时器所属行为使用 fake timer 并始终恢复。

## Pinia 与持久化

为每个测试创建新的 Pinia/store/query cache。谨慎使用测试 Pinia：stub 的 action 不能证明 action 副作用或转换。

在拥有处练习选择变更、重复提交、乐观回滚、缓存失效/回读、持久化 hydration/迁移和标识清理。

重置本地存储、service worker/原生 mock 和模块单例，使测试不能从先前状态通过。

## Router、Nuxt 与异步数据

通过仓库 router/Nuxt 工具测试参数/查询变更、导航意图、守卫/middleware、未找到/禁止状态和返回上下文。

在已接受的 HTTP/client/服务端边界 mock 并断言方法/路径/查询/体/状态/错误映射。等待可见结果或 flush 的 promise 而非任意休眠。

将 SSR/hydration/Nitro/运行时配置证据保留在 Nuxt 构建/集成测试中；jsdom 组件挂载不能证明服务端/客户端分离。

## 类型契约

当 props/emit/model/slot/template ref/注入/store/plugin 变更时将运行时测试与 `vue-tsc` 配对。运行时测试不证明 template 消费者类型，typecheck 不证明外部输入验证。

谨慎为可复用库契约使用编译 fixture，确保预期错误断言保持有意。

## Verification

- 运行受任务影响的聚焦组件/store/composable 测试加 SFC 类型/构建检查。
- 为拥有的变更/异步界面证明成功和有意义的验证/业务/冲突/不可用状态。
- 验证精确的 emit/model/导航/请求目标和无意外调用。
- 练习清理、被取代的异步工作、隔离的 store/缓存和持久化重置。
- 将浏览器/原生/PWA 工作流声明与单元/组件证据分开。

## 交付证据

命名公共行为、harness 边界、代表性状态和回归时会失败的断言。通过的浅挂载、快照或 mock 的 action 不能建立生命周期、store 转换、路由、SSR、浏览器或原生行为。

## 不安全默认行为

- 仅当已接受任务拥有 Vue 测试创建、测试修改或测试专用验证时才加载此参考。
- 尽管有仓库工具仍引入新 runner。
- 每个 child/action/router/client 都 stub 直到没有真实行为残留。
- 在无组件/effect 生命周期的情况下调用 composable。
- 跨测试泄漏的共享 Pinia/缓存/存储。
- 用于异步同步的任意休眠。
- 从 jsdom 组件测试做出浏览器/SSR/原生声明。

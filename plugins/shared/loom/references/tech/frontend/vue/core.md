# Vue 响应式与 Composable 交付

在仓库的 Vue 版本、Options/Composition API 约定、语言策略、路由、状态/数据库、SFC 工具、组件系统和 UI 质量契约内实现任务所属的 Vue 界面。不要附带转换已建立的模式。

## 仓库约定

当 `<script setup>` 和 Composition API 已建立或任务显式拥有迁移时使用它们。当转换会扩大范围或改变行为时，保留一个连贯的 Options API 功能。

在使用 `defineModel`、reactive props destructure、generic SFC 语法或其他版本特定行为之前，确认 Vue/compiler 版本和已启用的宏。外部"现代 Vue"示例不是兼容性证明。

将路由/页面组件保持为工作流编排，当状态、effect 或复用使界面难以检查时提取聚焦的功能组件/composable。

## Ref、Reactive 与标识

对原始值、可空/可替换对象、template ref 和替换有意义的值使用 `ref`。对保留一个代理标识的内聚对象状态使用 `reactive`。

不要将 reactive 对象解构为普通值。使用 `toRefs`、`toRef`、store helper 或通过代理访问。避免以断开消费者连接的方式替换 `reactive` 对象。

仅当不需要深度跟踪且更新显式替换/触发标识时，对大型不可变载荷或第三方实例使用 `shallowRef`/`markRaw`。

当生命周期不同时，将可编辑草稿、持久化/API 记录、选定快照、筛选器、待处理操作和乐观值分开。

## 派生状态与 Watcher

对纯派生值使用 `computed`，仅对真正的受控转换使用可写 computed。不要用 watcher 保持冗余状态同步。

当源/旧值/时序控制重要时使用 `watch`；对依赖在同步执行期间被有意发现的简洁 effect 使用 `watchEffect`。在 `await` 之后访问的异步依赖不会按预期自动跟踪。

对 DOM 相关工作有意识地选择 `flush` 时序，避免深度监听大型对象。改为监听 getter 或规范化子集。

用 watcher 清理（`onCleanup`/支持的清理 API）、`AbortController` 或操作标识取消或使过期异步工作失效，使先前的路由/筛选/记录不能覆盖当前状态。

## 生命周期与 Effect Scope

在 setup 期间同步注册生命周期 hook。释放监听器、定时器、观察者、订阅、worker、浏览器/原生集成和手动创建的 watcher。

仅当 composable/plugin 创建一组具有独立生命周期的 effect 时使用 `effectScope`，并暴露/执行 scope 释放。组件拥有的响应式 effect 通常在卸载时自动停止。

Template ref 在挂载前和卸载/条件移除后为可空。优先使用声明式渲染；仅对焦点、测量或第三方集成使用 DOM/命令式 ref。

## Composable 契约

为可复用的有状态行为或复杂的外部生命周期提取 composable，而非仅仅为了移动代码。按仓库约定接受 ref/getter/值，在适当时用 `toValue` 等支持的工具规范化。

返回精小的公共契约，当调用者不应修改时返回只读状态，为转换返回命名命令。避免在通用 composable 中隐藏 router、全局 store、租户、auth 或宽泛 API 行为。

对于异步工作，暴露有意义的 idle/loading/refreshing/ready/empty/error/mutating 状态、重试/取消语义和稳定的命令目标。

## 工作流与 UI

在受影响区域附近表示任务所属的加载、空、验证、冲突/过期、禁止、不可用、提交中、成功、禁用和回滚状态。

拒绝后保留有效输入，阻止重复提交，并协调返回的标识/版本/状态。产品 UI 不得暴露运行时命令、框架说明、交付备注或验证指令。

使用语义元素、标签、焦点处理、宣告和仓库 UIX 令牌/组件。Vue 指令和过渡必须保留键盘和减少动画行为。

## Verification

- 运行仓库提供的聚焦 SFC 类型/构建/lint 和组件/composable 测试。
- 证明源替换、解构边界、路由/记录变更和异步完成排序后的响应式更新。
- 测试 watcher 清理/取消、生命周期释放和外部资源的重新挂载行为。
- 练习所属的工作流状态、草稿保留、重复阻止、稳定目标和最终回读。
- 验证语义、焦点、键盘交互、长/本地化内容和响应式行为。

## 交付证据

命名响应式所有者、ref/reactive/computed/watch 决策、composable 生命周期和证明可见行为的断言。成功渲染或通过类型检查不能证明过期工作安全、清理、草稿完整性或可访问性。

## 不安全默认行为

- 将 Composition API 或 TypeScript 迁移附着到不相关的功能工作。
- Reactive 对象解构为非响应式值。
- 用 watcher 镜像 computed 状态。
- 在无有界源的情况下深度监听大型数据。
- 在所有者变更后接受异步 watcher 结果。
- 通用 composable 隐藏 router/auth/API/全局依赖。
- 在挂载前访问 DOM ref 或用于声明式行为。

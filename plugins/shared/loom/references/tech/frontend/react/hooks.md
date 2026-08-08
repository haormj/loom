# React Hooks 与响应式客户端生命周期

当任务拥有 effects、订阅、浏览器集成、定时器、可替换的异步工作、自定义 hooks 或其他响应式客户端流时，应用 hooks 指导。静态/展示型组件工作不应加载此参考。

## Effects 是同步

使用 effects 将 React 状态与渲染外部的系统同步：DOM/浏览器 API、订阅、定时器、分析、命令式控件或手动数据源。不要使用 effects 从 props/state 计算值或将一个状态变量镜像到另一个。

每个 effect 保持一个关注点，并包含所有响应式依赖。重构不稳定的对象/函数或移动逻辑，而非抑制 exhaustive-deps。

```tsx
useEffect(() => {
  const controller = new AbortController()
  void search(query, { signal: controller.signal }).then(setResults, error => {
    if (!controller.signal.aborted) setFailure(mapError(error))
  })
  return () => controller.abort()
}, [query])
```

处理开发环境 Strict Mode 的 setup-cleanup 重放；effect 必须可安全重复执行，cleanup 必须撤销 setup。

## 清理与过期工作

移除监听器/观察者/订阅，清除定时器/动画帧，中止请求，释放外部实例，并防止过期完成覆盖较新状态。

挂载标志可以阻止更新但不能取消工作。在可用时优先使用真正的取消/令牌排序，并定义最新/顺序/重复行为。

当回调依赖先前状态时使用函数式更新。Refs 保存可变的非渲染值，如 DOM 节点、定时器 ID、先前值和外部实例；UI 可见状态属于 state/reducer/store。

## 自定义 Hooks

为可复用的有状态行为或隔离复杂的外部生命周期提取 hooks，而非仅仅为了移动代码。保持类型化的输入/输出契约精小，暴露状态/操作而非实现内部。

自定义 hooks 遵守 hook 规则，不应有条件地调用 hooks。避免在通用 hook 中隐藏全局单例、隐式路由或宽泛的 API/错误行为。

对于异步 hooks，暴露有意义的 idle/loading/ready/empty/error/refreshing/mutating 状态和取消/重试语义。当失败重要时不要仅返回 `data | null`。

## 记忆化

对已测量的昂贵派生或需要稳定标识的场景使用 `useMemo`，对依赖函数标识的消费者使用 `useCallback`，在已证实的组件边界使用 `memo`。

记忆化不是语义正确性，可能保留过期依赖/对象或比重新计算成本更高。不要包装每个处理器/值。

React Compiler 或框架优化可能改变手动 memo 需求；遵循已接受的工具/版本并验证行为/性能，而非机械地删除/添加 memo。

## 浏览器 API 与 SSR

当 SSR/预渲染可能时，以惰性/effect 侧方式初始化仅浏览器值，并在服务端/首次渲染时使用确定性输出。保护 storage、媒体查询、ResizeObserver、window/document 和第三方控件。

Storage 事件、媒体订阅和外部存储在多个组件需要保持一致时需要 `useSyncExternalStore` 或正确的订阅快照契约。

验证持久化数据，并在标识/租户/模式变更时清除/重新限定。浏览器存储不是安全存储。

## 防抖、定时器与事件

根据产品行为进行防抖/节流，并在卸载/输入变更时取消待处理工作。避免每次渲染重新创建防抖函数或闭包过期值。

对于用户引起的操作，事件处理器优先于 effects。不要设置标志然后仅用 effect 来注意标志并提交。

## Verification

- 测试 setup/cleanup 重放、卸载释放和依赖驱动的重新订阅。
- 证明过期请求/定时器结果不能覆盖较新状态。
- 仅在拥有时用受控定时器验证防抖/节流时序和取消。
- 测试 SSR/缺失浏览器 API、持久化数据验证、标识清除和外部存储更新。
- 练习自定义 hook 的公共状态/操作，而非仅私有 refs/effect 计数。

## 交付证据

命名外部系统/生命周期以及证明它的清理/取消断言。完整的依赖数组或通过 happy-path hook 渲染不能证明过期结果安全性、strict-mode 重放、资源释放或 SSR 行为。

## 不安全默认行为

- 将 effects 用于派生值或用户事件命令。
- 禁用 hook lint 而非修复依赖。
- 将挂载标志视为取消。
- 将 refs 用于 UI 状态或隐藏在 hooks 中的可变全局。
- 全局使用 useMemo/useCallback/memo。
- 在 SSR 初始渲染期间读取浏览器 API 而无稳定回退。

# UIX 技术栈：Svelte

用于 Svelte、SvelteKit 和相关组件驱动的项目。

## 结构

- 遵循现有的路由、布局、store 和组件约定。
- 保持页面编排、业务组件、可重用 UI 原语和数据模块分离。
- 一致使用 SvelteKit load/actions 或仓库的数据方法。

## 建议的拆分

```text
src/routes/
src/lib/components/ui/
src/lib/components/feature-name/
src/lib/server|services/
src/lib/styles/tokens.css
```

## 实现规则

- 直接在模板中表示加载、空、错误、验证、成功和业务阻塞状态。
- 仅在状态跨界面共享时使用 stores；保持本地状态本地。
- 在 CSS 变量、应用 CSS、Tailwind 或现有样式系统中使用语义令牌。
- 保持过渡有目的并尊重减弱动效。
- 避免在过于聪明的响应式语句中隐藏产品行为。
- 将令牌模板合并到现有应用 CSS、Tailwind 配置或 SvelteKit 布局资产中。不要创建每组件的并行令牌块。
- 将业务操作靠近显示受影响对象的组件，以便成功/错误可以就地更新。

## 模板模式

```svelte
{#if state.status === 'loading'}
  <SkeletonRows />
{:else if state.status === 'error'}
  <ErrorState message={state.message} />
{:else if state.status === 'empty'}
  <EmptyState />
{:else}
  <DataTable rows={state.data} />
{/if}
```

## 验证

- 存在时运行聚焦的构建/类型/lint 命令。
- 渲染工作流并检查状态转换、焦点和响应式行为。
- 确认 stores、表单和过渡在加载/错误/成功状态间保留任务上下文。

## 页面加载和操作边界

对于 SvelteKit，页面数据、表单操作、布局和服务端/客户端边界属于框架工程契约。UIX 拥有它们的状态如何出现以及用户如何在界面中移动时保持上下文。

```text
layout shell -> page data state -> feature region -> form/action
-> pending/validation/result -> invalidate or reconcile affected region
```

- 在布局中保持持久外壳，在页面或功能组件中保持路由拥有的内容。
- 在它们解释的区域或操作旁边渲染 `loading`、`empty`、`error`、`validation`、`success` 和 `business-blocking` 状态。
- 对真正共享的 UI 状态（如导航或跨路由筛选器）使用 stores；可能时将选定记录和草稿保留在拥有界面本地。
- 操作后，使确切受影响数据无效或更新，并保留筛选器、选择和返回上下文。
- 保持过渡有目的且有界。过渡不得隐藏状态变化、移动主要操作或阻止键盘焦点。

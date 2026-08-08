# UIX 技术栈：Vue

用于 Vue、Nuxt、Vite Vue 和相关组件驱动的 Vue 项目。

## 结构

- 遵循现有的 Nuxt/Vue 路由、布局、composable 和组件约定。
- 分离应用外壳/布局、页面视图、业务组件、composable 和可重用 UI 原语。
- 在添加新系统之前使用现有设计系统或令牌方法。

## 建议的拆分

```text
components/
  layout/
  ui/
  feature-name/
composables/
  useFeatureQuery
  useFeatureMutation
pages|app/
assets|styles/
  tokens
```

## 实现规则

- 保持响应式状态作用域化：页面/查询状态、表单状态、选定记录和模态/抽屉状态不应冲突。
- 对派生 UI 标签、资格和筛选数据使用计算值。
- 保持异步加载/错误状态靠近依赖它们的视图。
- 对重复的表格操作、状态徽章、字段行和空/错误状态使用 slot/组件。
- 在自定义控件上保留可访问性属性。
- 将令牌模板适配到现有 Vue/Nuxt CSS、应用配置、Tailwind 配置或主题插件中。不要在现有资产旁边创建第二个视觉系统。
- 当跨视图重用时将业务格式化和验证保留在 composable/助手中。

## 模板模式

```vue
<template>
  <AppShell>
    <PageHeader />
    <FilterBar />
    <SkeletonRows v-if="state.status === 'loading'" />
    <ErrorState v-else-if="state.status === 'error'" />
    <EmptyState v-else-if="state.status === 'empty'" />
    <DataTable v-else :rows="state.data" />
  </AppShell>
</template>
```

## Nuxt 说明

- 使用布局作为持久外壳。
- 保持服务端/仅客户端代码分离。
- 避免将交付命令、构建说明或技术栈说明放入页面。

## 验证

- 存在时运行聚焦的构建/类型/lint 命令。
- 渲染并检查响应式行为。
- 验证表单和过渡保留用户输入和焦点。
- 确认选定的 UIX 参考反映在变更的视图/组件文件中，而非仅结果散文中。

## 页面、Composable 和运行时边界

Vue UIX 拥有可见组合和状态放置。Vue/Nuxt 工程参考拥有路由、SSR、数据获取、运行时配置和服务端处理边界。

```text
layout/shell -> page view -> feature component -> composable/data adapter
                                      \-> loading/empty/error/action feedback
```

- 使用布局作为持久产品 chrome，使用页面作为路由拥有的工作流组合。
- 保持 composable 聚焦于可重用状态或行为；不要在一个 composable 中隐藏无关的导航、API 和展示逻辑。
- 分离查询状态、可编辑草稿、选定记录和操作状态，以便刷新不能覆盖用户输入或变更错误记录。
- 对于 Nuxt，保留 SSR 确定性、水合安全的浏览器访问、路由中间件、服务端/客户端边界和直接深度链接行为。
- 变更后，更新或使确切受影响数据无效，并在工作流需要时保留用户的筛选器、选择、滚动和返回路径。

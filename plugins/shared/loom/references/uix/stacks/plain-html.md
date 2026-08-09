# UIX 技术栈：纯 HTML

用于静态 HTML、原生 JavaScript、简单 Tailwind/CDN 页面、服务端渲染模板或无前端框架的项目。

## 结构

- 优先保持语义化 HTML：地标、标题、表单、标签、表格、按钮、链接。
- 当无设计系统时使用 CSS 变量作为令牌。
- 如果项目已有单独文件则拆分 CSS 和 JavaScript；避免在非平凡产品中使用大量内联样式/脚本块。

## 最小布局骨架

```html
<body>
  <div class="app-shell">
    <aside class="app-nav"></aside>
    <main class="app-main">
      <header class="page-header"></header>
      <section class="page-feedback" aria-live="polite"></section>
      <section class="page-content"></section>
    </main>
  </div>
</body>
```

```css
:root {
  --surface: #fff;
  --text: #111;
  --border: #ddd;
  --space-4: 16px;
  --radius-md: 8px;
}
```

## 实现规则

- 渐进增强：核心内容和表单在可能时应在无复杂 JavaScript 的情况下保持可理解。
- 对抽屉、标签页、筛选器、toast 和表单等交互使用事件委托或小模块。
- 提供可见焦点状态，仅在原生语义不足时使用 ARIA。
- 为布局原语、状态和可重用组件使用稳定的类名。
- 将语义令牌放在一个样式表或服务端渲染主题包含中。避免在多个页面片段中重复自定义属性。
- 一致地使用 `data-state` 或状态类来表示加载、空、错误、活动、选定、禁用和业务阻塞状态。

## Tailwind 说明

- 如果 Tailwind 已存在，通过配置/类映射语义决策而非任意一次性值。
- 避免隐藏重复模式的长类字符串；当重复增长时提取组件或工具类。

## 验证

- 打开 HTML 或本地服务器渲染。
- 检查键盘导航、响应式断点和表单验证。
- 验证产品 UI 不包含设置或验证说明。
- 当反馈动态变化时检查生成的 DOM 的语义地标、标签和实时区域。

## 入口、资产和增强边界

在创建文件之前找到仓库的实际 HTML 入口、样式表/主题包含、脚本入口和静态资产根。技术栈参考描述渲染边界；构建命令和服务端模板约定来自仓库和选定的工程参考。

```text
document entry -> semantic shell -> feature region -> DOM state attributes
-> event handler -> local/API result -> updated region and live feedback
```

- 将全局令牌保留在一个加载的样式表或主题包含中，并将页面特定样式靠近其拥有的界面。
- 使用原生表单和链接作为基线，以便任务在增强前保持可理解。
- 仅在 `data-state`、`aria-busy` 和 `aria-live` 描述真实状态转换时添加它们；不要将类用作未记录的第二状态模型。
- 对重复交互使用小模块和事件委托。不要将多界面产品变成带无关全局选择器的一个内联脚本。
- 当 JavaScript 不可用或延迟时，将服务端渲染内容和表单操作保留为有意义的回退。

# UIX 焦点：Web 实现

当创建或修改用户可见的浏览器界面时使用此文件。此文件涵盖 DOM、CSS、浏览器行为和 Web 框架边缘情况，使 UI 感觉达到生产级而非仅有样式。

除非任务也包含围绕它们的正常浏览器 UI，否则不要将此文件用于原生移动屏幕、小程序目标或主要 3D 场景。

## 语义可访问性

- 优先使用原生元素：`button` 用于操作、`a` 或框架链接用于导航、`label` 用于表单标签、`table` 用于真实的表格比较。
- 不要对主要控件使用可点击的 `div` 或 `span` 元素。
- 仅图标控件需要可访问名称，且当含义非通用时需要工具提示或附近文本。
- 装饰性图标应对辅助技术隐藏。
- 动态验证、保存、删除和加载反馈应在受影响区域播报，而非仅作为断开的 toast。
- 标题必须形成可用的大纲。具有多个区域的页面仍需要一个清晰的顶级标题和有意义的区域标签。
- 传达内容的图像需要有用的替代文本。装饰性媒体不应与任务竞争。

可访问控件模式：

```tsx
<button
  type="button"
  aria-label="Refresh orders"
  className="inline-flex h-9 w-9 items-center justify-center rounded-md focus-visible:outline-none focus-visible:ring-2"
>
  <RefreshCw aria-hidden="true" />
</button>
```

## 焦点和键盘

- 每个交互控件都需要可见的焦点状态。
- 仅当存在可见替代时才可移除轮廓。
- 优先使用 focus-visible 样式，这样指针点击不会产生嘈杂的焦点环。
- 复合控件如搜索框、组合框、带操作的卡片和可编辑表格行需要 focus-within 处理。
- 仅悬停控件需要键盘和触摸等价物。
- 模态框、抽屉、面板和菜单必须保留焦点流并提供明显的退出路径。

焦点模式：

```css
.control {
  outline: none;
}

.control:focus-visible {
  box-shadow: 0 0 0 3px var(--focus-ring);
}
```

## 表单和操作

- 输入需要稳定的 `name` 值和可见标签。仅占位符标签不够。
- 使用匹配值的输入类型和输入模式：email、tel、url、number、decimal、search 等。
- 深思熟虑地使用 autocomplete，让浏览器能帮忙而不会填错字段。
- 不要在正常字段中阻止粘贴。粘贴是可访问性和恢复的一部分。
- 提交控件应仅在提交开始后进入提交状态；不要在用户可以操作之前禁用主要路径。
- 字段错误应放在字段旁边。表单级摘要应链接或引导回受影响字段。
- 失败的提交必须保留用户输入和选择上下文。
- 破坏性操作根据严重程度需要确认、撤销或明确的恢复路径。
- 当导航会丢弃有意义的用户工作时警告未保存的更改。

表单韧性模式：

```tsx
<label htmlFor="supplier-email">Supplier Email</label>
<input
  id="supplier-email"
  name="supplierEmail"
  type="email"
  autoComplete="email"
  aria-describedby="supplier-email-error"
/>
<p id="supplier-email-error" role="alert">
  Enter a valid supplier email address.
</p>
```

## 布局韧性

- 长名称、标识符、表格值和用户提供的文本需要换行、截断或展开行为。
- 包含文本的 Flex 子项通常需要 `min-width: 0` 才能使截断生效。
- 空字符串、空数组、缺失的可选值和部分记录不得使布局塌陷。
- 当比较不是主要目标时，数据表格需要水平溢出或移动卡片/详情回退。
- 固定格式控件、计数器、工具栏按钮和表格行在状态文本变化时不应改变尺寸。
- 避免不需要的页面级水平滚动；修复溢出的元素而非反射性地隐藏所有溢出。

长内容模式：

```css
.record-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: var(--space-3);
}

.record-title {
  min-width: 0;
  overflow-wrap: anywhere;
}
```

## 动效和交互

- 对过渡和动画尊重减弱动效偏好。
- 对频繁过渡动画 transform 和 opacity。避免在正常产品流中动画布局属性。
- 不要使用全捕获过渡；列出应该移动的属性。
- 动效必须是可中断的。如果用户在动画中途点击、关闭、滚动或更改选择，UI 应响应。
- 拖拽、调整大小和手势流应避免意外文本选择，并在需要时保持非活动区域惰性。
- 触摸目标必须足够大，移动流不得依赖悬停。

动效模式：

```css
.drawer {
  transition-property: transform, opacity;
  transition-duration: 160ms;
}

@media (prefers-reduced-motion: reduce) {
  .drawer {
    transition-duration: 1ms;
  }
}
```

## 媒体和浏览器性能

- 内容图像需要稳定的尺寸或宽高比以避免布局偏移。
- 折叠下方的媒体应避免预加载。首屏关键媒体应通过项目技术栈的正常机制优先处理。
- 当可见计数可能超出小型操作列表时，大型列表需要分页、虚拟化、分块渲染或 content-visibility 处理。
- 避免在渲染期间读取布局测量值。仅在 CSS 布局无法解决问题时在绘制后测量。
- 昂贵的受控输入需要防抖、本地缓冲或框架特定的优化。
- 字体和远程资产应通过项目现有的性能模式加载。

列表策略模式：

```text
small bounded list -> normal render
large operational list -> pagination or virtualization
append-only feed -> incremental loading with stable item identity
wide comparison table -> horizontal overflow with labeled scroll region
```

## 导航、区域设置和水合

- 当用户合理地分享、重载或返回视图时，筛选器、标签页、分页、选定记录和展开面板应通过 URL 状态或等价导航状态可恢复。
- 导航链接必须保留浏览器功能如在新标签页打开和复制链接。
- 对面向用户的值使用区域感知的日期、时间、数字和货币格式化。
- 当 UI 支持翻译时，防止代码令牌、品牌名称、ID 和产品标识符被意外翻译。
- 在服务端渲染技术栈中，避免随机值、当前时间、仅视口值或非受控到受控输入过渡导致的水合不匹配。
- 谨慎使用客户端专用渲染转义，仅用于在服务端和浏览器之间确实无法匹配的值。

## Browser Boundary Decisions

- 将仅浏览器 API、视口测量、随机值、当前时间值和存储访问放在项目已建立的客户端边界之后。
- 优先使用 CSS 布局和媒体查询而非渲染时测量。当需要测量时，在没有偏移或隐藏主要工作流的情况下处理初始未知状态。
- 为每个可滚动区域指定有意的所有者。嵌套滚动必须保留键盘、触摸、焦点和退出行为，而非困住用户。
- 对于可增长的列表，根据预期量和交互需求选择正常渲染、分页、虚拟化或增量加载。在测量实际瓶颈之前不要用复杂渲染器优化小型列表。
- 将 API、存储和浏览器失败消息保持为产品语言并靠近受影响界面；源代码和网络细节属于诊断。

格式化模式：

```ts
const amount = new Intl.NumberFormat(locale, {
  style: 'currency',
  currency,
}).format(value);
```

## Evidence Checklist

实现证据应展示：

- 语义控件、标签、焦点行为和动态反馈源检查。
- 表单元数据、错误放置、提交状态和恢复行为（当表单在范围内时）。
- 长内容、空值、媒体、列表大小和布局溢出处理。
- 动画 UI 的减弱动效处理。
- 当这些值或流对用户可见时的区域格式化和导航状态处理。
- 服务端渲染 Web 技术栈中检查的水合敏感值。
- 当这些关注点是变更界面一部分时检查的浏览器边界、滚动归属、列表策略和仅客户端值。

## Quality Gate Index

| Gate | 通过信号 | 失败信号 |
| --- | --- | --- |
| `web.semantic_accessibility` | 变更的浏览器 UI 中存在原生控件语义、可访问名称、可见焦点和作用域反馈播报。 | 可点击的非控件、未标记的图标按钮或字段、隐藏焦点、缺失动态反馈区域或不可访问的媒体。 |
| `web.form_and_state_resilience` | 表单/操作保持有意义的元数据、内联错误、可恢复输入、安全提交状态和破坏性操作恢复。 | 仅占位符标签、阻止粘贴、失败后丢失输入、远离字段的通用错误、重复提交风险或立即执行破坏性操作。 |
| `web.runtime_layout_safety` | 长内容、空值、媒体尺寸、大型列表、减弱动效、区域格式化、水合敏感值和可恢复状态在范围内得到处理。 | 文本破坏布局、空数据渲染损坏 UI、媒体偏移布局、大型列表天真渲染、动效忽略用户偏好、值硬编码或重载丢失预期状态。 |

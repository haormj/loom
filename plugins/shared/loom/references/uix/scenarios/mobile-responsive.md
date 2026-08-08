# UIX 场景：移动响应式 Web

当 Web 产品必须在手机和平板上良好工作而非原生应用时使用。移动是一等布局，而非缩小的桌面。

## 基线

- 密度通常为 `comfortable`。
- 关键操作必须可通过触摸和键盘到达。
- 必须考虑安全区域、粘性栏和浏览器视口变化。
- 仅悬停行为无效。

## 移动布局骨架

```css
.mobile-page {
  min-height: 100dvh;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  background: var(--surface);
}

.mobile-topbar {
  position: sticky;
  top: 0;
  z-index: var(--z-sticky);
  min-height: 56px;
  padding: env(safe-area-inset-top, 0) var(--space-4) 0;
  border-bottom: 1px solid var(--border);
  background: var(--surface-raised);
}

.mobile-content {
  min-width: 0;
  overflow: auto;
  padding: var(--space-4);
}

.mobile-actionbar {
  padding: var(--space-3) var(--space-4);
  padding-bottom: calc(var(--space-3) + env(safe-area-inset-bottom, 0));
  border-top: 1px solid var(--border);
  background: var(--surface-raised);
}
```

## 必需模式

- 移动导航：抽屉、底部导航、紧凑标签页或简化的顶栏。
- 表单：单列、可见标签、正确输入类型、字段附近验证。
- 表格：转换为卡片/详情路由或仅在数据比较需要时允许作用域水平滚动。
- 主要操作：可见且安全区域感知，但不覆盖内容。
- 反馈：适当时内联加 toast。

## 响应式升级

```css
@media (min-width: 768px) {
  .responsive-workspace {
    max-width: 720px;
    margin: 0 auto;
    padding: var(--space-6);
  }
}

@media (min-width: 1024px) {
  .responsive-workspace.is-operational {
    max-width: none;
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr) 320px;
    gap: var(--space-6);
  }
}
```

## 移动交互

- 触摸目标应至少 44px。
- 对简短次要流程使用底部面板；对复杂表单使用全屏路由。
- 保持表单的键盘打开行为可用。
- 对类应用全高页面使用 `100dvh`。
- 可能时关闭抽屉/面板时保留滚动位置。

## Web 特定检查

- 视口 meta 不得禁用用户缩放。
- 软键盘打开时粘性页眉/操作栏不得覆盖焦点输入。
- 抽屉/面板的滚动锁定必须正确释放。
- 表格/列表/详情回退应保留与桌面相同的业务操作。
- 仅桌面悬停功能需要可见的移动等价物。

## 验证信号

- 响应式行为在范围内时至少检查一个窄视口和一个桌面/平板视口。
- 长标签、验证消息和业务阻塞文案换行而不隐藏操作。
- 主要操作在滚动和验证错误后保持可达。

## 避免

- 水平页面滚动。
- 固定桌面宽度。
- 主要移动输入的字体小于 16px。
- 重要移动操作的桌面式居中模态。
- 禁用用户缩放。

## 键盘、方向和恢复

响应式行为包括瞬态设备状态，而非仅断点。主要任务必须在键盘、方向或浏览器 chrome 改变可用空间时保持可恢复。

```css
.mobile-task {
  min-height: 100dvh;
  padding-bottom: calc(var(--space-4) + env(safe-area-inset-bottom));
}

.mobile-actionbar {
  position: sticky;
  bottom: 0;
  padding: var(--space-3) var(--space-4) env(safe-area-inset-bottom);
}
```

- 焦点输入在虚拟键盘上方滚动到视图中；提交或下一步操作不得被覆盖。
- 设备旋转或视口变化时保留草稿值、筛选器和选定项目。
- 方向变化后重新计算固定或粘性区域，而非依赖初始视口高度。
- 在最窄支持宽度下保持点击目标、标签、错误消息和确认结果可用。
- 用点击/焦点展示替换依赖悬停的展示，并提供从详情到列表的清晰返回路径。
- 当网络丢失或请求失败时，保留用户上下文并在受影响操作附近暴露重试或修正。

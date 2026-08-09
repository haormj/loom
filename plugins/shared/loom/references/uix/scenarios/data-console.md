# UIX 场景：数据控制台

用于分析、监控、报告、日志、查询/结果页面、运营指标和数据密集型工作台。UI 必须帮助用户筛选、比较、检查和操作数据。

## 基线

- 首屏显示数据控件和结果，而非解释性内容。
- 密度通常为 `workbench_dense`。
- 布局优先考虑扫描、比较、筛选、新鲜度和下钻。
- 颜色区分数据系列、健康/状态和交互状态而不超载单一品牌色。

## 控制台结构

```html
<main data-region="data-console">
  <header data-region="console-header"></header>
  <section data-region="query-bar"></section>
  <section data-region="summary-strip"></section>
  <section data-region="workspace">
    <aside data-region="filters"></aside>
    <section data-region="results"></section>
    <aside data-region="inspector"></aside>
  </section>
</main>
```

```css
.data-console {
  min-height: 100dvh;
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr);
  background: var(--surface);
}

.data-workspace {
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(220px, 280px) minmax(0, 1fr);
  gap: var(--space-4);
  padding: var(--space-4);
}

.data-workspace.has-inspector {
  grid-template-columns: minmax(220px, 280px) minmax(0, 1fr) minmax(320px, 420px);
}

@media (max-width: 1023px) {
  .data-workspace,
  .data-workspace.has-inspector {
    grid-template-columns: minmax(0, 1fr);
  }
}
```

## 必需模式

- 带可见当前条件和清晰重置的查询/筛选控件。
- 带加载、空、部分、错误和过期数据状态的结果区域。
- 带明确溢出行为和稳定高度的表格/网格/图表区域。
- 通过侧面板、抽屉、标签页或路由的详情下钻。
- 数据随时间变化时的时间戳、刷新或数据新鲜度指示器。
- 仅在相关且安全时的导出/复制/分享操作。

## 表格和图表区域

```css
.console-panel {
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-raised);
}

.results-scroll {
  min-height: 0;
  overflow: auto;
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: var(--space-3);
}
```

- 表格对必须对齐的值使用表格数字。
- 图表需要标签、图例、空/加载/错误状态和可读坐标轴。
- 日志和查询输出需要等宽对齐、换行或水平滚动以及复制控件。
- 指标需要周期/来源上下文；不要在无决策上下文时显示巨大的数字。

## 运营记录变体

当数据控制台实际上是 CRUD/审批/请求工作台时，将数据控制台扫描与管理仪表板记录操作结合：

- 查询/筛选区域保持可见。
- 结果区域拥有加载/空/错误/过期状态。
- 检查器/详情区域拥有选定记录事实、操作资格、历史和业务阻塞反馈。
- 变更必须在数据行/卡片和详情摘要都可见时更新两者。

```html
<section data-region="results-plus-inspector">
  <section data-region="record-results"></section>
  <aside data-region="record-inspector"></aside>
</section>
```

## 状态

- 加载保留图表/表格尺寸。
- 空区分"尚无数据"和"筛选无结果"。
- 错误将查询/系统失败与权限/业务限制分离。
- 过期数据显示最后更新时间和刷新路径。
- 长标签和值通过换行、带 title 的截断或详情展开保持可检查。

## 验证信号

- 筛选器/搜索在刷新或变更后保留条件。
- 空状态区分无记录和无匹配。
- 溢出作用域限于表格/日志/代码区域，而非整个页面。
- 相关时可见新鲜度、时间戳或结果计数。

## 避免

- 仪表板壁纸：许多图表但无决策工作流。
- 没有底层数据访问的单一巨型 KPI 卡片。
- 未标记的颜色图例或无文本的状态颜色。
- 初始加载后用旋转器替换整个控制台外壳。

## 查询生命周期

将查询界面建模为可重复的生命周期而非静态图表墙：

```text
criteria -> submitted query -> loading -> results or empty -> stale/refresh
-> selected result -> inspector or drill-down
```

- 提交后显示活动条件和清晰的重置路径。
- 当刷新可以在不使先前结果无效的情况下完成时，保持先前结果可见并显示明确的加载指示。
- 在文案和恢复操作中区分无数据、无匹配、部分结果、权限失败和查询失败。
- 打开检查器或路由详情时保留选定结果和查询上下文。
- 刷新后结果变化时，显示最后更新时间或查询版本，以便用户知道他们正在检查哪些数据。

```ts
type QueryState<T> =
  | { status: 'idle'; criteria: Criteria }
  | { status: 'loading'; criteria: Criteria; previous?: T }
  | { status: 'ready'; criteria: Criteria; data: T; updatedAt: string }
  | { status: 'empty'; criteria: Criteria; updatedAt: string }
  | { status: 'error'; criteria: Criteria; message: string; previous?: T };
```

## 结果可访问性

- 表格在存在交互处暴露列含义、行标识、可排序状态和键盘导航。
- 图表将视觉编码与可读图例、单位、时间范围和可访问的表格或文本替代配对。
- 日志和查询输出使用刻意的换行或水平滚动边界；永远不要让整个应用水平滚动。
- 长值通过换行、复制、详情展开或稳定 title 保持可检查。截断不得移除决策所需的值。
- 每个颜色编码状态也有文本、图标或等价的非颜色信号。

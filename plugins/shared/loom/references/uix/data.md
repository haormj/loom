# UIX 焦点：数据

当前端展示列表、表格、详情、图表、日志、指标、账户记录、交易或查询结果时加载此文件。

## 数据界面规则

- 在同一扫描路径中展示数据对象、状态和可用操作。
- 当数据量需要时提供筛选/搜索/排序/分页。
- 变更后保留当前筛选器和选定记录。
- 对对齐的数字列使用表格数字。
- 明确显示单位、货币、日期和时间戳。
- 区分过期、待处理、失败、阻塞和完成状态。
- 在选择、变更、加载和错误状态期间保持稳定的对象标识可见。
- 当标识符存在时使用领域的真实标识符而非生成的演示名称。

## Brief Mapping

当任务简报包含信息契约时，将其用作最小数据界面契约：

- 必需字段成为可见字段、摘要、表格列、详情事实或图表标签。
- 扫描优先级决定视觉顺序，优先于装饰性分组或本地组件偏好。
- 身份字段在选择、变更、加载和错误状态期间保持可见。
- 状态字段靠近可用操作，以便用户理解资格。
- 长内容策略控制换行、截断、下钻、溢出或响应式卡片回退。
- 选定的数据视图命名此任务应实现或保留的确切视图；不要添加无关的仪表板或摘要。

## 记录工作台模式

用于 CRUD、审批、运营、案例管理和账户/订单/请求工作流：

```html
<main data-region="record-workbench">
  <header data-region="page-context"></header>
  <section data-region="workbench-toolbar"></section>
  <section data-region="workbench-body">
    <section data-region="record-list"></section>
    <aside data-region="record-detail"></aside>
  </section>
</main>
```

列表用于查找和比较记录。详情面板用于当前状态、领域字段、相关事件和合格操作。表单或变更结果必须更新拥有受影响对象的区域。

## 表格和列表

- 表格需要标题、行标识、溢出行为、空/加载/错误状态以及分页或虚拟/无限加载（相关时）。
- 列表/卡片需要足够的元数据以支持选择而无需打开每个项目。
- 详情视图需要源上下文：选定的记录 ID/名称/状态和返回/关闭路由。
- 行操作不应仅隐藏在悬停之后。

## 表格解剖结构

```html
<section data-region="data-surface">
  <header data-region="data-toolbar"></header>
  <div data-region="data-feedback" aria-live="polite"></div>
  <div data-region="data-scroll">
    <table>
      <thead></thead>
      <tbody></tbody>
    </table>
  </div>
  <footer data-region="pagination-or-selection"></footer>
</section>
```

```css
.data-scroll {
  min-width: 0;
  overflow: auto;
}

.data-scroll table {
  width: 100%;
  min-width: 720px;
  border-collapse: collapse;
}

.data-scroll th,
.data-scroll td {
  height: 44px;
  padding: 0 var(--space-3);
  border-bottom: 1px solid var(--border);
}
```

移动记录管理流程应优先使用卡片或下钻详情。仅当用户必须比较列时使用水平表格滚动。

## 记录卡片回退

```html
<article data-region="record-card">
  <header>
    <strong data-region="record-title"></strong>
    <span data-region="record-status"></span>
  </header>
  <dl data-region="record-facts"></dl>
  <footer data-region="record-actions"></footer>
</article>
```

当表格的目的是选择或操作时，在窄屏幕使用卡片。保持需要比较的网格可滚动并为滚动区域加标签。

## 图表和指标

- 图表需要标签、图例、坐标轴或等价上下文、空/加载/错误状态以及在可行时的可访问摘要。
- 指标必须包含上下文：周期、单位、比较或来源。
- 避免不支持用户决策的装饰性图表。

## 数据量和视图选择

根据用户的工作和预期量选择数据视图：

| 需求 | 首选视图 |
| --- | --- |
| 查找或比较许多记录 | 带声明字段、排序、筛选和有界分页的表格。 |
| 选择和操作少量集合 | 带标识、状态、关键事实和行操作的列表或卡片。 |
| 检查一个记录 | 保留源上下文的详情路由、抽屉或分屏视图。 |
| 观察趋势 | 配以标签、单位、范围、比较和可读数据摘要的图表。 |
| 检查技术输出 | 带复制、搜索、筛选和失败状态的有界日志/结果界面。 |

除非图表、KPI 网格、导出控件或摘要面板支持已接受的用户决策，否则不要添加。不要使用卡片网格来避免为不断增长的集合定义表格字段或分页。

## 查询和回读状态

- 在结果计数附近显示活动的筛选器、排序、范围或查询上下文。
- 打开详情、更改记录、重试或从详情路由返回时保留查询上下文。
- 区分无匹配结果与无记录、数据不可用、数据过期和权限受限数据。
- 变更后，更新归属行/详情/指标或明确标记回读为待处理。没有更新数据的成功 toast 是不完整的。
- 对于大型或远程数据，在实现结果视图之前定义分页、延续、虚拟化或有界加载。

## 业务反馈

- 业务阻塞规则应附加到受影响的行、详情或操作区域。
- 技术错误应可恢复并与领域限制分离。
- 成功应更新行/详情状态，而非仅显示 toast。

## 证据

完成的数据 UI 应引用：

- 数据界面文件和令牌消费者文件。
- 实现的查询/列表/详情/操作状态。
- 空、加载、错误和业务阻塞的放置位置。
- 密集值和长标签的溢出/响应式行为。

## Quality Gate Index

| Gate | 通过信号 | 失败信号 |
| --- | --- | --- |
| `data.surface.scan_action_path` | 用户可以在一条路径中扫描对象标识、状态、关键字段和可用操作，加载/空/错误/业务阻塞状态靠近受影响区域。 | 数据隐藏在通用卡片后面、状态/操作与记录分离，或状态反馈仅作为全局 toast/消息出现。 |

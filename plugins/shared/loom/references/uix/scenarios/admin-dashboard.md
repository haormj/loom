# UIX 场景：管理仪表板

用于内部运营、员工控制台、CRM/ERP/CMS 后台、SaaS 控制面板和管理工具。效率、可扫描性和工作流完成比视觉 spectacle 更重要。

## 基线

- 首个视口是工作控制台，而非着陆页。
- 典型外壳：侧边栏导航、带页面上下文/搜索/操作的顶栏、主内容、可选的右侧详情抽屉。
- 密度通常为 `workbench_dense` 或 `balanced`。
- 视觉风格克制：安静的表面、强信息层次、语义状态颜色和可预测的控件。

## 简报提取

当选定此场景时，按以下方式将场景规则映射到任务简报：

| 简报领域 | 管理仪表板提取 |
| --- | --- |
| 布局 | 侧边栏/顶栏/主区域/详情区域、工作台密度、桌面分屏布局、平板导轨/抽屉、移动列表到详情回退。 |
| 信息 | 记录标识、状态、关键决策字段、筛选/搜索/排序/分页、选定详情摘要、更新/历史上下文。 |
| 操作 | 主要创建/提交/审批操作靠近工作区域；行/详情上下文操作；破坏性操作带确认或恢复。 |
| 状态 | 结果区域加载/空/错误、表单验证、操作待处理/成功以及行/详情/操作附近的业务阻塞。 |
| 视觉系统 | 克制的操作表面、令牌化间距/排版/状态颜色、紧凑应用标识、无主视觉/营销/填充部分。 |
| 内容边界 | 仅产品文案：标签、状态、验证、筛选器、操作和帮助路由。无运行时、交付、技术栈或验证语言。 |

## 必需模式

- 导航：活动部分、分组导航项、稳定页面标题、深度大于一时使用面包屑。
- 数据：带筛选器、搜索、排序、分页或无限滚动的表格/列表、空/加载/错误状态。
- 详情：行选择打开侧面板、路由详情或内联展开而不丢失列表上下文。
- 表单：分组部分、字段验证、禁用/提交状态、业务阻塞反馈。
- 操作：主要操作在相关区域附近可见；破坏性操作需要确认或恢复。
- 反馈：成功更新受影响行/详情并出现在变更对象附近。

## 布局

- 桌面：侧边栏 220-280px、顶栏 52-64px、带最小宽度处理的内容区域。
- 平板：侧边栏折叠为抽屉或导轨；筛选器可能移入抽屉。
- 移动：将表格密集视图转换为列表/详情卡片或全屏详情路由；不要依赖悬停。
- 仅在不隐藏内容时使用粘性表头或操作栏。

## 具体外壳模式

将此用作结构模式，而非必需的类名：

```css
.admin-shell {
  min-height: 100dvh;
  display: grid;
  grid-template-columns: 240px minmax(0, 1fr);
  background: var(--surface);
  color: var(--text);
}

.admin-shell.has-detail {
  grid-template-columns: 240px minmax(0, 1fr) minmax(320px, 380px);
}

.admin-sidebar {
  position: sticky;
  top: 0;
  height: 100dvh;
  border-right: 1px solid var(--border);
  background: var(--surface-raised);
  overflow-y: auto;
}

.admin-main {
  min-width: 0;
  display: grid;
  grid-template-rows: 56px minmax(0, 1fr);
}

.admin-content {
  min-width: 0;
  padding: var(--space-6);
  overflow: auto;
}
```

```css
@media (max-width: 1023px) {
  .admin-shell,
  .admin-shell.has-detail {
    grid-template-columns: minmax(0, 1fr);
  }

  .admin-sidebar {
    position: fixed;
    z-index: var(--z-modal);
    inset: 0 auto 0 0;
    width: min(280px, 86vw);
    transform: translateX(-100%);
  }

  .admin-sidebar[data-open="true"] {
    transform: translateX(0);
  }
}

@media (max-width: 767px) {
  .admin-main {
    grid-template-rows: auto minmax(0, 1fr);
  }

  .admin-content {
    padding: var(--space-4);
  }
}
```

## 组件解剖结构

管理页面通常按此顺序需要这些区域：

```html
<aside data-region="sidebar">
  <header data-region="brand"></header>
  <nav aria-label="Primary"></nav>
  <footer data-region="workspace"></footer>
</aside>

<main data-region="main">
  <header data-region="topbar">
    <nav aria-label="Breadcrumb"></nav>
    <div data-region="global-search"></div>
    <div data-region="page-actions"></div>
  </header>

  <section data-region="page-heading"></section>
  <section data-region="filters"></section>
  <section data-region="results"></section>
</main>

<aside data-region="detail-panel"></aside>
```

不要将其渲染为可见的解释文本。它是实现的结构指南。

## 数据表格模式

- 工具栏左侧：搜索、筛选器、保存的视图、刷新（相关时）。
- 工具栏右侧：主要创建/导入操作、导出（仅有用时）、列设置（仅密集数据时）。
- 表头：仅在实现排序时使用可排序列。
- 行：标识、状态、关键字段、最后更新、上下文操作。
- 页脚：分页、选择计数操作或延续令牌。
- 详情面板：选定记录摘要、状态、合格操作、历史/事件和关闭/返回控件。

```css
.admin-table-wrap {
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-raised);
  overflow: hidden;
}

.admin-table-scroll {
  overflow: auto;
}

.admin-table {
  width: 100%;
  min-width: 760px;
  border-collapse: collapse;
}

.admin-table th,
.admin-table td {
  height: 44px;
  padding: 0 var(--space-3);
  border-bottom: 1px solid var(--border);
  text-align: left;
  vertical-align: middle;
}
```

在移动端，不要简单缩小此表格。当工作流是记录管理而非电子表格比较时，使用记录卡片或下钻列表。

## 状态

- 加载：表格骨架或行骨架，而非外壳加载后的全页旋转器。
- 空：说明业务原因和下一步操作。
- 错误：显示重试并保留筛选器/表单输入。
- 业务阻塞：消息必须引用规则和受影响对象。

## 状态放置

- 列表加载属于结果区域内，而外壳保持可用。
- 详情加载属于详情面板内，而非整个页面。
- 空状态替换结果行并保持筛选器可见。
- 业务阻塞消息出现在受影响操作旁边，当操作目标是选定记录时也在详情面板中。
- toast 可以确认成功，但行/详情状态也必须更新。

## 视觉密度

- 使用紧凑但可读的行：桌面工作台视图 40-48px 行高。
- 除非现有系统不同，否则使用 13-14px 表格文本和 15-16px 表单/控件文本。
- 在使用厚重阴影之前使用边框和微妙的表面对比。
- 将强色保留给主要操作、活动导航、焦点和语义状态。
- 仅当 KPI 卡片支持页面任务时才允许；避免装饰性主视觉指标。

## 生产标准

- 首个视口必须包含工作中的导航、当前页面上下文和至少一个真实工作区域，如表格/列表/表单/详情/操作面板。
- 应用标识在侧边栏或顶栏中保持紧凑。不要在工作表面上方添加大型品牌介绍块。
- 页眉/页脚文本必须是操作性的：筛选器、状态、用户/工作区、主要操作、分页或帮助路由。长功能描述不是重复使用控制台的一部分。
- 移动回退必须保持相同工作流可达：先列表/搜索，然后通过抽屉、卡片或路由进行详情/操作。
- 证据应指明管理外壳、数据/列表界面和变更/详情界面（当它们被涉及时）。

## 避免

- 主视觉部分、营销页脚、功能说明卡片、装饰性指标或长实现说明。
- 让用户丢失列表上下文的仅模态工作流。
- 没有溢出、分页或响应式回退的表格。

## Quality Gate Index

| Gate | 通过信号 | 失败信号 |
| --- | --- | --- |
| `admin.shell.work_surface` | 首个视口包含紧凑应用标识、导航/当前上下文、真实的表格/列表/表单/详情/操作区域和主要业务操作访问。 | 页面以主视觉文案、大型介绍、类页脚说明、装饰性指标或没有立即可用的工作表面开场。 |
| `admin.topbar.context_actions` | 顶栏/页眉承载操作上下文，如当前页面、搜索/筛选器、用户/工作区或相关命令。 | 页眉是填充文案、产品说明或与活动任务脱节。 |
| `admin.list.filter_table_detail` | 记录屏幕在筛选器、分页、选择、详情和变更反馈中保留列表上下文。 | 选择或变更丢失上下文、表格缺乏状态处理，或每个操作隔离在通用模态中而无行/详情回读。 |

## 筛选、选择和变更连续性

将列表、详情和变更视为一个工作流。筛选器或选定记录是用户工作上下文的一部分，必须在下一次操作中保留。

```text
filter state -> result list -> selected record -> detail/action -> updated row/detail
```

- 将筛选、排序、页和搜索状态保留在拥有列表的路由或屏幕状态中；不要在详情抽屉打开时重置它。
- 在 URL、稳定键或等价可恢复状态中识别选定记录。浏览器刷新不得留下空白详情区域。
- 成功变更后，从返回记录或显式重新获取更新受影响行和详情视图。仅 toast 不是回读。
- 错误后，保留记录、输入和列表上下文。说明用户是否可以修正请求、重试或联系管理员。
- 当选择因筛选或删除而无效时，仅清除无效选择并保留剩余查询上下文。

```tsx
const nextView = {
  query: currentQuery,
  selectedId: updatedRecord.id,
  records: records.map((record) =>
    record.id === updatedRecord.id ? updatedRecord : record,
  ),
};
```

不要将同一记录实现为表格、抽屉和模态中独立的过时副本。共享记录标识并使受影响界面在每次操作后协调。

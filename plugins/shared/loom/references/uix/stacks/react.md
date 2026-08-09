# UIX 技术栈：React

用于 React、Next.js、Remix、Vite React 和类似组件驱动的 React 项目。

## 结构

- 遵循仓库现有的路由器和组件约定。
- 当屏幕有真实工作流复杂度时，保持应用外壳、页面路由、功能组件和可重用 UI 原语分离。
- 业务界面优先使用功能文件夹，可重用原语使用 `components/ui` 或现有设计系统文件夹。
- 将数据获取/变更逻辑靠近路由/页面约定，但避免将每个状态和助手放入一个巨型组件。

## 建议的组件拆分

```text
src/
  app|pages|routes/
    feature-page/
  components/
    layout/
    ui/
    feature-name/
  lib|services/
    api-client
    formatters
  styles/
    tokens
```

对于工作台页面，至少拆分：

- `AppShell` 或路由布局。
- `PageHeader` 或顶栏。
- `FilterBar`。
- `DataTable` 或对象列表。
- `DetailPanel` 或抽屉。
- 用于创建/编辑操作的 `FormPanel`。
- `EmptyState`、`ErrorState` 和 `SkeletonRows`。

## 实现规则

- 通过 CSS 变量、Tailwind 主题、CSS modules、styled system 或仓库现有方法使用语义令牌。
- 显式表示 UI 状态：加载、空、错误、验证、提交、成功、禁用和业务阻塞。
- 使用受控表单状态或仓库中已有的表单库。
- 使用稳定键，仅在有用处进行 memoization，避免显示记录和提交记录之间可能漂移的状态。
- 使用现有图标库；优先使用带标签/工具提示的可访问图标按钮。
- 如果为任务选定了令牌模板，将其适配到仓库现有的全局 CSS、Tailwind 或主题位置。不要将模板声明粘贴到每个组件中。
- 当页面拥有多个工作流区域时，保持路由/页面组件作为编排，并将重复 UI 移入功能/共享组件。

## 状态模式

```tsx
type LoadState<T> =
  | { status: 'loading' }
  | { status: 'error'; message: string }
  | { status: 'empty' }
  | { status: 'ready'; data: T };
```

不要将业务阻塞折叠为通用 `error`。将其保持为在相关操作附近渲染的领域状态。

## React/Next 说明

- Next App Router：保持服务端/客户端边界清晰；仅在需要交互时将组件标记为客户端。
- Vite/SPA：当重用时将 API 客户端和状态助手放在页面组件之外。
- 避免随机值、日期或仅视口渲染导致的水合不匹配。
- 将元数据和运行时/部署说明放在文档或结果中，而非产品 UI。

## 验证

- 可用时运行仓库的聚焦构建/lint/test 命令。
- 渲染页面并在相关视口尺寸检查。
- 检查状态转换不会不必要地重新挂载整个界面。
- 确认证据指名实际消费令牌资产的 React 组件或样式。

## Quality Gate Index

| Gate | 通过信号 | 失败信号 |
| --- | --- | --- |
| `react.split.workflow_regions` | React 页面编排与可重用功能组件、数据/API 模块、格式化器、状态视图和令牌消费样式分离。 | 当工作流有多个区域时页面组件拥有所有获取、表单、表格、模态/抽屉、状态渲染和样式。 |

## 路由和数据边界

UIX 技术栈决定可见页面的组合方式；仓库的 React、Next.js、路由器、数据和 API 参考决定代码如何跨越运行时边界。

```text
route/layout -> page orchestration -> feature view -> shared primitive
                           \-> query/mutation adapter -> state view -> readback
```

- 保持路由/布局负责外壳和导航上下文，页面编排负责任务范围，功能组件负责可见区域和操作。
- 将 API 客户端、查询键、序列化器和变更保留在现有数据边界中。不要从每个展示组件获取数据。
- 对于 Next.js，保留服务端/客户端边界、水合确定性、加载/错误/未找到行为和直接路由刷新。对于 Vite 或仅客户端应用，保留已接受的路由器和同源/API 配置。
- 将稳定的记录标识和显式操作回调传入行/详情/表单组件；不要让过时的全局选择决定变更目标。

## 令牌和状态归属

```tsx
<FeaturePage>
  <PageHeader />
  <FilterBar value={query} onChange={setQuery} />
  <ResultRegion state={resultState} />
  <DetailPanel record={selectedRecord} onAction={handleAction} />
</FeaturePage>
```

每个关注点使用一个应用级令牌源和一个状态所有者。功能可以在其产品界面需要新角色时扩展语义令牌，但它不得创建与共享系统冲突的组件本地颜色、间距或状态约定。

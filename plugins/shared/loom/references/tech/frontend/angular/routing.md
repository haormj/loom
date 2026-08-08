# Angular 路由与导航

仅实现任务所属的导航：路由定义、路由参数、守卫、resolver、深链接、嵌套 outlet、重定向或未保存变更行为。仅组件工作不应接收路由指导。

## 路由所有权

将每个产品界面映射到稳定的路由 segment，保持有效路径与已接受的前端/API 部署 base 对齐。为内聚区域使用功能路由文件和惰性边界，而非一个按 URL 文本切换的 catch-all 组件。

```typescript
export const ORDER_ROUTES: Routes = [
  {
    path: '',
    loadComponent: () => import('./order-list.page').then(m => m.OrderListPage),
    title: 'Orders',
  },
  {
    path: ':orderId',
    loadComponent: () => import('./order-detail.page').then(m => m.OrderDetailPage),
    canActivate: [orderAccessGuard],
    resolve: { order: orderResolver },
  },
];
```

保留仓库的尾斜杠、hash/path 位置、base href、回退和部署 rewrite 约定。浏览器刷新/深链接必须到达 Angular 入口点而不破坏 `/api` 路由。

## 惰性加载与预加载

对初始不需要的大量功能边界使用 `loadComponent`/`loadChildren`。避免为每个叶子创建微小分块，避免击败惰性边界的急切导入。

基于可能的用户流和包成本预加载。不要在没有产品/运行时理由的情况下应用 `PreloadAllModules` 或教程中的自定义延迟。服务端也保护惰性路由；代码拆分不是授权。

## 参数与 URL 状态

在 API/store 操作之前验证路径/查询值。缺失、格式错误、未授权和未找到的标识符需要显式结果。

仅当应用已选择 `withComponentInputBinding` 且输入名称/类型与参数/解析数据对齐时使用它。否则使用 `paramMap`/`queryParamMap` 配以正确的生命周期清理或 signal interop。

筛选、排序、分页、选定标签和返回上下文在必须经受刷新/分享/返回导航时属于查询参数。有意识地保留或替换查询值；避免过期筛选意外合并到不相关界面。

不要将密钥、完整草稿或敏感个人数据放在 URL 状态中。导航 `extras.state` 是临时的，不应是可刷新路由的唯一来源。

## 守卫与授权

兼容版本上的函数守卫可以使用 `inject()`，应返回 `boolean`、`UrlTree` 或 observable/promise 等价物。返回 `UrlTree` 进行重定向而非命令式 `navigate` 加 false。

守卫改善导航体验；它们不是服务端授权。区分未认证登录重定向、禁止界面、无效状态和未找到行为。

将未保存变更守卫绑定到显式的脏草稿契约。当设计系统提供时优先使用产品确认对话框而非原始 `window.confirm`，覆盖浏览器/返回/关闭导航路径。

## Resolver 与加载策略

仅在路由激活前需要的数据使用 resolver。长或易失败的数据可以改为渲染路由级加载/错误状态。不要让每个页面等待不相关的仪表板请求。

Resolver 必须将错误映射到已接受的路由结果，并在导航变更时支持取消。为每次失败返回 `null` 会擦除未找到、禁止和不可用之间的区别。

保持 resolver 加载的数据、组件重新加载和 store 缓存一致；避免三层重复请求。

## 嵌套路由、Outlet 与标题

当产品层级和保留上下文需要时使用子路由/outlet。命名 outlet 增加 URL 和心智复杂性；为独立可导航面板使用它们，而非普通页面布局。

从业务上下文设置路由标题/元数据，不泄漏内部 ID 或过期解析值。从路由配置/状态派生面包屑和导航选择，而非重复的路径字符串检查。

## 导航生命周期

在显示全局进度时处理 `NavigationCancel` 和 `NavigationError` 以及开始/结束。用 `takeUntilDestroyed` 或 signal interop 清理路由事件订阅。

为列表-详情-返回流保留滚动/焦点/恢复行为。导航后，按可访问性/产品行为将焦点放在新页面上下文或恢复的控件处。

## Verification

- 测试精确路由匹配、重定向、惰性导入、参数/查询解析和通配/未找到行为。
- 练习守卫允许/重定向/禁止和 resolver 成功/未找到/禁止/不可用分支。
- 验证变更公共路由的直接深链接刷新和部署回退。
- 通过前进、后退、刷新和编程导航确认筛选/标签/分页/返回上下文。
- 在拥有时测试脏草稿导航和焦点/滚动恢复。
- 构建路由配置以捕获循环/缺失的 standalone 导入。

## 交付证据

标识有效 URL、路由所有者和证明激活及相关守卫/resolver/查询行为的 RouterTestingHarness 或浏览器断言。仅路由对象或直接守卫调用不能证明惰性加载、重定向、导航取消、深链接或部署回退。

## 不安全默认行为

- 从描述而非导航所有权选择路由参考。
- 将守卫视为服务端授权。
- 守卫内部命令式导航而非返回 `UrlTree`。
- Resolver 失败折叠为 null/主页重定向。
- 当需要 URL 持久化时筛选/草稿在隐藏组件状态中重复。
- 无工作流理由添加命名 outlet 和预加载。
- 仅通过应用内点击测试深链接。

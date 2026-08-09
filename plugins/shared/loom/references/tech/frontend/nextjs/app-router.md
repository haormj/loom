# Next.js App Router

仅当 TechnicalBaseline 选择 App Router 且任务拥有路由定义、segment、布局、导航、边界、route handler 或元数据时应用此参考。通用 Next.js 和仅组件任务不得接收它。

## Segment 所有权

将产品界面映射到清晰的 segment。使用 route group 进行代码/布局组织而不改变 URL，而非隐藏不相关的工作流。

```text
app/
  (workspace)/
    layout.tsx
    orders/
      page.tsx
      loading.tsx
      error.tsx
      [orderId]/
        page.tsx
        not-found.tsx
```

根布局拥有所需的文档壳/provider。嵌套布局在子导航间持久化；`template.tsx` 重新挂载，仅在需要导航时重置时使用。

按仓库约定将路由特定的 UI/数据保留在 segment 附近，将共享产品组件保留在路由文件夹之外。避免布局和功能模块之间的循环导入。

## 动态、Catch-All 与搜索参数

在数据/操作之前对 params/search params 进行类型化和验证。考虑所选 Next 版本的 async params API。缺失/格式错误/未授权/未找到状态需要显式结果。

仅对真实的层级内容使用 catch-all/optional catch-all。不要创建吞噬静态路径或内部资源的宽泛动态路由。

Query/search params 适用于可共享的筛选、排序、分页、标签和返回上下文。在服务端解析/白名单它们，并在导航时有意识地保留/清除。

## 加载、错误与未找到

在异步 segment 边界处放置 `loading.tsx`，使回退有帮助；避免为一个小慢区域替换完整的工作壳。对可独立流式传输的区域使用 Suspense。

`error.tsx` 是 Client Component，拥有意外片段失败以及重置/重试。它应通过所选边界记录，不暴露堆栈/provider 数据。

对真正缺失的记录调用 `notFound()`/提供 `not-found.tsx`。不要将禁止/不可用/验证结果转换为未找到，除非披露契约如此要求。

预期的业务错误应渲染任务所属的状态，而非抛入通用错误边界。

## Parallel 与 Intercepting 路由

对独立可导航/可渲染的 slot 使用 parallel 路由，配以显式默认值和刷新行为。对产品批准的模态/详情导航（同时有直接整页 URL）使用 intercepting 路由。

这些模式增加返回栈、默认值、刷新和可访问性复杂性。提供关闭/返回/焦点行为和直接深链接回退；不要将它们用于普通布局列。

## 导航与重定向

对可导航目标使用链接，对事件驱动转换使用 router 方法。为详情-返回工作流保留列表/筛选/滚动上下文。

对服务端已知结果使用服务端 `redirect`，仅在交互边界使用客户端 router 导航。避免在闪烁受保护/错误内容后重定向的挂载 effect。

定义中间件/auth 重定向而无循环，保留安全的目标目的地。导航检查不替代服务端授权。

## Route Handler

Route handler 是服务端 HTTP 接口。仅实现已接受的方法、路径、schema、状态、错误、授权和暴露行为。验证输入，限定标识/租户，映射失败，并将持久化/业务逻辑保留在已接受的应用边界中。

避免仅为代理现有同源后端而创建 route handler，除非架构需要 BFF。有意识地保留 cookie、流式传输、缓存、header 和体限制。

## 元数据

对稳定页面使用静态元数据，对动态已接受内容使用 `generateMetadata`。安全地去重数据读取，仅在产品/SEO 拥有时提供 canonical/OpenGraph/robots。

绝不在元数据中暴露私有记录详情、内部 ID 或失败的查找消息。元数据失败需要有界行为。

## Verification

- 为变更的路由文件契约和服务端/客户端边界运行生产构建。
- 练习精确的静态/动态/catch-all/query 路径、重定向和未找到/禁止结果。
- 验证加载流式传输、错误重置和预期业务状态放置。
- 测试布局持久化与 template 重新挂载以及列表-详情-返回上下文。
- 在拥有时练习 parallel/intercepting 直接刷新、返回/关闭和焦点。
- 对 route handler，断言精确的 HTTP 契约和 auth/失败分支。

## 交付证据

命名 segment/有效 URL 以及证明激活、持久化、边界、重定向、元数据或 handler 行为的构建/路由/浏览器断言。仅文件存在不能证明匹配优先级、深链接刷新、流式传输、返回栈或部署回退。

## 不安全默认行为

- 在无已接受 App Router 信号和导航任务时选择 App Router 参考。
- 用 route group/dynamic catch-all 隐藏不清晰的所有权。
- 为一个慢区域使用全壳加载回退。
- 将每次失败转换为 `notFound()` 或通用 `error.tsx`。
- 将 parallel/intercepting 路由用于普通页面布局。
- Route handler 重复已接受的后端契约。

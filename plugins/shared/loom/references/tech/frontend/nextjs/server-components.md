# Next.js React Server Components

仅对拥有服务端渲染组件组合、服务端/客户端边界、流式传输、hydration 或可序列化交接的 App Router 任务应用此参考。不要仅因选择了 Next.js 就附加它。

## 边界选择

为服务端拥有的读取、静态/派生标记、密钥/内部访问和减少浏览器包使用 Server Components。为事件处理器、本地 state/effect、ref、浏览器 API、客户端 store 和仅浏览器库使用 Client Components。

将 `'use client'` 下推到最小内聚岛屿。Client Component 边界使其导入子树变为客户端，因此不要在其下方导入服务端模块。

```tsx
// Server Component
export async function OrderPanel({ orderId }: { orderId: string }) {
  const order = await loadAuthorizedOrder(orderId)
  return <OrderActions initialOrder={toSerializableOrder(order)} />
}

// Client island
'use client'
export function OrderActions({ initialOrder }: { initialOrder: OrderViewModel }) {
  // interactive state and events only
}
```

## 可序列化交接

传递 JSON/React 可序列化的安全值。不要传递函数、类/ORM 实例、数据库 client、Request/Response 对象、密钥、未序列化的 Decimal/BigInt 或可变服务端句柄。

按已接受的客户端契约规范化日期、小数、bigint、枚举、URL、Map/Set 和错误。保持载荷最小，避免因客户端组件不显示而发送隐藏/内部字段。

仅通过其支持的引用/表单调用语义使用 Server Action；它们不是任意的回调 props。

## 数据与组合

在避免瀑布/重复同时保留所有权的最深共享服务端边界处获取数据。并行化独立读取并有意识地使用请求范围记忆化/缓存语义。

不要仅为获取数据而将组件转为客户端。当需要浏览器拥有的刷新/轮询/乐观行为时，用安全初始状态种子客户端数据边界并定义新鲜度/协调。

当客户端包装器不需要检查/克隆时，通过 Client Component 组合 Server Component 子项。Provider 需要客户端包装器；保持它尽可能窄，不要将根布局变为宽泛客户端边界。

## 流式传输与 Suspense

在独立有用的慢服务端区域周围使用 Suspense，配以稳定的回退尺寸和附近的错误归属。避免一个阻塞整个工作台的巨大边界，也避免数十个闪烁的微边界。

流式传输必须保留认证/数据限定，不在访问已知之前暴露私有壳/元数据。衡量早期内容是否改善用户流。

## 仅浏览器库

将仅浏览器的第三方组件包装在聚焦的 Client Component 适配器中，在适当时惰性加载。确保 SSR 禁用输出有有意义的加载/回退，不导致布局偏移或空白主要工作。

不要将 `dynamic(..., { ssr: false })` 用作通用 hydration 修复。纠正确定性的服务端/客户端输出或隔离实际的浏览器依赖。

## Hydration 确定性

在服务端/客户端初始渲染期间避免时间/随机性、浏览器存储、视口检查、区域设置/时区差异、生成 ID 和不稳定对象排序。

传递服务端派生的稳定值，将仅浏览器读取延迟到带稳定占位符的 effect，或在可能时使用 CSS/响应式渲染。`suppressHydrationWarning` 是窄逃生舱，不是不匹配子树的解决方案。

## 安全与失败

Server Components 通过框架请求可达，必须在读取数据处执行 auth/租户/所有权。客户端隐藏不是保护。

将预期的缺失/禁止/业务/不可用结果映射到已接受的路由/界面状态。意外失败到达 segment 边界和关联感知的记录，不泄漏内部信息。

## Verification

- 运行生产构建以捕获服务端/客户端导入和序列化失败。
- 检查客户端包排除仅服务端模块/密钥/provider。
- 练习初始数据交接和交互岛屿行为。
- 在变更处测试日期/小数/bigint/错误序列化。
- 验证流式回退、有用的早期内容、错误和 auth 限定。
- 在拥有时在真实浏览器中复现 hydration 敏感的区域设置/时间/存储/视口路径。

## 交付证据

标识服务端/客户端边界、载荷以及证明包隔离、序列化、流式传输或 hydration 的构建/浏览器断言。`'use client'` 标记或开发渲染成功不能证明生产导入图、密钥隔离、确定性 hydration 或运行时兼容性。

## 不安全默认行为

- 在无 App Router 和显式边界所有权时选择 Server Component 参考。
- 为一个 hook/控件将整个页面/布局转为 Client Component。
- 向客户端传递 ORM/类/密钥/不可序列化 provider 值。
- 将 `ssr: false` 或 hydration 抑制用作通用修复。
- 包围所有服务端内容的根级客户端 provider。
- 暴露或移位整个产品壳的流式边界。

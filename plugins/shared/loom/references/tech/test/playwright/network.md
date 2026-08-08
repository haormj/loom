# Playwright 网络与后端边界

网络控制用于与真实行为同步并创建特定边界条件。它不得擦除浏览器检查被分配证明的后端契约。

## 遵守后端模式

当 profile 声明 `backendMode: real`：

- 通过仓库支持的命令启动或复用实际项目后端；
- 通过现有运行时配置将前端指向该后端；
- 使用真实持久化或项目已接受的测试 provider；
- 通过支持的 fixture、API、迁移或测试 harness 种子数据；
- 断言可见结果，在相关时断言已接受的 API 或持久化效果。

不要用 mock 履行中央创建/更新/读取请求。那将仅证明前端渲染同时声称集成工作流通过。

当后端模式不适用时，不要发明 service。保持仅客户端状态确定且与生产行为对齐。

## 与因果请求同步

在触发请求的操作之前创建响应 promise：

```typescript
const saveResponse = page.waitForResponse(response =>
  response.url().endsWith('/api/cart/items/SKU-1042') &&
  response.request().method() === 'PUT'
);

await page.getByRole('button', { name: 'Update quantity' }).click();
const response = await saveResponse;
expect(response.status()).toBe(200);
await expect(page.getByRole('status')).toContainText('Cart updated');
```

HTTP 200 不是完整结果。断言对用户重要的可见或导航结果。相反，不检查分配的写入请求的可见成功可能遗漏断开连接的乐观 UI。

## Mock 决策

对昂贵、破坏性、外部或难以确定性地触发的条件使用请求拦截：

- 上游超时或不可用的第三方；
- API 契约已定义的精确验证/业务错误响应；
- 检查加载或重复提交预防所需的延迟响应；
- 已接受架构中无后端的静态客户端应用；
- 在其他地方有真实后端覆盖的前端响应映射测试。

不要 mock：

- 真实后端工作流的主要成功路径；
- 浏览器检查预期集成的请求载荷形状；
- 当登录/session 行为是功能时的认证；
- 交付需要存储状态时的持久化回读；
- 通过宽泛 `**/api/**` 处理器的每个请求，使意外调用不被注意地通过。

## 精确拦截

匹配方法和 URL，验证请求，并使意外变体失败：

```typescript
await page.route('**/api/subscriptions/plan', async route => {
  const request = route.request();
  if (request.method() !== 'PUT') {
    await route.abort('failed');
    return;
  }
  expect(request.postDataJSON()).toEqual({ plan: 'team' });
  await route.fulfill({
    status: 409,
    contentType: 'application/json',
    body: JSON.stringify({ code: 'PLAN_CHANGE_BLOCKED', message: 'Resolve the outstanding invoice first' }),
  });
});
```

将路由限定到一个测试并在 fixture 复用页面时注销长寿命处理器。避免意外拦截资源、健康检查或无关操作的通配处理器。

## 加载与并发状态

当测试必须持有请求时使用受控 promise 而非固定延迟：

```typescript
let release!: () => void;
const gate = new Promise<void>(resolve => { release = resolve; });

await page.route('**/api/catalog', async route => {
  await gate;
  await route.continue();
});

await page.goto('/catalog');
await expect(page.getByRole('status').filter({ hasText: 'Loading products' })).toBeVisible();
release();
await expect(page.getByRole('list', { name: 'Products' })).toBeVisible();
```

对于提交操作，验证控件在请求 pending 时禁用或以其他方式阻止重复提交。

## 响应修改与 HAR

`route.fetch()` 可在为窄客户端条件更改一个字段时保留真实响应。记录修改了什么以及为什么；永远不要在旨在验证未触及后端响应的检查上使用它。

HAR 回放适用于有审查过的 fixture 的稳定外部依赖。保持 HAR 文件不含密钥和个人数据。不要在普通验证运行中启用自动 HAR 更新，因为它可能静默地批准变更的响应。

## WebSocket、SSE、轮询与后台刷新

- 等待可见事件或应用状态，而非 `networkidle`。
- 在确定性排序重要时通过现有测试支持控制时钟或事件源。
- 仅在分配给任务时断言重新连接、过期数据或离线反馈。
- 确保轮询和 socket 随页面/context 关闭，不在验证后保持命令活跃。

## 失败分类

- 连接被拒绝、缺失凭据、不可用 registry/service 和无法启动的依赖是环境阻止项。
- 错误的请求方法、载荷、路径、响应映射或缺失反馈是产品/集成缺陷。
- 不再匹配已接受 API 的 mock 是测试缺陷；仅在比较当前契约后更新它。

记录简洁的请求标识、状态和用户可见结果。将完整体、头、trace 和密钥排除在交付结果摘要之外。

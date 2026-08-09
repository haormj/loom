# Playwright Fixture 与可复用模型

Fixture 应使状态所有权显式并保持测试可读。当复用移除重复设置或捕获稳定的产品交互时是有价值的；当抽象隐藏检查必须证明的行为时是有害的。

## 选择最小复用单元

当设置仅在一个文件本地时使用普通 helper。当多个测试需要相同的生命周期管理依赖时使用 fixture。当稳定的产品界面有多个重复交互时使用页面或组件对象。

不要为单个短测试创建 Page Object Model 目录。不要将断言、测试数据工厂、API client、导航和每个页面交互放入一个基类中。

| 需求 | 首选形式 |
| --- | --- |
| 一个测试创建一条记录 | local helper |
| 多个测试需要隔离记录 | worker/test fixture or typed factory |
| 与稳定界面的重复交互 | page/component model |
| 共享认证角色 | storage-state project or role fixture |
| 全局可变数据库状态 | avoid; use namespaced data and controlled reset |

## Fixture 生命周期

- 测试范围的 fixture 拥有可变 page、context、记录和临时文件状态。
- Worker 范围的 fixture 可拥有昂贵的不可变 service 或唯一 worker 命名空间。
- 拆卸必须在部分设置后安全且不得删除另一个 worker 的记录。
- Fixture 应暴露有意义的领域能力，而非原始的不相关值袋。
- 保持 fixture 依赖无环且在其参数列表中可见。

```typescript
import { test as base, expect, type APIRequestContext } from '@playwright/test';

type WorkspaceRecord = { id: string; slug: string };
type Fixtures = {
  workspaceRecord: WorkspaceRecord;
};

export const test = base.extend<Fixtures>({
  workspaceRecord: async ({ request }, use, testInfo) => {
    const externalKey = `pw-${testInfo.workerIndex}-${testInfo.retry}-${Date.now()}`;
    const response = await request.post('/api/test-support/workspaces', {
      data: { externalKey, name: 'Northwind Studio' },
    });
    expect(response.ok()).toBeTruthy();
    const record = await response.json() as WorkspaceRecord;
    await use(record);
    await request.delete(`/api/test-support/workspaces/${record.id}`);
  },
});

export { expect } from '@playwright/test';
```

仅在项目已允许时使用现有测试支持 API。不要在生产代码中发布不安全的重置或种子端点来使浏览器测试方便。

## 认证

- 优先使用项目支持的测试身份、本地 auth 绕过或受控登录 fixture。
- 将生成的 auth 状态存储在忽略的测试输出下，永远不在源控制中。
- 将角色分离到命名项目或 fixture 中，使测试不能意外继承管理员权限。
- 在过期重要时刷新状态；不要用永久有效的令牌掩盖过期行为。
- 对于登录工作流任务，测试真实登录交互而非预加载存储状态。

```typescript
import { test as setup, expect } from '@playwright/test';

const reviewerAuthFile = 'playwright/.auth/reviewer.json';

setup('authenticate as reviewer', async ({ page }) => {
  await page.goto('/login');
  await page.getByLabel('Email').fill(process.env.E2E_REVIEWER_EMAIL!);
  await page.getByLabel('Password').fill(process.env.E2E_REVIEWER_PASSWORD!);
  await page.getByRole('button', { name: 'Sign in' }).click();
  await expect(page).toHaveURL('/work-queue');
  await page.context().storageState({ path: reviewerAuthFile });
});
```

凭据来自项目环境。缺失凭据是环境阻止项，不是硬编码密钥的理由。

## 页面与组件模型

模型应暴露产品语言和稳定交互：

```typescript
export class WorkspaceSettings {
  constructor(private readonly page: Page) {}

  heading(name: string) {
    return this.page.getByRole('heading', { name });
  }

  async transferOwnership(email: string) {
    await this.page.getByRole('button', { name: 'Transfer ownership' }).click();
    const dialog = this.page.getByRole('dialog', { name: 'Transfer workspace ownership' });
    await dialog.getByLabel('New owner').fill(email);
    await dialog.getByRole('button', { name: 'Confirm transfer' }).click();
  }
}
```

当断言表达场景结果时将它们保留在测试中。模型可暴露状态定位器；它不应决定每次调用都必须断言相同消息。

避免模型继承。当导航、表格、对话框或编辑器交互共享时组合页面级和组件级模型。

## 测试数据

- 使用领域有效的最小记录。巨大的通用 fixture 模糊了哪些字段重要。
- 为可变数据生成唯一键；在断言需要处保留可读的显示值。
- 在拥有验证规则的测试中构建无效输入，而非削弱全局工厂。
- 永远不要依赖生产数据、时钟敏感的现有记录或测试排序。
- 仅通过现有项目支持在时间是被测行为时冻结或注入时间。

## 并行安全

在启用完全并行执行之前，验证：

- 记录键在每个 worker 中唯一；
- auth session 不修改同一用户状态；
- 文件下载使用 `testInfo.outputPath()`；
- 清理目标由相同 fixture 创建的 ID；
- 共享速率限制、队列和后台作业受控；
- 测试不复用单例 page 或浏览器 context。

串行模式对于真正有序的工作流是可接受的，但它应限定到该组并由产品依赖文档化。不要使整个套件串行来隐藏状态泄漏。

Fixture 设置失败应说明哪个先决条件失败。不要捕获并用通用"setup failed"消息替换所有错误。当它们有助于诊断失败时将 ID 或安全响应摘要附加到测试注解，但不要输出密钥或完整载荷转储。

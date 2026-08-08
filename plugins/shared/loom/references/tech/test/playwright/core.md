# Playwright 交付核心

使用 MCP 派生的浏览器验证 profile 作为范围权威。此参考解释如何将其检查转化为可维护的浏览器自动化，而不扩大任务或替换项目的测试栈。

## Loom 浏览器质量闭环

浏览器检查在实现和运行时交付之后的 MCP 生成的浏览器质量闭环中运行。普通 UI 任务实现并静态/组件测试其所属界面；它们不安装浏览器或声称渲染证据。

- `sourceTaskId` 和 `sourceVerificationId` 保留每个闭环检查所证明的业务任务。
- `required` 证据必须通过、使用已接受的外部证据，或在自动交付闭环之前获得显式质量豁免。
- `supplemental` 证据提高信心，但环境差距不会将已完成的产品代码变为失败任务。
- 仅运行闭环 profile 中的检查。不要从 UI 描述重新创建检查、扫描其他任务结果寻找额外范围，或添加宽泛的回归套件。

## 验证设计

从检查必须证明的行为开始：

1. 标识分配的 `verificationId`、视口、后端模式、任务所属工作流和 UI 界面。
2. 选择到达所需结果的最短用户可观察路径。
3. 仅安排该路径所需的状态。
4. 通过可见控件或声明的浏览器导航执行操作。
5. 断言业务结果、本地 UI 状态和相关的持久化或 API 效果。

浏览器检查不是应用导览。不要添加不相关的导航、完整回归覆盖或每个断点，仅因为 Playwright 能到达它们。

## 测试层边界

将 Playwright 用于需要浏览器边界的行为：

- 路由进入、导航、历史、刷新、深链接和浏览器存储；
- 跨组件、provider 和 API 调用的用户工作流；
- 渲染的响应式布局和视口特定交互；
- 键盘/焦点行为和浏览器级语义；
- 集成反馈，如提交、成功、验证和业务阻止。

将纯函数、reducer、composable、hook、隔离的组件状态和仅服务端规则保留在其现有的单元或集成测试层。不要将廉价的确定性检查移入浏览器套件。

## 项目适配

- 在存在时复用仓库的包管理器、Playwright 配置、测试根、脚本、fixture 和命名约定。
- 遵循请求选择的项目 runner 和共享运行时契约；此参考不覆盖 runner 选择或依赖所有权。
- 当不存在 Playwright 项目且任务拥有套件设置时，创建支持分配检查的最小配置和测试根。
- 不要在已接受的现有浏览器测试栈旁安装第二个 E2E runner，除非技术基线为新项目选择了 Playwright。
- 不要从闭环任务内部准备浏览器运行时。交付运行时在执行请求之前准备并附加到任务范围。

## 检查结构

使用陈述行为和结果的测试名称：

```typescript
test('saved profile name survives a browser reload', async ({ page }) => {
  await page.goto('/account/profile');
  await page.getByLabel('Display name').fill('River Team');
  await page.getByRole('button', { name: 'Save changes' }).click();

  await expect(page.getByRole('status')).toContainText('Profile updated');
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Account profile' })).toBeVisible();
  await expect(page.getByLabel('Display name')).toHaveValue('River Team');
});
```

示例断言可见状态转换。它不断言内部组件名称、CSS 类或实现状态。

## 后端模式

- `real`：针对真实项目后端和持久化边界运行分配的工作流。通过现有 fixture/API 路径种子受控数据。不要用 `page.route()` mock 替换中央成功路径。
- `not_applicable`：保持检查渲染且确定，不发明后端。静态或仅客户端界面仅在匹配生产行为时使用本地 fixture 数据。

网络拦截可控制特定失败或时序条件。它不得静默地将真实后端检查转换为 mock 的组件演示。

## 状态与隔离

- 每个测试创建或标识自己的记录；永远不要依赖另一个测试的执行顺序。
- 为可变记录使用稳定唯一值，并在清理重要时通过支持的项目 fixture 清理它们。
- 按角色和环境限定认证状态。不要提交凭据或生成的 auth 状态。
- 仅在状态所有权隔离后并行化。当测试修改共享记录时 `fullyParallel: true` 不是质量信号。
- 避免隐藏先决条件。失败的设置必须标识缺失的 service、凭据、种子或路由。

## 断言

优先使用针对可观察状态重试的 web-first 断言：

```typescript
await expect(page.getByRole('button', { name: 'Save' })).toBeEnabled();
await expect(page.getByRole('status')).toHaveText('Saved');
await expect(page).toHaveURL(/\/account\/profile$/);
```

不要使用 `waitForTimeout()` 作为同步。不要在有轮询、分析、socket 或后台刷新的应用中将 `networkidle` 用作通用就绪条件。等待实际门控下一步操作的响应、URL、元素状态或业务结果。

## 产物纪律

- 将 trace、截图、视频和报告保留在项目配置的输出目录中。
- 按配置在失败或重试时保留诊断产物；将其内容排除在简洁验证摘要之外。
- 重试成功仍然是重试成功。保留尝试计数并调查重复不稳定性，而非报告干净的首次通过结果。
- 如果环境准备失败，记录具体的缺失依赖或 service。不要将环境失败重新标记为产品缺陷。

检查仅在分配的视口和后端模式运行、观察到预期业务结果、调用返回控制、产物被引用而非嵌入、且无临时服务端或浏览器进程未被管理时才算完成。

# Playwright 浏览器可访问性验证

浏览器可访问性检查证明任务所属的语义和交互契约。自动化扫描是有用的支持证据；它们不替代键盘、焦点、名称、角色、状态和动态反馈检查。

## 语义入口检查

验证用户和辅助技术可以识别界面：

- 页面标题和主标题描述当前上下文；
- landmark 在适当时区分导航、主内容、互补区域和 banner；
- 控件暴露正确的角色、可访问名称、值、选中/展开/选定状态和禁用状态；
- 表单字段有持久标签和关联帮助/错误；
- 表格暴露理解行所需的头和关系；
- 仅图标控件有可访问名称和可见工具提示（当含义不通用时）。

```typescript
await expect(page).toHaveTitle(/Account settings/);
await expect(page.getByRole('main')).toBeVisible();
await expect(page.getByRole('heading', { level: 1, name: 'Account settings' })).toBeVisible();
await expect(page.getByRole('button', { name: 'Save changes' })).toBeEnabled();
```

能够通过角色定位控件也是可测试性信号。不要添加 test id 来避免纠正缺失名称或错误元素类型。

## 键盘工作流

对于任务所属操作，验证：

- 焦点顺序遵循视觉/任务顺序；
- 所有操作可在无指针的情况下到达和激活；
- 可见焦点不被裁剪或遮挡；
- 对话框在打开时陷阱焦点，通过支持的键关闭，并将焦点返回触发器；
- 菜单、标签、组合框、网格和展开控件使用其预期的键行为；
- 粘性头、sheet 和覆盖层不覆盖聚焦元素。

```typescript
await page.getByRole('button', { name: 'Invite member' }).focus();
await page.keyboard.press('Enter');
const dialog = page.getByRole('dialog', { name: 'Invite workspace member' });
await expect(dialog).toBeVisible();
await expect(dialog.getByLabel('Email address')).toBeFocused();
await page.keyboard.press('Escape');
await expect(page.getByRole('button', { name: 'Invite member' })).toBeFocused();
```

在真实工作流上使用 `press()` 和焦点断言。不要通过 JavaScript 调用点击处理器来模拟键盘支持。

## 表单与错误

- 无效提交将焦点移动到第一个无效字段或清晰的错误摘要（按产品约定）。
- 字段错误与其控件关联且保持可见足够长以采取行动。
- 提交状态通过语义传达，不仅通过颜色或动画。
- 保留的输入在服务端/业务失败后保持可用。
- 必需、只读、禁用和无效状态匹配实际行为。

```typescript
await page.getByRole('button', { name: 'Create account' }).click();
const email = page.getByLabel('Email address');
await expect(email).toBeFocused();
await expect(email).toHaveAttribute('aria-invalid', 'true');
await expect(page.getByText('Enter a valid email address')).toBeVisible();
```

## 动态反馈

加载、成功、验证、错误和业务阻止反馈应在不意外移动焦点的情况下可发现。

- 使用适合紧迫性的 status/alert/live-region 行为。
- 避免宣告每次按键或后台刷新。
- 列表更新时保留用户位置。
- 对于破坏性确认，使操作和后果显式。
- 当禁用操作可能变为可用时，通过附近内容或描述暴露原因。

## 导航与上下文

- 路由变更更新标题、头和当前导航状态。
- 当重复壳使跳过链接必要时，跳过链接或等效导航可用。
- 浏览器返回/前进和深链接入口为分配的工作流保留有意义的焦点/上下文。
- 抽屉和客户端路由转换不在已移除元素上留下焦点。

## 媒体、图表、Canvas 与 3D

验证任务所属的替代方案：alt 文本、字幕/转录、可访问名称、键盘控件、图表摘要或等效数据访问。Canvas 截图可证明渲染但不能证明可访问性。不要在仅检查了像素时声称语义覆盖。

- 检查媒体控件暴露名称、状态和键盘操作。
- 检查图表传达标题、系列/图例含义和产品所需的文本或数据替代。
- 检查 canvas/3D 控件不陷阱键盘焦点且基本操作有非指针访问。
- 将不受支持的辅助行为记录为差距，而非将图像比较视为语义证明。

## 自动化扫描

在可用时使用仓库现有的可访问性扫描器。将扫描限定到任务所属的路由或区域，并在相关状态渲染后运行。除非技术基线选择，否则不要为一个检查安装另一个扫描器。

自动化扫描不能证明：

- 合理的焦点顺序；
- 业务上下文中正确的可访问名称；
- 完整的键盘工作流；
- 有用的错误语言；
- 屏幕阅读器宣告时序；
- 对复杂可视化的等效访问。

将扫描违规视为待检查发现，而非待抑制计数。任何排除必须命名第三方/遗留边界和剩余风险。

## 视口与缩放

当分配响应式可访问性时，检查窄布局和放大内容行为。控件和文本必须保持可达而无需二维滚动（对于普通应用内容），除非产品本质上需要大型画布/表格。

验证粘性控件不覆盖聚焦内容、文本无裁剪地重排、触摸目标保持独立且缩放不移除主要操作的唯一路径。将例外限定到实际的大格式界面。

## 证据

记录检查的语义定位器或交互、键盘/焦点结果、动态状态、视口和使用的自动化扫描结果。不要仅基于零扫描器违规报告"可访问"。

对于未解决的问题，标识受影响的控件/工作流、输入方法、预期行为、观察到的行为以及原因是产品代码、第三方内容还是环境限制。

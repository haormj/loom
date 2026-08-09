# Playwright 定位器与断言

定位器是 UI 契约的一部分。它们应描述用户或辅助技术可以识别的内容，经受视觉重构，并在产品变得模糊时清晰失败。

## 定位器优先级

除非现有项目有更严格的约定，否则使用此顺序：

1. 带可访问名称的 `getByRole()`。
2. 用于表单控件的 `getByLabel()`。
3. 仅当占位文本是真正稳定线索时的 `getByPlaceholder()`。
4. 用于稳定业务文案或状态文本的 `getByText()`。
5. 用于非语义可视化、虚拟化内容或稳定集成锚点的 `getByTestId()`。
6. 仅在无法暴露更好契约的第三方或遗留边界处的 CSS 或 XPath。

```typescript
const save = page.getByRole('button', { name: 'Save changes' });
const amount = page.getByLabel('Approved amount');
const status = page.getByRole('status');
```

如果语义定位器无法找到普通按钮、字段、链接、标题、对话框、表格或警报，首先检查产品语义。添加 test id 不得隐藏可访问性缺陷。

## 先限定范围再定位

通过缩小到有意义的区域或记录来解决歧义：

```typescript
const members = page.getByRole('region', { name: 'Workspace members' });
const row = members.getByRole('row', { name: /jamie@example\.com/ });
await row.getByRole('button', { name: 'Edit role' }).click();

const dialog = page.getByRole('dialog', { name: 'Edit member role' });
await dialog.getByLabel('Role').selectOption('editor');
await dialog.getByRole('button', { name: 'Save role' }).click();
```

对重复记录使用 `filter({ hasText })` 或 `filter({ has })`。行 ID、稳定业务键或命名区域比 `.first()` 或 `.nth()` 更强。

## 精确性规则

- 当附近标签有意共享词汇且精确产品标签稳定时使用 `{ exact: true }`。
- 对生成标识符或本地化后缀使用窄正则表达式，而非可能匹配不相关内容的宽泛不区分大小写表达式。
- 不要绑定到时间戳、随机 ID、翻译文案或生成的类名，除非该值是被测行为。
- 当记录可能碰撞时优先使用稳定业务标识符而非示例人名。

```typescript
await expect(page.getByText('Approved', { exact: true })).toBeVisible();
await expect(page.getByRole('heading', { name: /^Order ORD-\d+$/ })).toBeVisible();
```

## 表单

- 可见标签应解析关联控件。
- 将重复字段标签限定到其表单、fieldset、行或对话框。
- 在字段附近或通过其 alert/status 关系断言验证。
- 使用真实输入方法：`fill`、`press`、`selectOption`、`check` 和 `setInputFiles`。
- 不要用 `evaluate()` 修改 DOM 值来绕过真实控件契约。

```typescript
const form = page.getByRole('form', { name: 'Create workspace' });
await form.getByLabel('Workspace name').fill('Northwind Studio');
await form.getByLabel('Region').selectOption('eu-west');
await form.getByRole('button', { name: 'Create workspace' }).click();
await expect(form.getByRole('alert')).toContainText('Plan is required');
```

## 表格、列表与虚拟化数据

- 对于语义表格，按其可访问行名定位行，然后定位其中的单元格/操作。
- 对于卡片/列表布局，按业务键定位列表项并将操作限定到该项。
- 对于虚拟化数据，通过组件的公共行为滚动。当行不以语义表示时，虚拟化视口上的 test id 是可接受的。
- 永远不要在重复内容中点击未限定的 `Edit`、`Delete` 或溢出按钮。

## 菜单、对话框、抽屉与 Portal

Portal 内容可能不是其触发器的 DOM 后代。按语义覆盖角色和可访问名称限定，而非父 CSS：

```typescript
await page.getByRole('button', { name: 'More actions' }).click();
await page.getByRole('menu').getByRole('menuitem', { name: 'Archive' }).click();
await expect(page.getByRole('dialog', { name: 'Archive workspace' })).toBeVisible();
```

当焦点进入模态内容并返回触发器行为为任务所属时验证它。

## Web-First 断言

使用定位器断言而非一次性值读取：

```typescript
await expect(locator).toBeVisible();
await expect(locator).toBeEnabled();
await expect(locator).toHaveText('Ready');
await expect(locator).toHaveAttribute('aria-current', 'page');
await expect(page).toHaveURL(/status=approved/);
```

`textContent()`、`isVisible()` 和原始元素句柄返回快照，可能与渲染竞争。仅当原始值本身必须被转换或比较时使用它们。

## Test ID 策略

test id 适用于：

- 无原生语义节点的 canvas/WebGL 根和图表系列；
- 虚拟化容器或拖拽句柄；
- 稳定的跨团队集成锚点；
- 可见标签动态但语义无法唯一化的控件。

按产品角色命名 ID，而非样式或组件实现：`account-activity-timeline`，不是 `blue-panel-2`；`chart-revenue-series`，不是 `recharts-layer`。

## 不允许的快捷方式

- 定位元素前无任意休眠。
- 在未证明顺序是契约的情况下使用 `.first()`、`.last()` 或 `.nth()` 来消除 strict-mode 歧义。
- 无生成的 CSS 类、深度嵌套选择器或通过布局包装器的 XPath。
- 无 `force: true` 绕过覆盖、禁用或不稳定控件，除非测试显式证明该低级条件。
- 无可能通过隐藏、重复或过期内容的宽泛文本定位器。

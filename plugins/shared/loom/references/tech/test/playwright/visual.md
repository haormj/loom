# Playwright 渲染与视觉验证

渲染验证证明分配的产品界面在所需视口下可见、可用且稳定。它比像素比较更广泛，比主观重新设计审查更窄。

## 两种不同实践

使用渲染检查来检查布局、层级、内容、状态、交互和视口适配。仅当仓库有审查过的基线工作流或任务显式拥有稳定视觉输出时使用视觉回归快照。

截图产物本身不能证明质量。检查必须陈述观察到了什么。

## 视口流程

对于每个 profile 视口：

1. 用确定性数据进入任务所属的路由或工作流。
2. 等待有意义的就绪状态。
3. 当场景需要时确认实际产品界面出现在首视口中。
4. 检查固定头、侧边栏、工具栏、对话框、表格、表单和操作区域是否有重叠或裁剪。
5. 练习分配的主要操作和状态反馈。
6. 检查长标签、代表性数据密度和滚动行为。
7. 仅在状态稳定后捕获截图。

```typescript
await page.setViewportSize({ width: 1440, height: 900 });
await page.goto('/analytics/revenue');
await expect(page.getByRole('heading', { name: 'Revenue analytics' })).toBeVisible();
await expect(page.getByRole('region', { name: 'Revenue trend' })).toBeVisible();
await expect(page.getByRole('button', { name: 'Export report' })).toBeVisible();
await page.screenshot({ path: testInfo.outputPath('revenue-desktop.png'), fullPage: true });
```

## 布局断言

优先使用可观察的产品断言而非硬编码像素坐标。仅对真实几何契约使用测量：

```typescript
const toolbar = page.getByRole('toolbar', { name: 'Report filters' });
const chart = page.getByRole('region', { name: 'Revenue trend' });
await expect(toolbar).toBeVisible();
await expect(chart).toBeVisible();

const toolbarBox = await toolbar.boundingBox();
const chartBox = await chart.boundingBox();
expect(toolbarBox && chartBox && toolbarBox.y + toolbarBox.height <= chartBox.y).toBeTruthy();
```

不要断言每个 padding 值。令牌一致性属于实现和 UI 质量审查；浏览器几何检查针对重叠、遮挡、不可用尺寸和所需的固定关系。

## 移动与响应式行为

- 验证主要操作在无 hover 的情况下保持可达。
- 检查导航转换、数据视图回退、对话框/sheet、粘性操作、键盘安全表单和水平溢出。
- 挤压低于其最小可用宽度的桌面表格是失败，即使没有元素技术上溢出。
- 验证文本有意换行或截断且控件保持足够的目标尺寸。
- 仅当产品契约包含时检查方向或平板布局。

## 视觉快照

对视觉基线已审查的稳定组件/页面使用 `toHaveScreenshot()`：

```typescript
await expect(page.getByRole('region', { name: 'Pricing summary' }))
  .toHaveScreenshot('pricing-summary-annual.png', {
    animations: 'disabled',
    caret: 'hide',
  });
```

基线规则：

- 固定字体、浏览器版本、视口、配色方案、区域设置、时区和确定性数据。
- 仅遮罩真正可变的值；不要遮罩已变更的区域。
- 保持快照范围尽可能小，以视觉契约为限。
- 将基线更新审查为产品变更，而非自动测试修复。
- 在缺陷消失之前不要提高差异容忍度。

## 动态内容

通过已接受的测试支持控制时间戳、随机 ID、动画、轮播、地图、远程图像和实时图表。优先使用确定性 fixture 值而非宽泛遮罩。当字体和关键媒体的渲染影响布局时等待它们。

- 通过现有应用/测试缝隙冻结或注入值；不要在应用加载后修补渲染文本。
- 为捕获禁用动画，同时在功能检查中保留正常交互路径。
- 当这些状态影响几何时保留一个代表性的长/空/错误值。

## Canvas、WebGL、图表与媒体

- 证明 canvas 或媒体有非零尺寸和非空白输出。
- 检查分配的控件或交互改变场景/状态。
- 使用截图或像素采样进行空白渲染检测，而非源存在。
- 如果资产失败时的回退/错误状态在范围内则验证它。
- 在产品需要处为图表/媒体保留语义标签或可访问摘要。

## 状态捕获

捕获证明检查的状态：仅在加载行为被分配时加载；无效操作后验证；带原因的业务阻止；数据刷新后成功。避免产生许多没有陈述目的的截图。

按界面、状态和视口命名产物，使审查者无需打开每个文件即可识别它们。不要为几个实际未渲染的状态复用一个截图引用。

## 环境阻止项

缺失浏览器二进制文件、不可用字体/资源、凭据、不可访问的预览或不受支持的 GPU 可能阻止渲染证据。记录尝试的检查和具体阻止项。源检查可以是回退证据，但不能报告为成功的渲染检查。

MCP 拥有浏览器运行时恢复。主机启动失败首先使用受管容器回退；两个环境都失败作为环境限制带到 Review。不要修改应用 CSS、Playwright 断言或全局超时来补偿从未启动的浏览器。所需的渲染证据然后需要环境重试、完整的外部证据或显式质量豁免。

通过确认所选浏览器修订、字体可用性、设备缩放和配色方案来区分主机特定的渲染差异和产品布局缺陷，然后更改应用样式或快照容忍度。

## 证据摘要

命名视口、路由/工作流、状态、观察到的布局/交互结果和产物引用。将图像二进制排除在描述之外。成功的构建是支持证据；它不替代渲染验证。

对于视觉回归，还标识基线名称以及比较是首次尝试通过、重试后通过还是保持 blocked/failed。

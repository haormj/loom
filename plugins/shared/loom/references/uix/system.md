# UIX 焦点：系统

当创建或修改视觉系统、应用外壳、共享组件、令牌或组件基础时加载此文件。

## 系统基线

- 从现有项目约定开始。扩展它们而非发明并行的设计系统。
- 为颜色、排版、间距、圆角、阴影、焦点、状态和动效定义语义令牌。
- 仅当原语能消除重复实现时才创建：Button、Input、Select、Textarea、Checkbox/Switch、Badge、Alert、Table、Card/Panel、Modal/Dialog、Drawer/Sheet、Tabs、Tooltip、Toast、Pagination、Skeleton。
- 保持组件 props 语义化。优先使用 `variant="danger"` 而非 `color="#ef4444"`。

## 最小令牌集

```css
:root {
  --surface: ...;
  --surface-raised: ...;
  --text: ...;
  --text-muted: ...;
  --border: ...;
  --primary: ...;
  --danger-surface: ...;
  --danger-text: ...;
  --space-4: 16px;
  --radius-md: 8px;
  --focus-ring: ...;
}
```

当仓库已有名称时使用它们。重要的部分是组件消费语义角色而非原始值。

## 令牌资产工作流

1. 在创建文件之前检查现有样式资产：Tailwind 配置、全局 CSS、变量/主题文件、组件库主题、原生主题或设计令牌包。
2. 如果存在兼容资产，就地扩展。如果不存在，为当前实现创建选定的令牌文件。
3. 在应用根目录导入或注册令牌资产一次。不要让每个页面导入自己的令牌文件。
4. 当任务创建共享或重复 UI 时，将页面本地原始值转换为语义别名。一次性原始值仅在资产尺寸或媒体裁剪细节时可接受。
5. 在实现证据中记录令牌资产文件和消费它们的 UI 文件。

## 令牌收敛

当现有屏幕包含多个令牌或原始值变体时：

1. 清点现有主题、全局 CSS、Tailwind 主题、组件库变量和页面本地重复值。
2. 按语义角色分组值，识别哪些变体是场景、状态、暗色模式或平台有意为之的。
3. 为每个有意含义选择一个规范角色，并记录必须保持兼容的别名。
4. 仅替换任务归属的消费者。不要将重写无关屏幕作为样式清理，除非任务拥有共享系统。
5. 验证规范资产注册一次，旧的重复角色已被移除或明确因已知兼容性边界而保留。

令牌模板的存在不证明替换现有主题是合理的。选定的令牌资产计划和仓库样式证据决定实现是重用、扩展还是创建资产。

产品 UI 的有用别名组：

```css
:root {
  --surface: var(--color-surface);
  --surface-raised: var(--color-surface-elevated);
  --surface-muted: var(--color-surface-tinted);
  --text: var(--color-on-surface);
  --text-muted: var(--color-on-surface-muted);
  --border: var(--color-border);
  --border-strong: var(--color-border-strong);
  --focus-ring: 0 0 0 3px color-mix(in oklch, var(--color-primary) 28%, transparent);
  --control-height-sm: 32px;
  --control-height-md: 40px;
  --row-height-compact: 40px;
  --row-height-default: 48px;
  --shell-sidebar-width: 240px;
  --shell-detail-width: 380px;
}
```

## 外壳

- 操作类应用需要稳定的导航、页面标题/上下文、主要操作区域、内容区域和反馈区域。
- 文档需要导航/内容/目录/搜索。
- 营销站点需要产品/优惠信号、证明部分和转化路径。
- 移动/原生界面需要安全区域和平台导航。

## 状态

每个可等待、失败、验证或禁用的可重用组件必须暴露这些状态。避免强制功能代码手工编写不一致的变体。

必需的通用状态：

- default、hover、focus、active、disabled。
- loading/submitting。
- success、warning、danger/error、info。
- 数据界面的 empty 和 skeleton。
- 领域规则停止的 business-blocking。

## Quality Bar

- 组件尺寸在状态变化时保持稳定。
- 焦点环可见且一致。
- 文本和图标干净对齐。
- 令牌可重用并通过名称记录，而非仅靠注释。
- 组件不编码交付流程语言。
- 新的共享组件必须包含其消费者所需的状态；不要让每个功能重新创建禁用/加载/错误变体。
- 令牌变更必须保留现有状态、焦点、密度和平台行为的视觉含义，除非任务明确更改该契约。

## 共享原语基线

对于生产级内部产品，第一轮 UI 通行通常应建立这些原语或重用仓库等价物：

```text
Button / IconButton / Input / Select / Textarea / Checkbox
Badge / StatusPill / Alert / EmptyState / Skeleton
Table / Pagination / Toolbar / Drawer or DetailPanel / Dialog
FormField / FieldError / Toast or InlineNotice
```

在工作流拥有上述控件之前不要创建装饰性原语。缺少字段错误、禁用状态、行状态和详情操作的业务 UI 仍然是演示级别的，即使它看起来有样式。

## Quality Gate Index

| Gate | 通过信号 | 失败信号 |
| --- | --- | --- |
| `token.single_source_consumed` | UI 通过应用样式入口或组件系统消费一个项目令牌/主题源，且证据引用了令牌资产文件和令牌消费者文件。 | 新的页面本地令牌系统与现有样式竞争，令牌文件被创建但未被导入/消费，或原始值仍散落在重复 UI 中。 |

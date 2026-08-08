# UIX 场景：移动原生

用于 iOS、Android、React Native、Flutter、Swift、Kotlin 或类原生移动应用界面。

## 基线

- 尊重平台导航、安全区域、触摸目标和系统约定。
- 密度为 `comfortable`。
- 屏幕支持一个清晰的用户任务并保留导航上下文。
- 原生 UI 不得看起来像压缩到手机中的桌面 Web 表格。

## 屏幕解剖结构

```text
Root navigation
  -> tab/stack/shell
    -> screen header
    -> scrollable content
    -> sticky or contextual action area
    -> sheet/dialog only for focused secondary work
```

## 必需模式

- 匹配平台预期的栈/标签导航。
- 安全区域感知的页眉、底部栏、面板和操作。
- 足够大的触摸目标和可达的主要操作。
- 相关时的离线/加载/错误/权限状态。
- 带移动键盘、验证和保留值的表单输入。

## 组件指导

- 列表使用带清晰行标识和状态的原生列表/卡片模式。
- 表单使用分组部分、可见标签和输入特定键盘。
- 底部面板用于简短选择或确认；复杂流程使用全屏。
- 破坏性或金融操作根据严重程度需要确认、审查或撤销。
- 空状态应提供下一个原生操作。

## 原生状态处理

- 加载应使用平台合适的进度指示器或骨架/列表占位符。
- 权限状态应说明缺失的权限并在可能时路由到恢复。
- 离线状态应将不可用网络与空数据分离。
- 键盘感知布局必须保持活动字段和提交操作可达。

```text
screen -> loading/empty/error/content
action -> pending/success/failure
navigation -> back/cancel/restore context
```

## 验证

- 可用时使用模拟器/设备或框架预览。
- 检查安全区域、键盘行为、滚动和触摸目标。
- 可能时检查平台返回行为和焦点/语音标签。
- 仅当应用支持两者时才检查暗色/亮色模式。

## 避免

- 仅 Web 的悬停交互。
- 微小的表格单元格、拥挤的工具栏和桌面侧边栏。
- 忽略平台返回行为或安全区域。
- 仅在瞬态 toast 中隐藏关键操作状态。

## 平台解析

在样式化屏幕之前解析平台行为。同一产品操作在 iOS、Android 或跨平台运行时上可能需要不同的导航、权限、键盘和反馈行为。

| 关注点 | 必需决策 |
| --- | --- |
| 导航 | 平台返回手势/按钮、深度链接、标签/栈归属和重新启动后的恢复。 |
| 安全区域 | 状态栏、刘海、主指示器、面板和键盘的插入。 |
| 输入 | 键盘类型、焦点顺序、滚动到焦点字段、自动填充和关闭行为。 |
| 权限 | 权限前说明、拒绝状态、重试/设置路由和功能回退。 |
| 反馈 | 原生或平台一致的待处理、成功、错误和破坏性确认行为。 |
| 触摸 | 最小目标尺寸、手势冲突解决方案和可达的主要操作。 |

```text
screen shell -> platform header/back -> task content -> validation/permission
-> bottom or inline action -> success route or recoverable failure
```

不要将平台限制隐藏在通用错误中。说明用户下一步可以做什么并在恢复可能时保留已输入数据。

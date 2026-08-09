# UIX 技术栈：原生移动端

用于 React Native、Flutter、SwiftUI/UIKit、Jetpack Compose、Kotlin/Android 和类原生移动应用。

## 结构

- 遵循平台导航和状态约定。
- 分离屏幕、可重用组件、领域 hooks/服务和导航配置。
- 尊重安全区域、键盘避让和平台返回行为。

## 屏幕组合

```text
NavigationContainer
  Tab or Stack Navigator
    Screen
      SafeArea
      Header
      Scroll/List Content
      Sticky Action or Sheet
```

## 实现规则

- 触摸目标必须舒适且分开。
- 除非存在设计系统，否则使用平台文本、颜色、间距和阴影约定。
- 表单需要输入类型、验证、加载/提交和错误恢复。
- 列表需要空/加载/错误状态和稳定的项目标识。
- 敏感金融或破坏性操作需要明确的确认/审查。
- 将 web 令牌意图转换为平台主题系统。不要为仅原生目标创建 CSS 令牌文件。
- 将安全区域、键盘避让和权限状态作为屏幕实现的一部分，而非仅审查说明。

## 跨平台说明

- React Native：当工作流增长时将展示组件与屏幕编排分离。
- Flutter：保持 widget 聚焦并一致使用主题令牌。
- 原生 iOS/Android：除非有理由使用自定义控件，否则使用平台控件。
- UniApp 或小程序目标在选定时应同时加载 UniApp 技术栈参考。

## 验证

- 可用时使用模拟器/设备或框架预览。
- 检查安全区域、键盘行为、滚动和触摸目标。
- 当完整验证不可用时在证据中记录平台/设备或预览约束。

## 平台实现边界

UIX 拥有可见的屏幕组合和平台行为。将框架配置、包选择、网络、持久化和原生构建设置保留在项目的工程参考和现有代码约定中。

```text
platform navigation -> screen shell -> task region -> native input/list
-> local validation -> async action -> platform feedback -> next route
```

- 屏幕拥有其页眉/返回功能、安全区域容器、滚动区域和主要操作放置。
- 功能组件拥有加载、空、错误、禁用、权限和成功状态的可见表示；不要将这些留为不可见的服务结果。
- 将平台适配器放在窄接口之后，以便 iOS/Android 差异不复制业务界面。
- 在引入自定义控件之前，将原生控件用于语义、焦点、键盘、可访问性和破坏性确认。

## 屏幕状态和恢复

```ts
type ScreenState<T> =
  | { kind: 'loading' }
  | { kind: 'ready'; value: T }
  | { kind: 'empty'; action?: 'create' | 'retry' }
  | { kind: 'error'; message: string; canRetry: boolean };
```

在后台化、旋转、权限提示或失败请求后保持草稿输入、选定标识和导航返回上下文可恢复。成功的变更必须更新可见对象并路由到下一个有用屏幕，而非仅显示瞬态通知。

# UIX 技术栈：UniApp 和小程序

用于 UniApp、微信/支付宝小程序、H5/移动混合目标和类似跨平台移动界面。

## 结构

- 遵循目标平台的页面、组件、store 和路由约定。
- 当多个目标在范围内时将平台特定能力放在小适配器之后。
- 优先为移动任务流设计；桌面 Web 模式不应泄漏到小程序屏幕中。

## 建议的拆分

```text
pages/
components/
stores/
services/
styles/
  tokens
```

## 实现规则

- 尊重安全区域、原生导航栏、标签栏和平台手势期望。
- 根据仓库现有技术栈使用平台兼容的单位 和组件。
- 保持表单单列、触摸友好并明确验证。
- 避免仅悬停交互和微小的表格布局。
- 在触发它们的页面上处理加载、空、错误、权限和业务阻塞状态。
- 将令牌模板意图转换为项目的 UniApp 样式变量或主题文件；不要添加目标无法消费的仅 Web CSS。
- 保持页面操作通过拇指友好的间距和平台键盘行为可达。

## 页面模式

```html
<view class="page">
  <view class="page-header"></view>
  <scroll-view class="page-content"></scroll-view>
  <view class="page-actionbar"></view>
</view>
```

使用项目原生语法和组件；此模式关于区域，而非确切标记。

## 验证

- 存在时使用可用的小程序/H5 预览目标。
- 检查安全区域、键盘行为、滚动和平台权限流程。
- 验证目标特定 API 限制或权限提示不会让用户停留在空白页面。

## 跨目标页面边界

UIX 拥有页面区域和移动任务流。平台条件编译、包配置、API 适配器和目标构建规则保留在仓库的 UniApp 工程边界中。

```text
pages.json route -> page shell -> scroll/content region -> action bar
-> validation/permission -> platform result -> updated page or next route
```

- 在 H5、小程序和原生构建间保持一个页面标识和一个主要任务；在平台能力需要时适配控件。
- 使用平台原生安全区域、导航栏、标签栏和键盘行为，而非导入桌面 Web 布局假设。
- 将加载、空、权限、错误、成功和业务阻塞反馈放在拥有操作的页面区域中。
- 围绕平台适配器或能力特定控件使用条件编译，而非围绕重复的业务工作流标记。
- 扩展现有 `uni.scss` 或主题变量以获取语义令牌。不要创建目标渲染器无法消费的仅 Web CSS 层。

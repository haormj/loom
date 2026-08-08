# UIX 技术栈：Three.js

用于 Three.js、React Three Fiber、WebGL、画布密集型 3D、配置器、游戏、模拟和沉浸式场景。

## 结构

- 保持场景设置、资产加载、控件、UI 覆盖和业务状态分离。
- 存在时使用仓库现有的 Three.js/R3F 约定。
- 场景应是沉浸式任务的主要界面，而非卡片内的装饰性预览。

## 场景模块拆分

```text
scene/
  create-scene
  camera
  lighting
  controls
  assets
  interactions
ui/
  SceneHud
  SceneInspector
  LoadingOverlay
```

## 实现规则

- 定义稳定的画布尺寸和调整大小行为。
- 为资产、WebGL 和着色器失败提供加载和回退状态。
- 保持相机、光照、控件和对象构图有意。
- 覆盖 UI 必须保持可读且不覆盖主体。
- 处置资源并避免失控的动画循环。
- 可行时尊重减弱动效或提供低动效控件。
- 通过与 UI 其余部分相同的语义令牌样式化 DOM 覆盖控件。
- 不要让场景代码拥有属于 UI 覆盖组件的业务表单/表格/详情状态。

## 渲染模式

```css
.scene-root { position: relative; width: 100vw; height: 100dvh; overflow: hidden; }
.scene-canvas { position: absolute; inset: 0; }
.scene-overlay { position: absolute; inset: 0; pointer-events: none; }
.scene-overlay > * { pointer-events: auto; }
```

## 验证

- 检查非空白画布像素。
- 检查桌面和移动构图。
- 确认资产加载和控件响应。
- 验证应用在调整大小或路由变化后保持可交互。
- 对于生成的场景，检查画布像素和覆盖控件两者；缺少任一都不完整。

## 场景、资产和覆盖边界

将渲染循环、资产、控件和产品 UI 保持为独立所有者。场景提供空间上下文；DOM 或原生覆盖组件提供标签、表单、状态和业务操作。

```text
scene root -> canvas/camera/controls -> selected object
                                   \-> overlay context/action/feedback
asset lifecycle -> loading -> ready | fallback | retry
```

- 场景状态拥有相机、选择、构图和交互模式；业务记录和表单草稿保留在 UI/数据边界中。
- 覆盖面板必须保留可读对比度、指针/键盘访问、安全放置和与周围产品相同的语义令牌。
- 加载和能力失败需要稳定、可操作的回退，使产品任务在没有画布的情况下也可理解。
- 资产 URL、预加载策略、像素比、处置和动画节流遵循仓库的渲染/运行时约定；不要在组件中发明第二个资产注册表。
- 调整大小或路由变化必须协调相机构图和覆盖尺寸而不丢失选定标识或待处理操作状态。

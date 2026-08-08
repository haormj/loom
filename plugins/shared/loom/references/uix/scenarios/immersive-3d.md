# UIX 场景：沉浸式 3D

用于 Three.js/WebGL/canvas 场景、产品配置器、空间工具、游戏、模拟和沉浸式交互视觉。

## 基线

- 主要场景是全出血或主导的，而非困在装饰性卡片中。
- 场景必须渲染非空白、正确构图并响应预期交互。
- 密度为 `immersive`。
- 控件支持场景任务而不覆盖关键视觉内容。

## 场景布局

```html
<main data-region="scene-page">
  <canvas data-region="scene-canvas"></canvas>
  <section data-region="hud">
    <header data-region="hud-top"></header>
    <aside data-region="scene-inspector"></aside>
    <footer data-region="scene-controls"></footer>
  </section>
</main>
```

```css
.scene-page {
  position: relative;
  width: 100vw;
  height: 100dvh;
  overflow: hidden;
  background: var(--surface);
}

.scene-canvas {
  position: absolute;
  inset: 0;
}

.scene-hud {
  position: absolute;
  inset: 0;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  padding: var(--space-6);
  pointer-events: none;
}

.scene-hud > * {
  pointer-events: auto;
}
```

## 必需模式

- 稳定的画布尺寸和调整大小处理。
- 资产/着色器/WebGL 支持的加载和回退状态。
- 场景控件：相机、缩放、旋转、重置、选择、模式切换或检查器（视需要）。
- 带可读对比度和安全放置的覆盖 UI。
- 可行时的减弱动效或性能回退。

## 控件组合

- 将主要控件停靠或覆盖在可预测区域；避免覆盖对象/场景中心。
- 检查器面板在与场景竞争时应可折叠或响应式。
- 为不熟悉的场景控件使用标签/工具提示。
- 如果场景代表产品/配置/游戏状态，可见 UI 必须暴露当前模式、选定对象和可用操作。

## 验证

- 验证画布像素非空白。
- 检查桌面和移动构图。
- 确认引用的资产加载。
- 确认动画/交互在初始渲染后继续。
- 检查控件不遮挡主体。
- 可行时检查调整大小处理和设备像素比行为。

## 避免

- 静态占位画布。
- 无可检查对象的深色模糊背景。
- 遮挡主体或无法通过触摸使用的控件。

## 加载、回退和性能

沉浸式界面在资产加载前、WebGL 不可用时和视口变化后行为可预测之前是不完整的。

```text
initial loading -> asset progress or skeleton -> interactive scene
                                      \-> fallback explanation + usable action
```

- 在初始化前预留画布尺寸，以便加载不偏移周围 UI。
- 当进度有意义时显示正在加载哪个资产或阶段；无恢复的永久旋转器不是加载状态。
- 为 WebGL、着色器、资产或能力失败提供可操作的回退。回退可以是静态图像、可检查的对象视图或等价的产品任务界面。
- 在调整大小和路由变化中保持相机构图、控件和覆盖状态稳定。
- 限制像素比，处置未使用资产，并在界面隐藏或请求减弱动效时暂停或减少动画。
- 在桌面和移动宽高比下测试场景；非空白画布本身不证明构图正确或控件可用。

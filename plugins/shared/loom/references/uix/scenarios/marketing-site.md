# UIX 场景：营销站点

用于着陆页、产品营销、活动、定价、发布和转化导向页面。

## 基线

- 首屏直接传达产品、优惠或类别。
- 视觉媒体应在可能时揭示产品、结果、地点、人物或体验。
- 主视觉文本不在卡片内。在常见视口的折叠下方应提示下一部分。
- 密度为 `immersive` 或 `comfortable`。

## 页面结构

```html
<main data-region="marketing-page">
  <section data-region="hero"></section>
  <section data-region="proof"></section>
  <section data-region="product-workflow"></section>
  <section data-region="comparison-or-pricing"></section>
  <section data-region="conversion"></section>
</main>
```

```css
.marketing-hero {
  min-height: min(760px, 88dvh);
  display: grid;
  align-items: end;
  padding: var(--space-8);
  position: relative;
  overflow: hidden;
}

.hero-copy {
  max-width: 760px;
  padding-bottom: var(--space-8);
  z-index: 1;
}

.hero-media {
  position: absolute;
  inset: 0;
  object-fit: cover;
}
```

## 必需模式

- 清晰的标题、支持性价值主张、主要 CTA 和次要路由。
- 产品证明：截图、真实媒体、数据、证言、比较、演示或工作流预览。
- 带变化布局的分区节奏，而非重复的等大卡片。
- 不重叠的响应式媒体和排版。
- 可访问的 CTA 和导航。

## 布局规则

- 可用时使用全出血或沉浸式主视觉媒体。
- 避免让页面看起来模板化的分割文本/卡片主视觉布局。
- H1 应是品牌/产品/地点/人物名称或字面优惠/类别。
- 支持文案承载描述性价值主张。
- 保持 CTA 组可见但不浮在不可读的媒体上。

## 转化界面质量

- 主视觉媒体应在存在时揭示产品、地点、人物、对象、工作流或结果。
- 证明部分应使用具体证据：截图、带上下文的真实指标、客户 logo、比较、演示或示例。
- 定价/比较部分必须可扫描且响应式；不要在长散文中隐藏计划差异。
- 表单和 CTA 需要像其他产品 UI 一样的验证、成功和错误状态。
- 首屏应提示下一部分，使页面感觉连续，而非像全屏海报。

## 验证信号

- H1 是产品/优惠/类别/名称，而非模糊口号。
- 主要和次要 CTA 在断点间保持可见和可读。
- 媒体有可访问的 alt/标签且不遮挡文本。

## 避免

- 无产品特异性的通用"主视觉 + 4 功能 + 证言 + CTA + 页脚"。
- 当需要相关媒体/产品视图时仅渐变的主视觉背景。
- 太小、模糊或裁剪到无法检查的装饰性截图。
- 运营产品首屏内的营销部分。

## 滚动节奏和证明

营销页面需要从优惠到证据到操作的可读序列。每个部分应通过回答买家问题或启用下一个决策来赢得其空间。

```text
offer/object -> problem or outcome -> concrete proof -> comparison/details
-> objection handling -> primary conversion action
```

- 将字面产品、地点、对象或优惠放在首屏；描述性价值主张支持它。
- 将声明与可检查的媒体、产品状态、客户证据、比较事实或具体演示配对。
- 每部分保持一个主要转化操作，并在媒体或交互后保留到它的清晰路由。
- 使用部分过渡建立层次结构，而非创建隐藏下一个有用内容的装饰性空白。
- 仅当页面足够长时才重复基本操作，并保持其标签一致。

## 媒体和交互

- 图像和视频揭示实际产品并在移动端保持可读；不要在检查重要时使用深色或模糊媒体。
- 交互式演示暴露稳定的回退、键盘替代、减弱动效行为和清晰的重置路径。
- 轮播暴露幻灯片标识、控件、暂停行为和非动画方式检查每个项目。
- 表单显示字段级错误、保留输入、防止重复提交并确认提交目标或下一步。

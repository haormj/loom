# UIX 场景：企业站点

用于公司、组织、场馆、作品集、机构或品牌信息站点。

## 基线

- 品牌、组织、地点或对象必须是首屏信号。
- 导航应使利益相关者任务明显：概览、服务、案例、新闻、联系、招聘、文档或支持。
- 视觉基调应适合组织而不落入类别反射性调色板。
- 密度为 `comfortable`。

## 结构

```html
<main data-region="corporate-site">
  <section data-region="identity-hero"></section>
  <section data-region="capabilities"></section>
  <section data-region="proof"></section>
  <section data-region="resources-or-news"></section>
  <section data-region="contact"></section>
</main>
```

```css
.corporate-section {
  padding: var(--space-12) var(--space-6);
}

.corporate-inner {
  width: min(100%, 1180px);
  margin: 0 auto;
}

.proof-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: var(--space-6);
}
```

## 必需模式

- 清晰的标识和主要受众路径。
- 带真实证明的结构化内容部分：能力、案例、位置、人员、历史、媒体或资源。
- 可见且可访问的联系/转化路径。
- 保持可检查的响应式媒体。
- 带有用导航的页脚，而非填充。

## 布局规则

- 使用编辑层次结构、分区带和媒体构图。
- 保持散文可读，避免在宽屏幕上拉伸段落。
- 仅对重复内容如案例、人员、文章或资源使用卡片。
- 在可行时在首屏保留下一部分的提示。

## 内容界面

- 标识主视觉：品牌/对象/地点立即可见；支持文案说明角色和受众。
- 证明：案例、资质、位置、产品、媒体、团队、客户或成果。
- 资源/新闻：列表项需要日期/类别/标题和清晰路由。
- 联系：业务路径、表单或联系渠道，以及交互时的验证/成功/错误状态。

```html
<section data-region="identity-hero">
  <div data-region="identity-copy"></div>
  <figure data-region="identity-media"></figure>
</section>
```

## 验证信号

- 品牌/产品/地点/对象在首屏可见。
- 导航、联系和页脚链接支持真实利益相关者任务。
- 图像/媒体保持可检查而非仅有氛围。

## 避免

- 带抽象渐变且无具体标识的通用企业主视觉。
- 隐藏主要操作的超载导航。
- 不揭示实际组织或产品的库存风格图像。

## 证明和转化连续性

页面必须将标识连接到可信的证明和利益相关者操作。没有证据或可达联系路径的主视觉句子是不完整的企业界面。

```text
identity -> capability or offering -> proof -> relevant resource -> contact/action
```

- 将每个主要声明与可检查的证明匹配，如案例、位置、产品详情、资质、人员、文档或成果。
- 为证明项提供标识、类别、日期和目标（当它们链接到更深内容时）。
- 在证明部分之后保持主要联系或咨询操作可用；不要让用户返回主视觉。
- 交互式联系表单需要字段级验证、保留输入、提交状态、成功确认和可恢复失败。
- 当媒体揭示组织、场馆、产品、人员或工作时将其视为证据。装饰性氛围不能承载主要声明。

## 响应式标识

在窄宽度下，在减少装饰之前保留标识信号和利益相关者路径。使用保持对象、证明和操作连接的单列阅读顺序。

```css
.identity-hero {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(280px, 0.8fr);
  gap: var(--space-8);
  align-items: center;
}

@media (max-width: 767px) {
  .identity-hero { grid-template-columns: 1fr; gap: var(--space-5); }
  .identity-hero [data-region="identity-media"] { order: -1; }
}
```

不要在移动端将组织名称、位置、联系路由或证明标签隐藏在仅悬停交互之后。

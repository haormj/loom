# UIX 场景：文档站点

用于文档、知识库、API 参考、指南、技术手册和帮助中心。阅读、搜索、导航和示例是产品工作流。

## 基线

- 首屏显示文档结构和到达有用内容的直接路径。
- 密度为 `balanced`。
- 当任务是参考查找时，UI 不应看起来像营销页面。
- 内容宽度、代码可读性、导航状态和搜索质量比装饰更重要。

## 文档外壳

```html
<div data-region="docs-shell">
  <header data-region="docs-topbar"></header>
  <aside data-region="docs-nav"></aside>
  <main data-region="docs-content"></main>
  <aside data-region="docs-toc"></aside>
</div>
```

```css
.docs-shell {
  min-height: 100dvh;
  display: grid;
  grid-template-columns: 260px minmax(0, 760px) 220px;
  justify-content: center;
  gap: var(--space-8);
  padding: 0 var(--space-6);
}

.docs-nav,
.docs-toc {
  position: sticky;
  top: 64px;
  height: calc(100dvh - 64px);
  overflow: auto;
}

.docs-content {
  min-width: 0;
  padding: var(--space-8) 0 var(--space-12);
  line-height: 1.65;
}

@media (max-width: 1180px) {
  .docs-shell {
    grid-template-columns: 240px minmax(0, 760px);
  }
  .docs-toc {
    display: none;
  }
}

@media (max-width: 767px) {
  .docs-shell {
    display: block;
    padding: 0 var(--space-4);
  }
  .docs-nav {
    position: fixed;
    inset: 0 auto 0 0;
    width: min(300px, 86vw);
    transform: translateX(-100%);
    background: var(--surface);
    z-index: var(--z-modal);
  }
}
```

## 必需模式

- 左侧导航或部分索引、内容区域和可选的右侧目录。
- 当内容量需要时的搜索或命令面板。
- 带语言标签、复制控件、可读对比度和溢出行为的代码块。
- 用于注意/提示/警告/危险状态的标注。
- 当前页面/部分状态和上一页/下一页路由。
- 带恢复建议的搜索空/无结果状态。

## 内容解剖结构

```html
<article data-region="doc-page">
  <header>
    <p data-region="eyebrow"></p>
    <h1></h1>
    <p data-region="summary"></p>
  </header>
  <section data-region="body"></section>
  <nav data-region="page-pagination"></nav>
</article>
```

```css
.docs-content h1 { font-size: var(--text-3xl); line-height: 1.15; }
.docs-content h2 { margin-top: var(--space-10); padding-bottom: var(--space-2); border-bottom: 1px solid var(--border); }
.docs-content p,
.docs-content li { max-width: 72ch; }
.docs-content pre { overflow: auto; border-radius: var(--radius-md); }
.docs-content code { font-family: var(--font-mono); }
```

## 文档交互

- 搜索/无结果状态应建议替代术语或导航。
- 代码示例仅在实现复制时需要复制控件。
- API 参考页需要带稳定锚点的参数、响应、错误和示例部分。
- 指南需要前提条件、步骤、预期结果和故障排除。
- 版本、平台或语言切换器应显示当前选择。

## 验证信号

- 左侧导航活动状态和页面标题一致。
- 长代码块滚动而不破坏页面。
- 移动导航打开/关闭并返回内容而不意外丢失滚动。
- 页面可在无营销内容阻碍参考查找的情况下阅读。

## 避免

- 全宽段落。
- 对比度差或无溢出行为的代码块。
- 在桌面上将文档导航隐藏在多次点击之后。
- 延迟访问文档的营销主视觉部分。

## 阅读和代码交互

文档页面有两个同时进行的任务：让读者理解概念并让他们使用示例。保持说明、代码和结果连接。

```html
<article data-region="article">
  <header data-region="article-heading"></header>
  <nav data-region="on-this-page" aria-label="On this page"></nav>
  <section data-region="article-body"></section>
  <pre data-region="code-example"><code></code></pre>
  <div data-region="example-feedback" aria-live="polite"></div>
</article>
```

- 使用描述任务或概念的标题，而非仅产品功能名称。
- 保持可运行示例靠近前提条件、预期输出和下一步。
- 代码块需要可读的换行或水平滚动、语言标签和带反馈的复制操作。
- 不要将必需说明仅放在悬停工具提示、折叠面板或图像中。
- 清楚标记外部链接和版本特定行为，同时在读者处于交互式示例中时将设置细节排除在产品的主要成功消息之外。

## 响应式阅读

在窄宽度下，保持阅读顺序并使代码可检查而不缩小到不可用的大小。

```css
.docs-layout {
  display: grid;
  grid-template-columns: 15rem minmax(0, 1fr) 13rem;
  gap: var(--space-8);
}

.docs-code {
  max-width: 100%;
  overflow-x: auto;
  tab-size: 2;
}

@media (max-width: 900px) {
  .docs-layout { grid-template-columns: minmax(0, 1fr); }
  .docs-toc { order: -1; }
}
```

在侧边栏和目录折叠后保持文章标题、当前部分、代码复制控件和导航可达。

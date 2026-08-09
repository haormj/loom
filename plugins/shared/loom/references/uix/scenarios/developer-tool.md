# UIX 场景：开发者工具

用于 IDE 类工具、API 浏览器、SDK 控制台、构建/部署工具、日志、调试、自动化和运行时仪表板，其中技术内容是产品的一部分。

## 基线

- 仅当技术术语是面向用户的产品概念时才允许使用。
- 首屏暴露工具工作区，而非交付说明。
- 密度通常为 `workbench_dense`。
- 等宽字体、代码块、日志、命令片段和结构化元数据需要仔细的层次结构。

## 工作区布局

```css
.devtool-shell {
  min-height: 100dvh;
  display: grid;
  grid-template-columns: 260px minmax(0, 1fr);
  background: var(--surface);
}

.devtool-workspace {
  min-width: 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(320px, 420px);
  grid-template-rows: auto minmax(0, 1fr);
}

.devtool-output {
  min-width: 0;
  overflow: auto;
  font-family: var(--font-mono);
}
```

## 必需模式

- 带导航、命令/操作区域、输出/结果区域和详情/错误面板的工作区外壳。
- 复制按钮、状态指示器、日志、重试操作和清晰的失败分类。
- 键盘友好控件和可见焦点。
- 带换行或水平滚动的代码/日志格式化。
- 帮助开发者启动的空状态。

## 交互

- 复制/下载/破坏性运行时控件必须在视觉上区分。
- 当量大时日志需要筛选、搜索和时间戳/来源上下文。
- 错误应分离用户配置问题、环境失败和工具失败。
- 仅在实现且有用时才显示键盘快捷键。

## 结果和错误界面

```html
<section data-region="tool-runner">
  <header data-region="tool-controls"></header>
  <section data-region="tool-output"></section>
  <aside data-region="tool-diagnostics"></aside>
</section>
```

- 输出应有状态、已知时的持续时间/进度、有用时的复制/下载以及清晰的空状态。
- 配置表单应在运行昂贵或破坏性操作前验证。
- 原始 JSON/日志可以显示，但仅在带结构、搜索、换行/滚动和用户应做什么的说明时。
- 危险操作应通过放置和样式与常规控件分离。

## 验证信号

- 键盘和复制流程工作。
- 长日志或 JSON 不溢出页面。
- 技术错误对开发者有用而不暴露无关的内部工作流细节。

## 避免

- 在非开发者产品中使用开发者术语。
- 当用户需要决策时无结构地转储原始 JSON 或日志。
- 在常规复制/下载控件旁边隐藏破坏性运行时操作。

## 安全技术输出

仅当技术输出帮助用户检查或操作工具时，它才是产品的一部分。保持原始输出有界并区分用户可操作的失败与诊断细节。

```html
<section data-region="result" aria-live="polite">
  <header data-region="result-summary"></header>
  <div data-region="actionable-message"></div>
  <details data-region="diagnostic-output">
    <summary>Details</summary>
    <pre><code></code></pre>
  </details>
</section>
```

- 将状态、持续时间、结果标识和下一步操作放在摘要区域。
- 当堆栈跟踪、请求负载和冗长日志有用但下一次决策不需要时，将它们放在显式详情边界之后。
- 在渲染或复制输出前编辑密钥、令牌、个人数据和环境路径。
- 提供带清晰成功和失败反馈的复制和重试操作；不要让用户用指针选择日志。
- 运行失败时保留命令、查询或选定对象，以便修正不需要重建设置。

## 键盘工作流

键盘移动必须遵循工作区顺序：导航、输入/命令、运行操作、结果，然后是详情。焦点应移到新可用的结果或错误内容而不将用户困在面板中。

- 每个仅图标命令都有可访问名称和可见焦点状态。
- 快捷操作必须有指针可访问的等价物，且不得在用户于文本编辑器中输入时触发。
- 可调整大小的面板有键盘可访问的替代或可用的默认宽度。
- 复制、重试、取消和展开操作通过本地状态文本报告其结果。

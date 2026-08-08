# UIX 场景：消费者应用

用于面向客户的 Web 应用、门户、预订、商务、内容、学习、生产力和服务工作流。

## 基线

- 首个视口让用户启动或继续产品任务。
- 密度通常为 `comfortable` 或 `balanced`。
- 导航面向任务且可恢复。
- 视觉设计可以富有表现力，但工作流清晰度优先。

## 应用结构

```html
<main data-region="consumer-app">
  <header data-region="app-header"></header>
  <section data-region="current-task"></section>
  <section data-region="object-list-or-feed"></section>
  <section data-region="detail-or-checkout"></section>
</main>
```

```css
.consumer-shell {
  min-height: 100dvh;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
}

.consumer-content {
  width: min(100%, 1120px);
  margin: 0 auto;
  padding: var(--space-4);
}

@media (min-width: 768px) {
  .consumer-content {
    padding: var(--space-6);
  }
}
```

## 必需模式

- 清晰的主要路径和次要操作。
- 账户/会话、保存状态或进度指示器（相关时）。
- 带验证、输入保留和清晰成功/错误状态的表单。
- 带有用下一步操作的空状态。
- 支持触摸和窄内容的响应式行为。
- 浏览对象时一致的卡片/列表/详情。

## 工作流规则

- 多步流程需要进度和返回/取消行为。
- 详情页暴露主要操作而不隐藏支持信息。
- 推荐或相关内容不得隐藏用户当前任务。
- 通知和 toast 不应是已完成操作的唯一记录。

## 界面模式

- 主页/任务界面：恢复当前项目、启动主要任务或显示相关信息流。
- 浏览/列表界面：当量需要时可搜索/筛选，带清晰的选定项目路径。
- 详情/操作界面：主要操作、支持事实、相关历史和恢复路径。
- 账户/设置界面：用户控制的偏好和状态，不隐藏在营销文案之后。

```html
<section data-region="task-surface">
  <header data-region="task-context"></header>
  <section data-region="task-body"></section>
  <footer data-region="task-actions"></footer>
</section>
```

## 验证信号

- 首屏让用户操作或恢复，而非仅阅读产品承诺。
- 移动布局保持主要操作、错误和成功可见。
- 长用户生成内容和空状态不破坏卡片/列表。

## 避免

- 应用任务的仅营销首屏。
- 不帮助用户操作的通用卡片网格。
- 仅在悬停时出现的关键控件。

## 浏览、详情和提交

消费者工作流应从发现到决策再到确认结果有可见的进展。在每个步骤保持当前项目和待决策清晰可读。

```text
browse/filter -> item identity and summary -> detail facts -> primary action
-> confirmation or review -> success state with updated item/order/progress
```

- 浏览界面暴露足够的标识、状态、价格、日期或进度，以便无需打开每个卡片即可选择项目。
- 详情界面重复项目标识并将主要操作放在其提交的事实附近。
- 表单或结账在验证或可恢复请求错误时保留输入值。
- 确认与风险成正比。对不可逆、付费或隐私敏感操作使用审查步骤；不要为无害导航添加确认。
- 成功就地更新受影响对象或进度，并提供返回用户下一个有用操作的路由。

```html
<section data-region="browse-results" aria-busy="false"></section>
<section data-region="selected-detail" aria-labelledby="item-title">
  <h1 id="item-title"></h1>
  <div data-region="item-facts"></div>
  <div data-region="commit-feedback" aria-live="polite"></div>
  <button data-action="commit"></button>
</section>
```

不要让推荐、促销卡片或全局 toast 替代完成任务所需的详情、决策和结果界面。

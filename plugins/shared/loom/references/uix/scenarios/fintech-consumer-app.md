# UIX 场景：金融科技消费者应用

用于面向客户的金融、交易、银行、钱包或投资体验。信任、清晰、错误恢复和移动可用性比新颖性更重要。

## 基线

- 首屏支持客户当前的金融任务：概览、账户、交易、订单、支付或投资操作。
- 密度为 `comfortable` 或 `balanced`。
- 敏感值、确认和风险消息必须明确且本地化。
- 界面区分待处理、已完成、失败、阻塞和可逆状态。

## 移动应用外壳

```css
.finance-app {
  min-height: 100dvh;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  background: var(--surface);
}

.finance-content {
  min-width: 0;
  overflow: auto;
  padding: var(--space-4);
  padding-bottom: calc(var(--space-4) + env(safe-area-inset-bottom, 0));
}

.finance-bottom-nav {
  min-height: 64px;
  padding-bottom: env(safe-area-inset-bottom, 0);
  border-top: 1px solid var(--border);
  background: var(--surface-raised);
}
```

## 必需模式

- 带单位和最后更新上下文的账户/投资组合摘要（相关时）。
- 带高风险操作审查步骤的交易/订单/操作表单。
- 内联验证和业务阻塞消息。
- 带参考 ID、状态和下一步操作的确认回执。
- 不暴露实现细节的安全和权限状态。
- 当目标类应用时的移动优先导航和触摸目标。

## 金融组件

```html
<section data-region="account-summary">
  <header></header>
  <div data-region="primary-balance"></div>
  <div data-region="quick-actions"></div>
</section>

<section data-region="transaction-list"></section>
<section data-region="review-and-confirm"></section>
<section data-region="receipt"></section>
```

```css
.amount {
  font-variant-numeric: tabular-nums;
  letter-spacing: 0;
}

.transaction-row {
  min-height: 64px;
  display: grid;
  grid-template-columns: 40px minmax(0, 1fr) auto;
  gap: var(--space-3);
  align-items: center;
}
```

## 敏感操作流程

```html
<section data-region="financial-action">
  <section data-region="input-step"></section>
  <section data-region="review-step"></section>
  <section data-region="receipt-step"></section>
</section>
```

- 审查屏幕应在相关时显示金额、账户、费用/限额、时间和不可逆后果。
- 回执应包含参考 ID、时间戳、状态和下一步操作。
- 安全提示应说明用户必须做什么，而非系统如何实现。
- 当产品需要隐私时遮蔽敏感值，但保留足够的上下文用于确认。

## 状态

- 加载保留余额/交易布局，绝不显示误导性的零值。
- 空状态说明是无数据、访问受限还是设置未完成。
- 错误包含金融操作的恢复和支持路由。
- 业务阻塞区分余额不足、资格、限额、合规和市场状态限制。
- 成功回执持续足够长以便复制/分享/参考。

## 验证信号

- 加载状态绝不暗示不正确的余额/订单状态。
- 风险/费用/限额信息在确认前可见。
- 失败金融操作的错误恢复路径可见。
- 移动触摸目标和安全区域已检查。

## 避免

- 仅通过消失的 toast 确认风险操作。
- 损害可读性的深色或"高端"样式。
- 隐藏费用、限额或不可逆后果。

## 金额和风险展示

金融信息必须在用户提交前可读。在同一决策路径中展示值、货币、方向、时间、状态和相关费用或限额。

```text
account or counterparty -> amount + currency -> fee/rate/limit -> timing
-> risk or eligibility message -> review action
```

- 用明确的货币上下文格式化货币；永远不要依靠颜色或裸数字来传达借记与贷记。
- 保持待处理、已完成、失败、撤销和阻塞状态区分并说明下一个可用操作。
- 当时间影响决策时显示日期和时区。避免对结算或截止日期使用模糊的相对日期。
- 在为确认保留足够标识的同时部分遮蔽敏感标识符。
- 当费用、汇率、限额或资格原因影响审批时，不要将它们隐藏在次要交互之后。

## 交易反馈

```html
<section data-region="transaction-review">
  <dl data-region="transaction-facts"></dl>
  <p data-region="risk-message"></p>
  <div data-region="submit-feedback" aria-live="polite"></div>
  <button data-action="confirm" data-state="ready"></button>
</section>
```

- 请求待处理时禁用重复提交并保持审查事实可见。
- 成功时显示持久的交易标识、更新状态以及到历史或支持的路由。
- 失败时保留输入值并区分可修正的验证问题与被拒绝或不可用的操作。
- 成功 toast 可以补充结果但不能是资金移动的唯一确认。

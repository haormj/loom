# UIX 场景：金融科技工作站

用于面向员工的金融、证券、账户、风险、合规和交易运营。UI 必须支持准确性、可审计性和业务规则清晰度。

## 基线

- 首屏是运营工作站：队列/列表、筛选器、详情和操作路径。
- 密度通常为 `workbench_dense`，带清晰的视觉分组。
- 业务状态和阻塞规则比装饰性品牌更突出。
- 数字数据使用表格对齐和清晰的单位/货币/日期格式。

## 工作站外壳

```css
.fintech-workstation {
  min-height: 100dvh;
  display: grid;
  grid-template-columns: 240px minmax(0, 1fr);
  background: var(--surface);
}

.record-workspace {
  min-width: 0;
  display: grid;
  grid-template-columns: minmax(420px, 1fr) minmax(360px, 440px);
  gap: var(--space-4);
  padding: var(--space-4);
}

@media (max-width: 1100px) {
  .record-workspace {
    grid-template-columns: minmax(0, 1fr);
  }
}
```

## 必需模式

- 按业务标识符搜索：账户 ID、客户 ID、订单 ID、交易 ID、证书 ID 或类似领域键。
- 带当前状态、风险、资格和最新事件的列表/详情工作流。
- 与选定记录关联的业务操作：开户、审批、拒绝、冻结、关闭、补发、绑定、存款、取款等（视适用情况）。
- 操作附近的规则反馈：为何阻塞、什么条件必须改变以及是否可以重试。
- 领域需要可追溯性时的审计跟踪或事件历史。
- 所有可见标签使用面向员工的中文文案或项目语言。

## 记录详情解剖结构

```html
<section data-region="record-detail">
  <header data-region="record-summary"></header>
  <section data-region="eligibility"></section>
  <section data-region="action-panel"></section>
  <section data-region="domain-fields"></section>
  <section data-region="event-history"></section>
</section>
```

```css
.money,
.quantity,
.account-number {
  font-variant-numeric: tabular-nums;
}

.rule-block {
  border: 1px solid var(--danger-border);
  background: var(--danger-surface);
  color: var(--danger-text);
  border-radius: var(--radius-md);
  padding: var(--space-3);
}
```

## 操作和风险模式

```html
<section data-region="action-eligibility">
  <dl data-region="risk-facts"></dl>
  <div data-region="eligible-actions"></div>
  <div data-region="blocking-rules" aria-live="polite"></div>
</section>
```

- 在高风险操作前显示资格。
- 分离警告、错误和业务阻塞状态。
- 为审计敏感流程保持操作者、时间戳、受影响账户/订单和最新状态可见。
- 对不可逆或受监管操作使用明确的确认/审查步骤。

## 状态

- 加载作用域限于列表/详情/操作面板。
- 空状态区分无记录、无搜索结果和权限不可用。
- 技术错误可恢复且无堆栈跟踪或内部工具名称。
- 业务阻塞消息引用规则和受影响对象。
- 成功刷新列表/详情并追加或更新可见事件。

## 验证信号

- 数值使用表格对齐和单位。
- 业务阻塞文案引用受影响记录和规则。
- 需要时审计/历史区域在成功状态转换后变化。
- 无金融/风险状态仅通过颜色传达。

## 避免

- 降低可读性的通用"金融"视觉陈词滥调。
- 仅通过工具提示隐藏风险或资格。
- 将业务规则阻塞视为通用错误。
- 显示无单位、符号、日期或状态的金融数字。

## 密集金融工作台

密度仅在标识和风险保持可见时支持重复审查。使用稳定列和紧凑行进行比较，然后展开选定记录而不丢失工作队列。

```text
queue filters -> record identity/status -> amount and key dates -> eligibility
-> selected detail -> approval/rejection -> row + audit update
```

- 在首次扫描路径中保持队列条件、选定记录标识、状态、金额、货币和操作资格。
- 对可比金额使用表格数字和右对齐，但保持货币和单位标签相邻。
- 保持稳定的详情区域或路由，在导航后保留队列位置、筛选器和选择。
- 用文本和明确原因表示待处理、升级、阻塞、批准、拒绝和撤销状态（适用时）。
- 避免将队列或审批操作推到首个有用视口下方的装饰性 KPI 面板。

## 风险和审计连续性

- 高影响操作在提交前显示审查摘要、操作者/角色、授权条件和确认要求。
- 拒绝或升级需要出现在受影响决策旁边和历史视图中的原因。
- 审计事件包括时间戳、操作者、操作、先前状态、下一状态和对受影响记录的可读引用。
- 如果操作不可用，在操作附近显示业务规则或缺失权限而非静默禁用它。

```html
<aside data-region="decision-panel">
  <dl data-region="approval-facts"></dl>
  <p data-region="eligibility-message"></p>
  <textarea data-region="decision-reason"></textarea>
  <div data-region="decision-feedback" aria-live="polite"></div>
</aside>
```

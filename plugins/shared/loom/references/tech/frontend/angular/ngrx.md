# Angular 与 NgRx 的共享状态

仅当 TechnicalBaseline 选择 NgRx 且任务拥有共享客户端状态、reducer/store、选择器、effect、实体集合或跨界面生命周期时应用 NgRx。本地组件状态不能证明 store 的必要性。

## 状态所有权

围绕产品工作流和真相来源边界建模状态，而非每个 API 响应一个字段。仅保留跨组件/路由所需、用于协调 effect 或用于稳定工作流历史的状态。

功能状态在确实使用时可包含规范化实体、请求状态/错误、选定标识、筛选/分页、脏草稿、乐观操作和新鲜度元数据。不要重复选择器可以计算的派生列表/计数/资格。

保持服务端数据所有权显式。NgRx 状态是客户端表示/缓存，不是对并发后端变更的权威。

## 功能注册与键

在正确的应用/路由生命周期使用 `provideState`/`provideEffects` 或仓库的模块设置。保持功能键稳定且唯一；更改键可能破坏选择器、路由集成、持久化状态、devtools 和测试。

一次性注册 effect。惰性路由注册需要清晰的清理/重入行为，不得重复副作用。

## Action 与 Reducer

使用具有业务/事件源和类型化载荷的 action 组：

```typescript
export const OrdersActions = createActionGroup({
  source: 'Orders Workbench',
  events: {
    'Load Requested': props<{ filter: OrderFilter }>(),
    'Load Succeeded': props<{ orders: readonly OrderSummary[] }>(),
    'Load Failed': props<{ error: UiError }>(),
    'Approval Requested': props<{ orderId: string; expectedVersion: number }>(),
  },
});
```

在命令中包含稳定的目标/上下文，使 effect 不会在用户导航或筛选后读取可能已变更的 `selectedId`。

Reducer 是纯函数且不可变。在正确的发起/成功/失败事件上清除过期错误/加载/乐观状态。永远不要修改实体数组、嵌套草稿或错误对象。

对具有稳定标识和列表/详情更新的规范化集合使用 `createEntityAdapter`。仅当 `selectId` 和排序匹配领域标识和所需规范顺序时配置它们；分页顺序可能需要单独的 ID 列表。

## 选择器与 Facade

保持选择器类型化、纯、可组合，无 service 调用、修改、时间/随机或仅组件格式化。在多个界面需要时构建可复用的业务就绪视图模型。

避免在渲染期间无记忆化/生命周期控制的情况下重复创建的工厂选择器。优先使用 selected-ID 加 entities 选择器或复用选择器实例的 facade 方法。

当 facade 隐藏 store 机制、集中命令/视图模型或保护组件 API 免受 action/键变更影响时很有用。不要创建仅一一重命名每个 dispatch/select 的传递 facade。

在仓库选择时在容器边界将选择器转为 signal；展示型组件仍接收类型化值/事件。

## Effect 与并发

Effect 协调异步/外部工作并分派结果。按操作语义选择扁平化：

- `switchMap` 用于可替换的列表/筛选加载
- `exhaustMap` 用于重复提交预防
- `concatMap` 用于有序写入
- bounded `mergeMap` for independent operations

在内部操作内捕获错误以保持 effect 流存活。将验证/冲突/权限/不可用失败映射为类型化事件而非通用字符串。

仅当 action 到达后真正需要最新 store 值时使用 `concatLatestFrom`。不要通过宽泛状态读取隐藏缺失的 action 载荷上下文。

路由、toast、分析和其他非分派副作用需要 `dispatch: false`，不应替代产品可见状态/恢复。

## 乐观与持久化状态

乐观变更需要临时标识/版本、回滚/协调、重复响应处理和可见的 pending/失败行为。在无已接受设计的情况下避免对破坏性/高冲突操作的乐观更新。

仅持久化显式安全的状态，配以 schema/版本/迁移和登出/租户清除。默认永远不要持久化令牌、密钥、敏感记录、临时加载/错误或过期授权决策。

## Verification

- 测试 reducer 转换、不可变更新、entity adapter 标识/顺序和过期状态清除。
- 测试选择器的空/加载/错误、筛选/排序、选定标识、权限和视图模型派生。
- 测试 effect 的成功/失败、操作符并发、重复提交、取消、重试边界和非分派 effect。
- 在变更时在集成边界验证功能注册/键和惰性路由生命周期。
- 在拥有时验证乐观回滚/协调和持久化状态迁移/清除。
- 确认组件分派显示的记录标识并渲染选择器/facade 状态。

## 交付证据

标识功能键/状态、action、reducer/选择器/effect 决策和证明它的转换/发射断言。Redux DevTools 可见性或成功 API 响应不能证明不可变性、操作目标、effect 并发、回滚或持久化状态安全。

## 不安全默认行为

- 在无已选择技术栈和共享状态所有权的情况下加载/引入 NgRx。
- API 响应整体复制到重复的功能状态。
- Effect 读取可变选定状态而非 action 目标上下文。
- Reducer 修改嵌套/实体数据。
- 每个选择器/action 一个传递 facade 方法。
- Effect 流在第一次错误后终止。
- 无生命周期/迁移策略持久化敏感或授权状态。

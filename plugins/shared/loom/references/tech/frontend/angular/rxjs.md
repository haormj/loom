# Angular 与 RxJS 的响应式客户端流

为任务所属的流语义、API 绑定、取消、排序、扇出、订阅生命周期或共享 observable 结果使用 RxJS 指导。不要按约定将直接的本地 signal 状态转为流。

## 选择正确的响应式原语

对同步本地/派生视图状态使用 signal，对异步序列、取消、多次发射、路由/事件、HTTP 组合和 NgRx effect 使用 Observable。在 UI 边界用 `toSignal`/`toObservable` 转换以澄清所有权；避免重复转换循环。

Subject 是事件源，不是通用可变 store。在需要 subject 时暴露 `asObservable()` 并保持写入私有。仅当每个订阅者需要当前值且初始值有意义时优先使用 `BehaviorSubject`。

## 扁平化语义

从业务并发选择高阶操作符：

| 需求 | 操作符 |
|---|---|
| 最新搜索/筛选/详情优先 | `switchMap` |
| 独立有界操作可重叠 | `mergeMap` with concurrency |
| 写入必须保持顺序 | `concatMap` |
| 活动时忽略重复提交 | `exhaustMap` |

```typescript
readonly results$ = this.query.valueChanges.pipe(
  debounceTime(250),
  map(value => value.trim()),
  distinctUntilChanged(),
  switchMap(query => this.search.search(query).pipe(
    map(items => ({ kind: 'ready' as const, items })),
    catchError(error => of({ kind: 'error' as const, error: mapApiError(error) })),
  )),
);
```

不要对必须完成的写入使用 `switchMap`，或对双击敏感命令使用 `mergeMap`。操作符选择是产品正确性的一部分。

## 加载、错误与终结

在应恢复的边界内放置 `catchError`。在长寿命操作/搜索流外部捕获可能永久终止它。不要将每次失败转为 `[]`/`null`；保留类型化的验证、冲突、权限、不可用和传输状态。

对必须在成功、错误和取消时运行的加载清理使用 `finalize`，但当 UX 行为不同时区分取消和可见失败。

仅对已接受的瞬时/幂等操作进行有界尝试、延迟/抖动、取消和最终错误的重试。永远不要通过通用拦截器/操作符重试验证/auth/业务冲突或非幂等写入。

## 组合与完成

对长寿命最新值输入使用 `combineLatest`，对必须全部完成的有限操作使用 `forkJoin`，对位置配对使用 `zip`，对独立发射使用 `merge`。确保每个源有所需的初始/完成行为；一个从不发射的 `combineLatest` 源可能使视图停滞。

当一个源可能失败而不使整个界面失效时显式建模部分失败。避免依赖调用的嵌套订阅；通过操作符组合使取消和错误保持可见。

## 生命周期与清理

为组件/路由生命周期优先使用 async pipe、`toSignal` 或 `takeUntilDestroyed`。在注入上下文之外调用时捕获 `DestroyRef`。

具有应用生命周期的 service 不应使用组件销毁作为清理模型。显式定义缓存/订阅生命周期并在所属 provider 结束时释放 WebSocket/事件/浏览器资源。

避免订阅中的订阅、被遗忘的事件流和命令式订阅数组。永远不要仅为触发 HTTP 请求而订阅同时丢弃其错误/取消语义。

## 共享与缓存

仅当有意共享一个结果且理解重置/失效语义时使用 `shareReplay({ bufferSize: 1, refCount: true })`。进程范围的 replay 可能泄漏用户/租户特定数据或在变更/登录变更后保持过期结果。

对于服务端数据，定义真相来源、新鲜度、失效、错误保留和重新获取行为。RxJS 共享不自动是持久缓存或状态管理架构。

## 背压与事件量

用合适的防抖/节流/采样/缓冲行为限定 typeahead、resize、scroll、upload 和轮询事件速率。保持轮询可见性、取消、重叠和重试显式；当界面/标识不再拥有时停止轮询。

为文件/批次/网络工作限制 `mergeMap` 并发。无界并行请求可能耗尽浏览器/provider 资源并扰乱用户反馈。

## Verification

- 当时序、取消、顺序或重试是声称的行为时使用聚焦的 scheduler/marble 测试。
- 在拥有处证明最新优先、重复提交预防、有序写入、有界并发和部分失败。
- 验证成功、失败、取消和快速重复操作后的加载/禁用状态。
- 测试组件销毁、路由变更、模态关闭、标识变更和流错误时的清理。
- 验证共享结果失效和无跨用户/租户泄漏。
- 当状态/错误语义驱动流时练习真实 HTTP adapter 映射。

## 交付证据

命名源流、所选并发/恢复规则和证明它的发射/订阅断言。流类型或一次成功发射不能证明取消、清理、重试边界、排序或缓存失效。

## 不安全默认行为

- 在无响应式/API 绑定任务时用描述关键词选择 RxJS 指导。
- 将 Subject 用作非结构化全局可变状态。
- 对必需写入应用 `switchMap` 或对重复提交应用 `mergeMap`。
- 错误转换为空数据且长寿命流意外终止。
- 无界重试、轮询、replay 或并行。
- 嵌套订阅和缺失清理。
- 共享 replay 保留标识敏感的过期数据。

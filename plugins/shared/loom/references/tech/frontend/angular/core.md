# Angular 应用实现

在仓库的 Angular 版本、应用引导、设计系统、API 契约和功能边界内实现已接受的前端体验。Angular 17+ 模式仅在所选项目版本支持时可用。

## 应用组合

保留 standalone 与 NgModule 架构。新 standalone 功能应使用 `ApplicationConfig`、`bootstrapApplication`、路由/provider 函数和显式组件导入；已建立的 NgModule 应用不应被部分重写，除非迁移为任务所属。

在组合根中一次性注册应用级 provider。将功能 service/state/route 保持在其所属能力附近，避免对应有路由/功能生命周期的有状态 provider 使用 `providedIn: 'root'`。

为可配置端口和浏览器/运行时抽象使用注入令牌。`inject()` 和构造函数注入均有效；遵循仓库风格，仅当函数真正需要注入上下文时使用 `runInInjectionContext`。

## 状态边界

按生命周期和共享需求选择状态：

| 状态 | 适用所有者 |
|---|---|
| 本地视觉/编辑状态 | component signals/form model |
| 派生本地状态 | `computed()` |
| 可复用功能操作 | service/facade |
| 共享跨界面生命周期 | selected store such as NgRx |
| URL 可共享筛选/选择 | router params/query params |
| 服务端真相来源 | API service/cache policy, not duplicated client truth |

Signal 是同步状态原语。对本地/派生状态使用 `signal`、`computed` 和受控 `effect`；避免将一个 signal 复制到另一个、发出不受控写入或隐藏依赖循环的 effect。

```typescript
readonly records = signal<readonly RecordSummary[]>([]);
readonly selectedId = signal<string | null>(null);
readonly selected = computed(() =>
  this.records().find(record => record.id === this.selectedId()) ?? null,
);
```

保持持久化记录足够不可变以使 OnPush/signal 相等性有意义。维护单独的可编辑草稿并在已接受保存/回读后协调。

## API 与错误边界

在类型化 service/adapter 中集中 HTTP 传输。保留已接受的方法、路径、载荷、状态、错误、auth、分页和同源/base URL 规则。不要跨组件重复端点字符串。

仅当广泛适用时为横切传输关注（如凭据、关联或错误规范化）使用拦截器。业务特定的错误映射属于功能 service/facade。

在 UI 状态中区分验证、冲突/过期状态、权限拒绝、未找到、不可用依赖和传输失败。不要将每次失败转换为空列表或通用 toast。

## 渲染与变更检测

在兼容时为任务所属的业务组件使用 `ChangeDetectionStrategy.OnPush`。通过显式事件更新 signal/不可变输入，避免将手动 `detectChanges`/`markForCheck` 作为常规状态机制。

仅在兼容 Angular 版本上使用 `@if`、`@for`、`@switch` 和 deferred view。按稳定领域标识跟踪动态集合，绝不用可变数组索引。`@defer` 需要不隐藏主要工作的加载、占位符、错误和触发行为。

将昂贵转换排除在模板之外。使用 computed 视图模型、纯 pipe、选择器或有界 service 投影。不要从模板 getter 调用 API 或修改状态。

## 表单与工作流状态

对具有验证、嵌套结构、动态行和显式提交生命周期的业务工作流使用响应式表单。类型化表单应正确建模可空性和禁用控件；`form.value` 可能省略禁用字段而 `getRawValue()` 包含它们。

客户端验证改善反馈但不替代服务端规则。映射后端字段/全局错误而不丢弃用户草稿，在相关字段变更或重新提交成功时清除过期错误。

在所属区域/控件处表示加载、空、就绪、提交中、成功、禁用和业务阻止状态。在保留重试/恢复的同时防止重复写入。

## 安全与内容

将所有浏览器代码/配置视为公开的。永远不要嵌入密钥或依赖路由守卫/UI 可见性作为服务端授权。净化或避免不受信 HTML；使用 Angular 的绑定模型，不要在没有审查过的源契约的情况下用 `DomSanitizer` 绕过安全。

保持产品界面不含运行时命令、框架说明、交付进度、验证指令和实现备注。使用已接受的 UIX 令牌/组件和业务语言。

## Verification

- 构建/类型检查受影响的 Angular 项目并编译 template/导入/provider。
- 练习任务所属的加载、空、就绪、验证、冲突、权限、不可用、提交中和成功状态。
- 验证不可变更新、稳定跟踪、草稿保留和回读协调。
- 当 API 绑定变更时测试类型化 HTTP 映射和精确错误/状态行为。
- 确认未引入敏感配置或不安全 HTML 路径。

## 交付证据

标识 Angular 组合、状态、HTTP、表单或渲染决策以及证明它的公共行为/断言。仅编译或截图不能证明 API 映射、失败恢复、状态所有权或表单语义。

## 不安全默认行为

- 将 standalone/NgModule 迁移混入不相关的功能工作。
- 为便利使用根范围可变功能状态。
- 用 effect 镜像派生 signal 状态。
- 在组件内部进行 API 调用和业务错误处理。
- 使用数组索引跟踪可变业务行。
- 使用手动变更检测补偿不清晰的所有权。
- 将浏览器环境文件视为密钥存储。

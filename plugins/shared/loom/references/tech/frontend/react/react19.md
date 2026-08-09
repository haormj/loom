# React 19 特性

仅当已接受的技术栈版本、框架运行时、类型包和任务所有权支持时才使用 React 19 API。当版本特定 API 不增加产品或交付价值时，保留已建立的仓库模式。

## 兼容性边界

确认已安装的 `react`、`react-dom`、框架、渲染器、TypeScript 类型、测试渲染器、lint 插件和打包器支持所选 API。传递依赖或外部示例不能证明应用可以使用 React 19 语义。

将框架特定的 Server Actions、缓存、路由和部署行为保留在该框架的参考中。此文件管理组件边界处的 React API 行为。

不要将 React 升级作为附带变更。如果任务拥有升级，包含依赖兼容性、已弃用 API 移除、构建/测试迁移和回滚范围。

## 用 `use()` 读取

仅在 React/框架 Suspense 语义已建立的地方使用 `use()` 在渲染期间读取 promise 或 context。在正确的所有者处创建或缓存 promise；在每次客户端渲染时创建新的请求 promise 可能持续重启工作。

在慢速区域周围放置有意义的 Suspense 回退，在被拒绝的资源周围放置错误边界。在资源键中保留目标/租户/auth context，防止过期或跨用户复用。

API 允许条件 `use(context)`，但组件分支仍须确定性，provider 必须保留清晰的缺失值策略。

## Action State

当已接受的操作/表单集成拥有 pending 和返回状态时使用 `useActionState`。定义类型化的状态联合，区分字段验证、业务拒绝、冲突/过期状态、授权失败、服务失败和成功（在适用时）。

不要返回原始异常或 provider 消息。保留成功的响应标识/版本/状态，使可见视图可以与真相来源协调。

```tsx
type SaveState =
  | { status: 'idle' }
  | { status: 'invalid'; fields: Record<string, string> }
  | { status: 'conflict'; message: string }
  | { status: 'saved'; id: string; version: number }
```

`useFormStatus` 必须位于其观察提交的表单下方。仅用它禁用冲突控件、传达 pending 状态和阻止重复命令，不使不相关的页面操作不可用。

## 乐观状态

仅当临时转换可理解、可逆且键到稳定目标时使用 `useOptimistic`。当多个写入可能重叠时分配唯一的客户端操作标识；共享 `temp` ID 不安全。

为成功、验证拒绝、授权失败、冲突、网络不确定性、乱序响应和服务端规范化值定义协调。对于破坏性或高影响操作，除非产品契约显式支持乐观行为，否则优先使用已确认状态。

乐观视觉更新永远不会授权命令。服务端仍负责认证、授权、验证和并发检查。

## Ref 作为 Prop

仅在确认 React 19 类型和仓库组件库约定后使用 ref-as-prop。在包消费者或支持的版本仍需要时保留 `forwardRef`。

为实际的命令式/DOM 集成暴露 ref，而非作为受控状态的替代。当包装器需要内部和外部引用时保留泛型元素类型、可空性和组合 ref。

## 文档元数据与其他 API

使用 React 管理的元数据或资源提示时，保持路由/框架所有权清晰，避免由现有 head manager 生成的重复标签。资源预加载/预初始化需要已测量的导航/渲染需求和正确的 origin/cross-origin 属性。

在采用前对照精确的已安装版本审查 provider 简写、清理 API 和变更的错误报告；不要仅从主版本标签推断语义。

## 渐进式表单

对于 action 支持的表单，保留语义表单提交、标签、必需约束、键盘行为、错误时焦点、拒绝后的草稿值和可见的最终回读。JavaScript 增强的 pending UX 不得使基础表单契约含糊。

## Verification

- 运行仓库 typecheck 和生产构建以证明 API 和渲染器兼容性。
- 在拥有时测试 action pending、成功、字段验证、业务拒绝、冲突、重复提交和返回回读。
- 练习乐观重叠、回滚、乱序完成和服务端规范化结果。
- 验证 `use()` 的 Suspense 回退、资源拒绝、重试和资源键隔离。
- 通过实际的组件库/compiler/测试设置检查 ref 行为。

## 交付证据

命名 React 19 API、已安装版本证明、为何现有模式不足以及 pending/error/协调/回退的行为断言。仅编译导入的 hook 不能证明框架支持或生产语义。

## 不安全默认行为

- 在已接受技术栈缺乏支持时从描述中选择 React 19 行为。
- 渲染期间创建新的客户端 promise 并传递给 `use()`。
- Action state 由一个通用错误字符串表示。
- 无稳定操作标识和回滚/协调的乐观写入。
- `useFormStatus` 在提交的表单子树之外。
- 跨仍支持 React 18 消费者的包导出 ref-as-prop。

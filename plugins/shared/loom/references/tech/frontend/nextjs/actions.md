# Next.js 服务端变更与 Actions

仅对显式拥有服务端表单/action 变更的 App Router 任务应用 Server Action 指导。除非已接受的设计将变更分配给 Server Action，否则保留已选择的后端/route-handler 架构。

## Action 边界

将 action 与其功能或聚焦的 server action 模块放在一起。每个 action 是可远程调用的服务端入口点：在 action/应用路径内认证、授权、验证、限定租户/所有权并执行业务资格。

将 `FormData` 和编程参数视为不可信。用类型化的 schema/command 解析并保留字段/全局错误形状。

```tsx
'use server'

export async function approveOrder(
  previous: ApprovalState,
  formData: FormData,
): Promise<ApprovalState> {
  const actor = await requireActor()
  const input = ApprovalSchema.safeParse(Object.fromEntries(formData))
  if (!input.success) return validationState(input.error)

  const result = await orders.approve(actor, input.data)
  if (!result.ok) return businessFailureState(result.error)

  revalidateTag(orderTag(result.value.id))
  return { kind: 'succeeded', order: result.value }
}
```

按仓库约定使用模块级 `'use server'` 或内联 action。不要创建 catch-all action 文件或暴露通用数据库操作。

## 表单与客户端状态

使用 `useActionState`、`useFormStatus` 或所选表单库在提交的表单/控件处渲染 pending、字段验证、业务冲突、禁止、不可用和成功状态。

`useFormStatus` 必须在所属表单下方渲染。阻止重复提交并在预期失败后保留草稿值。多行/表单需要稳定的目标标识；永远不要依赖可变的选定记录。

渐进增强应在已接受处工作，重定向/导航应仅在成功持久化变更后发生。

## 授权、CSRF 与 Origin

会话/cookie 变更需要框架/部署的 origin 和 CSRF 保护加显式授权。不要假设隐藏的 action ID 或同组件放置能保护 action。

在加载当前数据后验证资源所有权/租户/状态，并为并发完整性执行数据库约束。永远不要从表单字段信任 actor/租户/初始状态。

速率限制/幂等性/审计属于已接受的安全/API/应用设计，特别是对于敏感或可重复的编程调用。

## 事务与副作用

一个应用操作拥有事务和外部效果排序。不要跨 action 模块直接执行不相关的数据库写入或在文件/邮件/provider 调用期间保持事务打开。

对于文件上传，强制执行大小/数量/类型/内容/名称，流式传输到已接受的持久存储，在需要时扫描，永远不要写入临时/公共应用路径作为永久存储。

Cookie 必须使用已接受的 secure/httpOnly/sameSite/path/domain/expiry 行为，不应包含敏感载荷。

## 重新验证与回读

成功后，失效每个受影响的路径/tag/缓存/读取模型，但不更广。稳定的领域拥有标签优于临时字符串。

重新验证本身不是 UI 状态协调。确保当前表单/列表/详情接收返回或重新获取的标识/版本/状态/计数且不保持过期。

仅当导航是已接受结果时在变更后使用 `redirect`。记住 redirect 抛出控制流；不要在宽泛的 action catch 中捕获它。

## 乐观更新

仅对可预测操作使用乐观 UI，配以稳定的临时/目标标识、重复预防、回滚/冲突处理、过期响应排序和可访问的 pending/失败反馈。

不要在没有已接受设计的情况下乐观确认破坏性、高冲突、授权敏感或不可逆工作。

## 失败映射

返回可序列化的类型化预期失败。将意外失败抛出到路由错误边界/记录路径。永远不要返回原始数据库/provider/令牌/堆栈消息。

宽泛的 catch 块不得吞没 redirect/notFound 或将编程失败转换为用户验证消息。

## Verification

- 测试成功变更加 action 拥有的验证、auth、所有权、冲突、重复、不可用和意外路径。
- 验证精确的目标标识和服务端拥有字段不能被伪造。
- 证明事务/回读和受影响 tag/path 重新验证无无关失效。
- 练习多表单/行的 pending/禁用/草稿/错误/成功状态。
- 在变更处验证重定向/cookie/文件行为和限制。
- 为 action 序列化/服务端-客户端边界运行生产构建。

## 交付证据

命名 action/应用操作以及证明它的变更、auth、状态和重新验证/回读断言。仅表单调用 action 或 `revalidatePath` 调用不能证明验证、授权、持久写入、缓存一致性或重复处理。

## 不安全默认行为

- 当独立后端/handler 拥有变更时引入 Server Action。
- 原始 FormData/对象展开传递到持久化。
- 从隐藏字段信任 actor/租户/资源状态。
- 通用 catch 吞没 redirect/notFound/编程错误。
- 将重新验证视为足够的可见回读。
- 文件写入临时/公共路径和无回滚的乐观破坏性操作。

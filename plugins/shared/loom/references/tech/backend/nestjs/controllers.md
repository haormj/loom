# NestJS Controller 与 HTTP 路由

Controller 将已接受的 HTTP 契约适配到应用 service。它们拥有路由绑定、传输验证入口、状态和 header 行为以及响应映射；它们不拥有持久化或业务工作流。

## Controller 边界

按内聚的资源或能力分组端点，保留仓库的模块结构。通过构造函数注入 provider，保持 handler 足够精简，使传输行为可见，而非将领域决策隐藏在装饰器中。

显式绑定每个已接受的输入：

- `@Param` 用于路径标识符，配以匹配的 parse pipe
- query DTO 用于过滤、排序、cursor/page 控制和可选查询值
- `@Body` 配以专用的 command/input DTO
- `@Headers` 仅用于契约拥有的 header，如幂等或前置条件
- 来自已建立的 guard/decorator 边界的受信任 identity，而非来自客户端拥有的 body 字段

当 Nest 装饰器和 DTO 能表达契约时，不要将原始 `Request`、`req.body` 或任意 query 对象传入 service。仅对 Nest 抽象无法清晰表示的传输行为使用平台 request/response 对象，如流或 provider 特定的回调。

## 路由组合

从已配置的全局前缀、controller 路径和 method 路径组合有效路径。精确匹配已接受的接口，包括参数名、trailing-slash 约定和嵌套资源归属。

对原始路由值使用 `ParseUUIDPipe`、`ParseIntPipe`、enum pipe 或聚焦的自定义 pipe。解析失败应产生仓库的验证 envelope，而非带着无效标识符到达 service。

嵌套路由应在 service/query 边界建立父级归属。同时接收 `projectId` 和 `itemId` 并不证明 item 属于该 project。

不要从框架约定添加全局前缀或 URI/header 版本控制。仅当已接受的 API 契约和现有 bootstrap 拥有它时才配置。

```typescript
@Controller("orders")
export class OrdersController {
  constructor(private readonly orders: OrdersService) {}

  @Post()
  async create(
    @Body() input: CreateOrderDto,
    @CurrentActor() actor: Actor,
  ): Promise<OrderResponseDto> {
    const created = await this.orders.create(actor, input);
    return OrderResponseDto.from(created);
  }

  @Get(":id")
  findOne(@Param("id", ParseUUIDPipe) id: string) {
    return this.orders.findOne(id);
  }
}
```

上方的自定义 identity 装饰器表示已选择的认证边界；不要为未认证接口创建它。

## 状态、Header 与响应

仅当匹配契约时使用 Nest 默认值。为非默认成功语义应用 `@HttpCode`，保持 `204` 响应无 body。仅当声明或已建立时设置 location、分页、缓存、重试、幂等、ETag 或前置条件 header。

返回显式响应 DTO 或映射的传输对象。当 ORM 实体暴露持久化字段、懒关系、凭证、内部状态或 provider 特定值时，不要返回它们。

保持预期的失败映射在各 controller 间稳定。使用类型化的 application/domain 失败加现有异常 filter，而非每个 handler 中的宽泛 `try/catch` 块。保留验证、not found、冲突、认证、授权、限流和依赖不可用之间的区分。

## Pipe、Guard、Interceptor 与 Filter

按职责选择 Nest 扩展点：

| 关注点 | 边界 |
|---|---|
| 解析和传输验证 | Pipe |
| 认证或操作授权 | Guard |
| 横切请求/响应行为 | Interceptor |
| 异常到响应转换 | 异常 filter |
| 可复用应用行为 | Provider/service |

仅当真正是应用范围时才在 bootstrap/module provider 中应用全局行为。Method/controller 覆盖必须保持可见，不得静默绕过全局安全、验证或序列化行为。

Interceptor 可以塑造成功响应或添加遥测，但不应遮蔽端点特定的状态/body 契约。异常 filter 不得泄漏堆栈跟踪、数据库错误、token 或内部 provider 消息。

## OpenAPI 对齐

当仓库发布 OpenAPI 时，保持操作、参数、请求、响应和错误元数据与已接受的接口对齐。优先使用 DTO 驱动的 schema 和针对状态变体的显式响应装饰器。不要发明仅文档字段或使运行时验证偏离生成的 schema。

Swagger bootstrap、服务器 URL、auth scheme 和文档暴露是应用级决策。除非任务拥有该设置，否则不要将它们添加到功能 controller。

## 验证

- 编译所属模块，使路由元数据和依赖注入得以解析。
- 执行真实 Nest 应用以验证路由组合、全局前缀/pipe/guard/filter 和适配器行为。
- 断言成功状态/body/header 以及拥有的验证、not-found、冲突和授权分支。
- 验证路径和 query 解析、嵌套资源归属、分页边界和响应字段排除。
- 仅当任务变更已发布接口时生成或检查 OpenAPI。

## 交付证据

命名 controller 和有效路由，然后标识证明绑定、状态、响应映射和相关横切行为的 HTTP 断言。Controller 单元调用不能证明全局 pipe、guard、interceptor、filter、前缀或适配器行为。

## 不安全默认

- 业务规则或 repository 调用分散在 controller handler 中。
- 原始请求 payload 转发给 provider。
- `@Res()` 用于普通 JSON 响应，禁用 Nest 响应处理。
- 在没有已接受契约的情况下引入全局前缀/版本控制。
- ORM 实体作为公共响应模型返回。
- OpenAPI 装饰器被当作运行时验证。

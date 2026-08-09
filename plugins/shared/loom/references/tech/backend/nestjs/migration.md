# Express 到 NestJS 迁移

使用 NestJS 作为目标实现，在有意契约变更被单独批准之前保留已接受的 Express 行为。迁移是兼容性操作，不是重新设计路由、错误、认证、持久化或部署约定的许可。

## 建立对齐基线

在移动代码之前盘点行为：

- 挂载的 router 前缀和有效 method/path 对
- path/query/body 解析和默认值
- 验证规则和错误 envelope
- 中间件顺序和短路行为
- 认证、授权、归属和公共例外
- 成功/错误状态、header、cookie、重定向和响应形态
- 事务、外部效果、事件和后台工作排序
- 配置 key、启动行为、健康/就绪和关闭
- 当前证明这些行为的测试

创建逐路由对齐矩阵，包含旧入口点、Nest 目标、必需行为、验证和有意差异。不要仅从 Express handler body 推断契约；包含 router 挂载和应用中间件。

## 慎重映射职责

| Express 职责 | NestJS 目标 |
|---|---|
| `express.Router` 和 handler 绑定 | Controller/module |
| 原始请求解析 | Param/query pipe |
| 结构化输入验证 | DTO 加 `ValidationPipe` |
| 认证门 | Strategy/guard |
| 操作/role 授权 | Guard/策略加 service/query 范围限定 |
| 请求/响应横切行为 | Interceptor 或中间件 |
| 错误中间件 | 异常 filter/映射边界 |
| 手动构造的 service/client | 注册的 provider 和注入 token |
| 进程 bootstrap/配置 | Nest bootstrap/config module |

不要机械地将每个 Express 中间件转换为 Nest 中间件。根据职责和生命周期选择 guard、pipe、interceptor、filter 或 provider。

## 创建共存边界

对于增量迁移，为每个路径选择显式的路由归属。Express 和 Nest 不得意外地同时处理同一路由，proxy 前缀不得双重剥离或重复 `/api`。

在可行的地方共享契约和稳定端口，但不要让 Nest provider 通过应用层导入 Express request/response 对象。如果两个运行时共存，定义 identity、关联、错误、配置、数据库事务和关闭如何跨边界传播。

优先可以独立验证和切换的垂直路由/能力切片。避免长期半转换状态，其中 controller 使用 Nest 而 service 仍是由 Express 模块构造的隐藏 singleton。

## 转换模块与依赖注入

创建内聚的功能模块，用显式 import 和 export 注册 controller/provider。用构造函数注入替换模块级 singleton 和 `new` 构造。

为 repository/外部 client 使用稳定的 provider token 并保留其生命周期。在 provider 构造之前验证配置。通过更改归属解决依赖循环；不要将 `forwardRef` 规范化为迁移架构。

当移动持久化代码时，保留所选的 ORM/client 和迁移历史。NestJS 不需要 TypeORM。除非任务显式拥有两者变更，否则不要在迁移 Web 框架时重写存储。

## 保留 HTTP 与错误行为

在全局前缀和 controller 组合之后验证有效路由。保持状态、header、cookie、重定向、分页和序列化字段兼容。

通过一条生产 bootstrap 路径配置全局 pipe、filter、guard 和 interceptor，并在 HTTP 测试中复用。默认 Nest 验证/错误 payload 可能与 Express 不同；当需要兼容性时慎重映射。

保留适用于迁移路由的受信任 proxy、CORS、CSRF、body 大小、raw-body/webhook、multipart、压缩和限流行为。Express 和 Fastify 之间的适配器差异必须显式。

## 保留安全与副作用

将 auth 中间件映射到所选 strategy/guard 并重现公共例外、role/permission 检查、owner/tenant 范围限定和错误资源披露行为。

保留事务和副作用顺序。先提交再发出事件的 Express handler 不能迁移到在事务内发出事件的 provider 而没有已接受变更。为迁移的回调和 job 定义重试/幂等行为。

切勿仅为对齐而保留明文密钥、临时 token 解析或错误细节泄漏；将必需的安全修正记录为带测试的显式迁移变更。

## 切换与移除

在切换路由组之前，对两个实现或捕获的契约 fixture 运行对齐检查。确认流量路由、配置、健康/就绪、metric/日志关联和回滚行为。

切换后，在所属范围内移除过时的 router、中间件、手动构造函数、重复配置、测试和依赖。保留两个实现活跃不是完成的迁移。

## 验证

- 为每个迁移的路由组证明成功和每个拥有的失败/auth 分支。
- 在需要对齐的地方比较精确的状态、body、header、验证和错误行为。
- 编译具有生产代表性的 Nest 模块并通过 HTTP 执行全局 bootstrap。
- 当涉及时验证 provider 注入、数据库持久性/回滚、外部效果排序和关闭行为。
- 测试共存/proxy 路径使前缀归属无歧义。
- 用已接受需求和证据记录每个有意差异。

## 交付证据

引用对齐矩阵条目并标识证明等价性的旧/新请求或契约断言。Nest 编译、生成的文件或一个成功的路由不能证明中间件、安全、错误、持久化或切换对齐。

## 不安全默认

- 从文字描述而非结构化任务操作激活迁移。
- 使用端点计数或团队规模选择架构。
- 每个 Express 中间件翻译为 Nest 中间件。
- 因为 Nest 示例使用 TypeORM 就引入它。
- 切换期间 Express 和 Nest 同时拥有同一路由。
- 假设默认 Nest 错误/验证 payload 兼容。
- 接受后旧 router 和重复配置保持活跃。

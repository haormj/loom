# NestJS 测试模式

选择能证明变更行为的最窄测试边界。使用仓库已建立的 runner 和 Nest bootstrap helper；不要因为示例使用了就替换 Jest 为另一个 runner 或添加 E2E 基础设施。

## 测试边界选择

| 要证明的行为 | 首选边界 |
|---|---|
| 纯规则或 mapper | 纯 TypeScript 单元测试 |
| Service 编排 | 带已拥有端口 mock/fake 的 `TestingModule` |
| Provider token/模块配置 | 聚焦的模块编译测试 |
| Pipe、guard、interceptor、filter | 直接单元测试加注册有影响时的 HTTP 测试 |
| 路由/全局 bootstrap 行为 | Nest 应用 HTTP 集成/E2E 测试 |
| ORM 约束/事务/查询 | 所选 provider 的集成测试 |

Controller 方法调用仅证明委托和映射。它们不执行路由元数据、全局 pipe、guard、interceptor、异常 filter、中间件、serializer 或 HTTP 适配器。

## TestingModule 构造

注册被测单元及其实际注入 token。仅 mock 已拥有的边界，如 repository、HTTP client、队列、clock、mail、对象存储和事件发布者。不要 mock 被测试的 service 方法或策略。

使用类型化 mock 或 protocol fake，使签名漂移导致编译失败。在案例之间重置调用历史和实现状态；模块范围的可变 mock 可能泄漏结果和顺序依赖。

仅当被覆盖的关注在测试声明之外时才覆盖 provider/guard。认证测试必须执行真实所选 guard/strategy；controller 契约测试可通过已建立的测试 harness 提供稳定的已认证 principal。

当 import、export、动态模块选项、provider token、scope 或 `forwardRef` 行为变更时编译代表性模块。最小单元模块可能意外隐藏缺失的生产 import。

## HTTP 应用保真度

尽可能通过与生产相同的 bootstrap/配置函数创建测试应用。否则显式镜像所有契约相关的全局配置：

- 全局前缀和适配器
- `ValidationPipe` 选项
- guard 和 public 元数据
- interceptor 和序列化
- 异常 filter 和错误 envelope
- CORS、cookie、关闭 hook 和版本控制（仅当拥有时）

在 Supertest 请求之前初始化并始终关闭应用。除非依赖需要否则避免绑定网络端口；`app.getHttpServer()` 足以进行普通 HTTP E2E 测试。

```typescript
const moduleRef = await Test.createTestingModule({ imports: [AppModule] }).compile();
const app = moduleRef.createNestApplication();
configureApplication(app); // 同样的全局 pipe、filter、guard 和前缀
await app.init();

await request(app.getHttpServer())
  .post("/api/orders")
  .send(validInput)
  .expect(201);

await app.close();
```

## 行为覆盖

断言精确的已接受状态、body、header 和持久副作用。覆盖相关的失败类别：格式错误输入、not found、冲突/并发、未认证、禁止/错误归属者、依赖失败和回滚。

对于分页/过滤，断言确定性排序、边界、空结果和元数据。对于敏感响应，在真实序列化路径上断言禁止字段不存在。

状态转换测试应证明结果状态和被拒绝的转换。外部效果测试仅当调用排序/幂等是操作契约的一部分时才证明它们。

## 持久化与事务

Repository mock 不能证明 ORM 映射、数据库约束、事务回滚、锁、迁移或 provider 特定值。对这些声明使用已接受的 provider，并通过仓库的 fixture/事务策略隔离每个测试的数据。

不要静默地将 SQLite 替代 PostgreSQL/MySQL 涉及 JSON、小数、collation、索引、锁或生成值的行为。将测试容器或共享服务限定在拥有此类证据的套件中。

## 时间、Async 工作与清理

仅在设计支持 fake timer 的代码周围使用 fake timer 并恢复真实 timer。优先注入 clock 和确定性 ID/随机性用于业务结果。

Await 异步断言和后台完成信号。不要使用任意 sleep 让队列/事件测试通过。关闭数据库 client、队列、消费者、服务器和应用，使开放 handle 警告保持可操作。

对于基于 Observable 的 provider/interceptor，断言完成/错误行为并在需要时取消订阅；不要留下悬挂的流。

## 验证命令

先运行变更的测试文件或项目 target。当共享模块元数据、bootstrap 或 provider 契约变更时运行所属包的聚焦 typecheck/lint/test target。使用仓库的 E2E 命令/配置，而非假设 `npm run test:e2e` 存在。

## 交付证据

记录测试边界、场景、命令和有意义的断言。通过的套件计数本身不能证明全局 Nest 配置、拒绝授权、持久持久化、事务行为或资源清理。

## 不安全默认

- 为每个纯函数或单元分支使用 E2E 测试。
- Controller 单元调用声称为 HTTP 契约证据。
- 在唯一的授权测试中替换真实 guard。
- 共享可变 mock 和测试顺序依赖。
- 测试 bootstrap 偏离生产全局配置。
- ORM mock 声称为事务或约束证据。
- 任意 sleep、未 await 的 promise 或未关闭的应用。

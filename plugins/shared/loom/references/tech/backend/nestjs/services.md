# NestJS Service、模块与依赖注入

Nest provider 实现应用行为并连接显式模块依赖。保留仓库的 domain 和适配器边界，而非将每个类变成通用 service 或每个模块变成共享容器。

## Service 职责

将业务规则、状态转换、事务编排、repository 调用和外部端口协调保持在 controller 之外。Service 方法应表示一个应用操作，具有显式的输入、输出、失败模式和副作用顺序。

当仓库已分离 domain/application 层时，不要将应用行为耦合到 HTTP DTO 或 Nest 异常。在 controller 边界转换传输模型，通过已建立的异常层映射 domain 失败。

当 provider 拥有不同生命周期或依赖时才拆分它们，而非仅因为文件变大。避免不增加策略且复制已有 repository/client 抽象的传递 service。

## 模块边界

功能模块拥有内聚的 controller 和 provider。导入导出所需能力的模块，仅导出有意被其他地方消费的 provider。

对普通功能依赖避免全局模块。它们隐藏归属并使测试意外编译通过。将循环依赖检测为边界问题；`forwardRef` 仅当两个方向确实不可避免且有文档记录时才是临时逃生舱。

动态模块适合可配置的基础设施适配器或可复用的平台能力。保持 `forRoot`/`forRootAsync` 配置类型化、已验证，并与按功能注册分开。

## Provider Token 与工厂

使用构造函数注入。类 token 适合具体的内部 provider；symbol 或稳定常量对端口和非类值比自由格式字符串更安全。

工厂 provider 必须在 `inject` 中声明每个依赖，在构造客户端之前验证配置，并为连接或 worker 定义生命周期/清理行为。不要在功能方法中读取分散的 `process.env` 值。

对别名使用 `useExisting`，对可替换实现使用 `useClass`，对稳定值/测试替身使用 `useValue`，对依赖配置或其他 provider 的构造使用 `useFactory`。在所属模块中保持选择可见。

```typescript
export const ORDER_REPOSITORY = Symbol("ORDER_REPOSITORY");

@Module({
  providers: [
    OrdersService,
    { provide: ORDER_REPOSITORY, useClass: TypeOrmOrderRepository },
  ],
  exports: [OrdersService],
})
export class OrdersModule {}
```

适配器类和 token 应遵循所选的持久化技术栈和已有命名；示例演示的是端口边界，而非引入 TypeORM 的要求。

## Provider 范围与上下文

Singleton 是默认值，不应保留请求特定的可变状态。请求范围通过依赖图传播并可能增加分配/延迟；仅当上下文无法显式传递或通过已建立的上下文边界提供时才使用它。

Transient provider 每次注入创建新实例，不是共享状态 bug 的通用修复。在没有 HTTP 请求上下文的 gateway、定时 job、队列消费者和持久 worker 中验证 scope 交互。

## 持久化与事务

遵循所选的持久化适配器（TypeORM、Prisma、MikroORM、repository 端口或其他已建立的技术栈）。不要从 NestJS 示例引入 ORM。

将多写不变量保持在一个显式事务边界内。通过操作传递事务感知的 repository/client，而非混合事务性和全局客户端。翻译已知的唯一/约束/并发失败，而不在业务代码中匹配不透明的 provider 字符串。

不要跨 email、HTTP、队列或文件操作保持数据库事务打开。对于持久的跨边界效果，遵循已接受的 outbox、job 或补偿设计。

## 外部效果与失败映射

在仓库的适配器/端口约定之后注入外部客户端。从已接受的架构决策定义超时、取消、重试、幂等和部分失败行为；不要习惯性地对非幂等写入添加重试。

慎重地排序持久状态和外部效果。所选的应用可观测性参考拥有单一的支持关联的日志边界；不要在每个 provider 层记录并重新抛出同一异常。

根据已有的分层使用类型化的 domain/application 失败或聚焦的 Nest 异常。切勿暴露原始 ORM/client 错误、密钥或堆栈跟踪。

## 生命周期与后台 Provider

仅对拥有的启动/关闭行为使用 Nest 生命周期 hook。启动应对强制依赖明确失败，不应在测试或热重载期间启动重复的定时器/worker。

在关闭 hook 中关闭 client、消费者和应用资源。定时或队列驱动的 provider 需要幂等 handler 和显式并发/错误行为，因为请求范围的假设不适用。

## 验证

- 当配置变更时，用真实 provider token 和模块导入编译所属 `TestingModule`。
- 在 repository 和外部端口边界使用类型化 mock/fake 对应用决策进行单元测试。
- 证明操作拥有的 not-found、冲突、授权、事务回滚和外部失败行为。
- 对 mock 无法证明的事务、约束或查询行为执行所选持久化 provider。
- 当变更时验证 provider scope、动态模块配置、生命周期清理和开放 handle 的缺失。
- 为模块元数据和注入 token 错误运行聚焦的 lint/typecheck/build。

## 交付证据

命名 service/provider 操作、其注入的边界以及证明业务或事务结果的断言。编译的模块证明 DI 配置，而非状态转换、持久化持久性或外部效果排序。

## 不安全默认

- 应用代码中手动 `new Service()` 构造。
- 功能 provider 在没有消费者契约的情况下导出或标记为全局。
- 使用 `forwardRef` 而非解决循环归属模型。
- 为方便而引入请求范围 provider。
- 从示例而非已接受技术栈复制 ORM 选择。
- 数据库事务内执行外部调用。
- 每个 provider 层的宽泛捕获/记录/重新抛出块。

# ASP.NET Core 应用架构

实现架构契约中已接受的架构风格和模块边界。Clean Architecture、vertical slice、CQRS 和 MediatR 是选项，而非框架默认；除非任务拥有结构性决策，否则保留仓库已有的架构形态。

## 依赖方向

对于分层架构，依赖指向 application/domain 策略：

| 边界 | 拥有 | 不得拥有 |
|---|---|---|
| Domain | 实体、值对象、不变量、领域失败 | ASP.NET、EF Core、传输 DTO、外部客户端 |
| Application | 用例、端口、编排、事务意图 | HTTP 绑定、provider 配置 |
| Infrastructure | EF/client/message/file 适配器 | 隐藏在适配器中的产品策略 |
| Web/API | 传输绑定、identity 上下文、结果映射 | 持久化查询和业务工作流 |

项目引用和命名空间应强制执行已接受的依赖方向。Web 项目中名为 `Domain` 的文件夹本身并不构成架构边界。

对于模块化单体或 vertical-slice 设计，同样要明确能力归属：一个 slice/module 拥有其命令、查询、规则、存储适配器和公共契约。共享项目应包含稳定的横切基础设施，而非被每个功能导入的杂项代码。

## 用例边界

用应用 service/handler 表示每个变更或查询操作，使其输入、输出、取消、授权上下文、失败和事务责任清晰可见。

```csharp
public sealed record ApproveOrder(Guid OrderId, Guid ActorId);

public sealed class ApproveOrderHandler(
    IOrderRepository orders,
    IUnitOfWork unitOfWork,
    IClock clock)
{
    public async Task<OrderResult> Handle(ApproveOrder command, CancellationToken ct)
    {
        var order = await orders.GetForUpdate(command.OrderId, ct)
            ?? throw new OrderNotFound(command.OrderId);
        order.Approve(command.ActorId, clock.UtcNow);
        await unitOfWork.SaveChangesAsync(ct);
        return OrderResult.From(order);
    }
}
```

该示例展示了一个明确的应用边界，并非要求在已接受设计直接使用 `DbContext` 时仍然创建 repository/unit-of-work 包装器。

## CQRS 与 MediatR

当读和写的职责、授权、验证、事务或扩展性确实不同时，使用命令/查询分离。不要为了形式化的 CQRS 重复创建相同的 DTO/handler。

仅当仓库选择了 MediatR 且 pipeline behavior 消除了真实的横切重复时才使用它。从正确的程序集注册 handler，并保持 behavior 顺序的慎重性。验证、授权、事务、幂等和日志 behavior 不得各自执行 handler 或隐藏副作用。

不要仅因为 query handler 很薄就在每个端点放 EF 查询。保持查询归属与所选的 application/data 边界一致，并直接投影到读模型。

## 依赖注入与组合

在靠近所属项目的显式组合扩展中注册应用服务和适配器实现。请求/工作单元服务使用 scoped 生命周期，仅对线程安全无状态/共享资源使用 singleton，对廉价的独立组件使用 transient。

避免在业务代码中通过 `IServiceProvider` 进行 service locator 访问。当运行时选择是已接受设计的一部分时，工厂是合适的；注入窄接口的工厂而非容器。

在启动时验证必需的选项，并将基础设施细节排除在 domain/application 构造函数之外。防止循环项目引用，而非用共享工具程序集来掩盖。

## 验证与失败边界

传输验证处理格式错误的 HTTP 输入。应用验证处理用例前置条件和授权。领域对象强制执行必须跨入口点保持的不变量。数据库约束是最终的并发完整性边界。

使用类型化的 domain/application 失败，并在 API 边界处转换一次。不要让领域异常继承 ASP.NET HTTP 异常类型，也不要让 EF/provider 异常泄漏到 handler 响应中。

## 事务与副作用

一个用例拥有其持久化不变量的事务。Repository 不应独立地静默提交。除非已接受设计明确协调，否则将 HTTP/email/queue/file 调用保持在数据库事务之外。

对于持久事件，使用已接受的 outbox 或消息边界，并在可能重复投递时使 handler 幂等。进程内 MediatR notification 不是持久事件总线。

## 验证

- 构建受影响的项目图，以捕获非法/缺失的引用和 DI 注册错误。
- 在不依赖 ASP.NET 或 EF 基础设施的情况下测试领域不变量。
- 测试 handler/service 的授权、状态转换、事务、取消和失败映射。
- 当注册或 pipeline behavior 变更时，编译具有生产代表性的 service provider。
- 仅当仓库使用架构依赖测试且该决策足够重要需要机制化强制时，才添加架构依赖测试。
- 当端点也属于任务拥有时，单独验证 HTTP 映射。

## 交付证据

标识已接受的决策、所属模块/项目、变更的依赖方向，以及证明它的测试/构建证据。命名空间布局、生成的项目树或通过的端点本身不能证明架构边界或事务归属。

## 不安全默认

- 在没有已接受架构决策的情况下引入 Clean Architecture 或 MediatR。
- Domain 类型依赖 ASP.NET、EF Core 或传输 DTO。
- 共享项目成为功能逻辑的垃圾场。
- 每个简单的 getter 一个 handler 类，没有有意义的分离。
- `IServiceProvider` 被用作 service locator。
- 在一个用例中 repository 独立提交。
- 将进程内 notification 声称为持久集成事件。

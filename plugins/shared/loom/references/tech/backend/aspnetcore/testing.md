# ASP.NET Core 测试

使用仓库已选择的测试框架和能证明任务的最窄边界。ASP.NET Core 工作并不自动需要每种测试类型；框架测试指引仅对拥有测试实现的任务选择。

## 证明边界

| 声明 | 合适的证明 |
|---|---|
| 领域不变量/值对象 | 纯单元测试 |
| 应用 handler/编排 | 使用已拥有端口 mock/fake 的单元测试 |
| EF 映射/查询/事务 | 已选 provider 的集成测试 |
| DI/options 注册 | 聚焦的 service-provider/host 启动测试 |
| 路由、中间件、过滤器、认证、序列化 | `WebApplicationFactory<Program>` HTTP 测试 |
| 发布的运行时/AOT 行为 | 发布产物的冒烟/集成测试 |

直接调用 Minimal API delegate 或控制器方法不能证明路由绑定、全局验证、中间件、异常 handler、授权、响应序列化或 host 配置。

## 单元与 Handler 测试

当框架 DI 无关时，直接构造 domain/application 组件。Mock repository、clock、queue、HTTP 端口、identity 和其他已拥有的边界；不要 mock 被测试的规则/handler。

断言返回值以及状态转换、调用、禁止副作用的缺失、取消和类型化失败。在结果依赖时间、ID 和随机性时，通过显式端口保持它们的确定性。

使用测试数据 builder/factory 创建可读的有效默认值，仅覆盖场景相关字段。避免一个巨大的共享 fixture 因其变更造成顺序依赖。

## WebApplicationFactory 保真度

从实际入口点派生聚焦的 factory，保留生产服务注册、中间件、route group、JSON 选项、验证、认证和异常处理。仅覆盖测试声明之外的外部/运行时依赖。

```csharp
public sealed class ApiFactory : WebApplicationFactory<Program>
{
    protected override void ConfigureWebHost(IWebHostBuilder builder) =>
        builder.ConfigureTestServices(services =>
        {
            services.RemoveAll<IClock>();
            services.AddSingleton<IClock>(new FrozenClock(TestTime.UtcNow));
        });
}
```

使用 factory 客户端断言精确的状态、header、problem detail、JSON 形态和持久效果。释放客户端/factory，避免测试间共享可变的 host/数据库状态。

不要在唯一的安全测试中替换认证。非认证行为可使用稳定的测试 scheme，而专门的认证测试执行所选的真实 scheme/policy 和拒绝路径。

## EF Core 测试

测试约束、SQL 翻译、小数、JSON、collation、迁移、事务、锁或并发时使用已接受的 provider。EF InMemory 的行为不像关系数据库，SQLite 也不能通用替代 SQL Server/PostgreSQL/MySQL。

通过事务、schema/数据库或与 provider 兼容的确定性清理来隔离每个测试的数据库状态。断言数据库回读和回滚，而不仅仅是内存中的 tracked 实体。

仅对拥有 provider 保真度的测试套件使用容器/共享依赖，并按照仓库 harness 复用基础设施，不跨用例泄漏数据。

## 配置、健康与 Hosted Service

Host 启动测试应覆盖有效选项和缺失/无效的强制设置。健康测试应证明 liveness/readiness 分类和依赖状态转换。

对于 `BackgroundService`，创建确定性的完成信号和取消。不要等待任意的墙钟延迟。在拥有的边界验证 scope 创建、有界重试、幂等、关闭和资源清理。

测试后检查是否有开放的服务器、数据库连接、定时器、消费者和未观察的 task。开放 handle/资源警告是需要理解的失败，而非全局抑制的噪声。

## HTTP 契约覆盖

对于变更的操作，覆盖相关的成功、格式错误输入、not found、冲突/并发、未认证、禁止/错误归属者、依赖不可用和取消行为集合。

当声明时断言分页边界/顺序、条件 header、location、content type 和敏感字段排除。将列表隔离与 detail/mutation 授权分开测试。

当仓库使用 OpenAPI 快照时可以检测契约漂移，但它们不替代行为测试。

## 验证命令

先运行变更的测试项目/过滤器，例如 `dotnet test tests/Orders.Tests --filter FullyQualifiedName~ApproveOrder`。当共享契约、DI、中间件或项目引用变更时，运行所属项目/solution 构建或聚焦测试套件。保留仓库配置和 target framework 标志。

## 交付证据

记录测试边界、场景、命令和有意义的断言。仅凭通过的 `dotnet test` 计数不能证明真实中间件、授权、provider 语义、启动验证、迁移安全或资源清理。

## 不安全默认

- 为每个纯规则添加集成测试。
- 将直接端点调用声称为 HTTP 管道证据。
- 在唯一的授权测试中替换真实认证。
- 将 EF InMemory/SQLite 声称为已选 provider 语义的证明。
- 共享可变的 factory/数据库状态和顺序依赖的测试。
- 对 hosted/background 行为使用任意 sleep。
- 将 OpenAPI 快照用作唯一的路由行为证明。

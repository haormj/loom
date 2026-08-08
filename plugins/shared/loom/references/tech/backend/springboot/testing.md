# Spring Boot 测试

使用能证明任务拥有行为的最小 Spring 测试边界。此参考不使每个 Spring 任务成为测试任务；仅当已接受的任务归属显式包含测试实现时才适用。

## 测试边界矩阵

| 被测行为 | 首选边界 | 证明什么 |
|---|---|---|
| 纯 domain/service 规则 | 仅当存在协作者时使用带 Mockito/fake 的 JUnit 5 | 分支、状态规则、协作者契约，无需 Spring 启动 |
| MVC controller/advice | `@WebMvcTest` + `MockMvc` | 路由、绑定、验证、序列化、状态、错误转换 |
| WebFlux controller | `@WebFluxTest` + `WebTestClient` | 响应式路由、body、状态、错误路径 |
| JPA repository/映射 | `@DataJpaTest` | Repository 查询、映射、约束、fetch 行为 |
| 安全 filter/方法 | Web slice 或聚焦安全测试 | 允许、未认证、禁止、CSRF/CORS 行为 |
| 配置属性 | Binder 测试、context runner 或聚焦 context | 默认值、验证、无效/缺失配置 |
| 跨 bean 工作流 | 带目标协作者的 `@SpringBootTest` | 配置、事务、迁移、安全、运行时集成 |
| Provider 特定持久化 | Testcontainers 或仓库标准真实 provider | Dialect、迁移、约束、原生查询、锁 |

不要为纯计算或映射代码使用 `@SpringBootTest`。不要 mock 被测类、Spring 内部、JPA 实体或值对象。

## MVC 与 WebFlux Slice

MVC 测试应验证已接受的传输行为，而不仅是 service 方法被调用。

```java
@WebMvcTest(OrderController.class)
@Import(ApiExceptionHandler.class)
class OrderControllerTest {
    @Autowired MockMvc mvc;
    @MockitoBean OrderApplicationService orders;

    @Test
    void invalidRequestReturnsValidationProblem() throws Exception {
        mvc.perform(post("/api/orders")
                .contentType(MediaType.APPLICATION_JSON)
                .content("""{"supplierName":"","lines":[]}"""))
            .andExpect(status().isBadRequest())
            .andExpect(jsonPath("$.code").value("VALIDATION_ERROR"));
    }
}
```

使用仓库 Spring Boot/Spring Framework 版本支持的 mocking 注解。较新技术栈可使用 `@MockitoBean`；已有项目可能仍使用 `@MockBean`。不要仅为采用示例中的注解而重写测试技术栈。

导入或包含 slice 所需的 controller advice、转换器、JSON 模块和安全配置。当任务拥有受保护行为时避免禁用所有 filter。

## 数据测试

`@DataJpaTest` 通常默认使用嵌入式数据库。仅对 provider 中立的映射/查询行为保留它。测试以下内容时禁用替换并使用所选 provider：

- 迁移 SQL
- 原生查询
- provider 特定列或 enum 类型
- 生成 ID 和默认值
- 锁、隔离或事务语义
- 大小写敏感、collation、JSON、数组、全文或时间戳行为

当迁移是运行时的一部分时在集成路径中使用 Flyway/Liquibase。`ddl-auto=create-drop` 不证明迁移。

同时测试 repository 结果和持久化状态。对于写入工作流，当提交行为有影响时验证提交/回读；测试级回滚可能隐藏 post-commit 事件、约束时序或事务同步。

当已接受的数据访问选择是 MyBatis-Plus 时，在最小合适测试边界中使用仓库的 Mapper 和 `SqlSessionFactory` 配置。验证 mapper 扫描、XML namespace 解析、wrapper SQL、分页、逻辑删除、乐观锁、TypeHandler、interceptor scope 和 provider 特定行为。测试 SQL、插件、迁移、锁、JSON、collation 或生成默认值时不要用 H2 替换所选 provider。

## Testcontainers

复用仓库的容器生命周期和所选 provider。支持时 Spring Boot service connection 适用；`@DynamicPropertySource` 对显式属性绑定仍有效。

在项目配置而非此参考中固定兼容的 provider 镜像。不要静默用 H2 替换 MySQL、PostgreSQL、SQL Server、Oracle 或文件数据库。

容器启动失败是运行时不可用的环境证据。针对运行中容器的 SQL 断言失败仍是代码/测试失败。

## 安全测试

使用真实的 role、authority、CSRF token 和 claim。通过以下覆盖变更的策略：

- 允许的调用者
- 缺失认证
- 权限不足
- 适用时的资源归属拒绝
- 稳定的 `401`/`403` 响应形态

不要在禁用所有 filter 的情况下仅测试受保护端点。避免为测试方便使用生产默认用户或凭证。

## 配置与运行时测试

使用 `ApplicationContextRunner`、binder 测试或聚焦的 `@SpringBootTest` 证明：

- `@ConfigurationProperties` 默认值
- 缺失/无效值的验证
- 条件 bean 选择
- profile 特定行为
- 不意外依赖外部基础设施的启动

通过注入的 `Clock` 固定时间。对异步行为使用同步原语、Awaitility、latch、虚拟时间或完成信号；不要使用任意 sleep 作为断言机制。

## 集成边界

用 MockWebServer、WireMock 或仓库等价物测试出站 HTTP 客户端。在不调用真实共享服务的情况下证明序列化、认证/header 传播、provider 错误、超时和重试分类。

对于缓存行为，证明键分离、命中/未命中、失效和回退。对于弹性，证明尝试和终态结果而不等待生产持续时间的定时器。

## 隔离与可维护性

- 保持 fixture 接近业务场景，避免共享可变全局状态。
- 确定性地重置外部资源；不依赖测试执行顺序。
- 使用基于行为和结果描述的测试名。
- 通过避免不必要的按测试配置变更来保留 Spring 上下文缓存。
- 当旧行为可重现时为已修复缺陷添加回归测试。
- 不要禁用测试或弱化断言以使套件通过。

此处不施加通用覆盖率百分比。覆盖率有助于发现未测试路径，但不能替代契约、失败、持久化和安全断言。

## Verification Focus

有用的测试证据标识：

- 所选测试边界以及为何匹配变更行为
- 覆盖的精确成功和阻塞/失败路径
- 用于持久化行为的 provider 和迁移路径
- 涉及的 Spring context 或 slice 配置
- 目标构建/测试命令和结果
- 环境阻塞与代码失败分离

## 不安全默认

- 为每个单元启动完整应用上下文。
- 使用 H2 作为 provider 特定 SQL 的证明。
- 在唯一的 controller 测试中禁用安全 filter。
- 依赖测试回滚证明 post-commit 行为。
- 异步测试的固定 sleep。
- 没有风险理由的硬编码覆盖率目标。

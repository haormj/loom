# Java Spring 容器基础

此参考拥有 Spring 应用间共享的 Spring 容器机制：bean 构造、依赖注入、代理、组件扫描、包根和生命周期。Spring Boot web、数据、安全、运行时、测试、缓存、异步、集成和可观测性行为属于选中的后端参考。

## When To Use

当 Java 实现工作拥有 Spring bean 构造、依赖注入、组件发现、代理支持的注解、配置类或受管生命周期时使用此参考。它适用于 Spring Framework 和 Spring Boot 应用。

不要用作框架特定传输、持久化、安全、测试、运行时、缓存、消息或集成指导的替代。这些关注需要其任务拥有的后端参考。

## Implementation Focus

### Package And Scan Boundary

将应用/引导类保持在拥有的组件包之上。使用现有的生产包根或构建元数据。对于新仓库，使用确认的命名空间、`app.<project_slug>` 或 `app.generated`；永远不要在教程根下创建生产代码。

避免宽泛的 `scanBasePackages` 或 repository/entity 扫描覆盖来隐藏不连贯的包布局。新模块应通过正常包树保持可发现，除非有意作为配置导入。

### Constructor Injection

为必需依赖使用构造函数注入。它使不变式可见、支持不可变字段并允许普通单元构造。

```java
@Service
final class OrderPolicy {
    private final Clock clock;
    private final EligibilityRules rules;

    OrderPolicy(Clock clock, EligibilityRules rules) {
        this.clock = clock;
        this.rules = rules;
    }
}
```

避免字段注入、静态上下文访问和服务定位器查找。除非仓库约定出于清晰度或工具需要，否则不要在单个构造函数上添加 `@Autowired`。

当多个 bean 有意实现一个契约时使用 `@Qualifier`。对于长期边界优先使用类型化的自定义限定符而非重复脆弱的字符串名。

### Bean Ownership

使用构造型传达角色，而非装饰：

- `@Service` 用于面向应用/领域的操作
- `@Repository` 用于持久化适配器和异常转换
- `@Controller`/`@RestController` 用于传输适配器
- `@Configuration` 用于 bean 装配
- 不需要容器服务的值、领域行为和辅助功能使用普通 Java 类

不要将每个类都变成 bean。保持框架无关逻辑可在没有 Spring 上下文的情况下构造。

### Proxy Semantics

Spring 注解（如事务、方法安全、缓存和异步执行）通常依赖代理。调用必须跨越代理：

- 同类自调用绕过通知
- private/final 方法根据代理模式可能不是有效的通知入口点
- 用 `new` 创建的对象不受容器管理
- 注解顺序/组合可能改变事务、缓存和安全时机

将通知操作放在清晰的公共边界上，并通过拥有的 Spring Boot 参考测试框架行为。不要添加自注入来强制代理遍历。

### Bean Lifecycle

保持构造函数确定性且无 I/O。仅对有界验证/准备使用显式初始化。对数据库或外部服务的启动调用必须遵循运行时依赖契约。

通过容器拥有资源并通过受支持的生命周期回调关闭它们。销毁钩子应释放资源，而非启动业务工作流。

避免循环依赖。修复职责放置或引入显式契约而非使用 `@Lazy` 作为结构性修复。

### Configuration Classes

保持配置类内聚。Bean 方法应装配协作者，而非包含业务决策。条件 bean 需要显式的回退/缺失模型和每个支持条件的测试。

不要因为示例提到就添加 starter、自动配置、Lombok、映射库、验证、安全、actuator 或迁移依赖。依赖遵循任务拥有的行为和选中的基线。

## Verification Focus

有用的容器证据包括：

- 框架无关逻辑的普通单元构造
- 用于组件扫描和 bean 选择的聚焦上下文启动
- 当存在多个实现时正确的 `@Qualifier` 解析
- 无循环依赖和隐藏的服务定位器访问
- 相关时代理通知调用跨越 Spring 代理的证明
- 真实生产根下的包布局

## Evidence Focus

记录证明拥有的容器行为的窄产物：用于发现或 bean 选择的聚焦上下文测试、用于框架无关构造的普通单元测试，或通过受管代理应用通知的集成断言。仅构建成功不证明扫描、限定符选择、生命周期顺序或代理遍历。

当实现变更包根或配置导入时，在审查证据中包含受影响的生产包和引导配置。当变更通知边界时，标识外部调用的方法和通知提供的行为。

## Unsafe Defaults

- 字段注入。
- 宽泛的组件扫描来补偿放置错误的应用类。
- `@Lazy` 作为循环依赖修复。
- 期望同类调用触发事务、异步、缓存或安全通知。
- 在没有依赖原因的情况下将纯领域/值类变为容器管理。

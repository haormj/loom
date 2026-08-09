# Spring Boot 运行时与配置

本参考负责应用配置、profile、bean 启动、生命周期和优雅关闭。可观测性、异步执行器、缓存、外部客户端和部署资产有独立参考。

## 类型化配置

对相关设置使用类型化配置，并验证决定启动或行为的值。

```java
@ConfigurationProperties("orders.client")
@Validated
public record OrderClientProperties(
    @NotNull URI baseUrl,
    @NotNull Duration connectTimeout,
    @NotNull Duration readTimeout
) {}
```

通过仓库已建立的约定注册属性：configuration-properties 扫描、显式启用或 auto-configuration。保持属性名稳定，通过 `Duration` 和 `DataSize` 等类型记录单位，而非围绕原始整数的注释。

密钥值属于环境/密钥 provider。`application.yml` 包含占位符和非密钥安全默认值是有效的；提交凭证、签名材料或生产端点是无效的。

## Profile 与默认值

Profile 表示一致的环境或运行时模式，而非单个功能标志。保持本地/测试默认值足以满足不拥有共享基础设施的任务。仅当已接受的运行时契约说应用没有它就不能服务时，bean 才对必需依赖快速失败。

避免分散的 `@Profile` 和 `@Value` 分支创建不可测试的组合。优先使用类型化属性、条件配置和显式功能边界。

属性优先级很重要。当任务变更外部提供的值时，验证命令行、环境、profile 和默认配置行为。

## Boot 条件与启动

仅当可选性是真实的时候才使用 `@ConditionalOnProperty`、classpath 或 bean 条件，并测试两种结果。条件应解释匹配时存在什么能力以及不匹配时保留什么。避免产生多个候选或静默移除必需 bean 的重叠条件。

保持 Boot auto-configuration 覆盖窄。优先使用显式应用 bean 而非排除宽泛 auto-configuration，保留仓库标准自定义点。Bean 构造、注入、扫描、qualifier 和代理机制仍在 Java Spring 容器参考中。

启动工作属于显式生命周期边界。`CommandLineRunner` 和 `ApplicationRunner` 必须是幂等的、有界的和失败感知的。不要执行与 Flyway/Liquibase 冲突的 schema 创建。

## 必需与可选依赖

对每个运行时依赖分类：

| 依赖类 | 启动行为 | 请求行为 |
|---|---|---|
| 每个能力都必需 | 根据已接受运行时契约快速失败或就绪失败 | 在可用之前不声称健康 |
| 一个能力必需 | 启动应用；将该能力暴露为不可用 | 返回可操作的不可用行为 |
| 可选增强 | 没有它也启动 | 使用显式回退而不损坏业务含义 |

不要捕获启动异常并以部分初始化状态继续。当任务不拥有时，不要使本地/测试启动需要仅生产的 discovery、tracing、broker 或密钥基础设施。

## 生命周期与关闭

慎重使用 Spring 生命周期 hook。关闭时：

- 在放弃进行中的工作之前停止接受新工作
- 以有界等待停止调度器和执行器
- 关闭容器外管理的客户端和资源
- 根据其归属契约保留可重试或持久工作
- 避免从销毁回调启动新的数据库/外部工作

仅当运行时契约和 hosting 模型支持时才配置优雅服务器关闭。保持超时值外部化和可测试。

## 定时工作

调度器需要显式归属、幂等、重叠行为、时区、失败可见性和关闭行为。在多实例运行时中，定义工作是否可以在每个实例上运行或需要 leader/分布式协调。不要在没有已接受多实例需求的情况下添加分布式锁库。

为业务时间注入 `Clock`。保持 cron/时区配置类型化和已验证。

## Verification Focus

有用的运行时证据包括：

- 具有默认值和无效值的配置绑定
- 条件 bean 和组件扫描的上下文启动
- 没有不相关生产基础设施的本地/测试启动
- 必需依赖快速失败或能力特定的不可用行为
- 变更处的 profile/属性优先级
- 幂等的 runner/调度器行为
- 当拥有时的优雅关闭和有界工作完成

## 不安全默认

- 为一个子系统分散的字符串类型化 `@Value` 设置。
- 作为值提交的生产凭证或 URL。
- 构造期间联系外部服务的新 bean。
- Profile 用作不受控的功能标志系统。
- 无界或非幂等的启动 runner。
- 嵌入在应用配置中的部署假设。

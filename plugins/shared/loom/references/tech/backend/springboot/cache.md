# Spring Boot 缓存实现

Spring Cache 是围绕已拥有真值来源的优化。它不得成为隐式的第二真值来源或改变授权和业务语义。

## 缓存契约

在添加注解之前定义：

- 真值来源
- 缓存名和归属
- 键形态和 tenant/actor scope
- 缓存值形态
- 新鲜度/TTL 预期
- 变更失效或更新行为
- null、not-found 和错误缓存策略
- provider 不可用行为
- 敏感数据限制

仅因为 Spring Boot 支持缓存不需要缓存。

## 键设计

优先使用显式稳定键而非默认参数序列化。

```java
@Cacheable(cacheNames = "order-summary", key = "#tenantId + ':' + #orderId", sync = true)
public OrderSummary findSummary(String tenantId, UUID orderId) {
    return queries.findSummary(tenantId, orderId).orElseThrow(OrderNotFound::new);
}
```

包含每个改变授权或结果内容的维度。不要无意间跨 tenant、locale、权限 scope 或查询过滤器共享条目。避免将可变对象和 JPA 实体作为缓存值；使用不可变 DTO/读模型。

## 失效与事务时序

变更和缓存时序必须一致：

- 当缓存数据反映数据库状态时，仅在成功提交之后驱逐/更新
- 使变更影响的所有键失效，包括列表/查询缓存
- 定义批量更新和外部写入器的行为
- 除非缓存小且按设计全局失效，否则避免 `allEntries = true`

Spring 代理规则适用于缓存注解。同类自调用绕过缓存。注解与事务的排序可能在错误边界驱逐/更新时暴露未提交或回滚的数据。

优先使用简单的 cache-aside 行为。`@CachePut` 仅在返回值精确表示已提交缓存状态时有用。在没有经过验证的条件模型时不要在同一方法上组合 `@Cacheable` 和 `@CachePut`。

## 新鲜度与 Provider 行为

通过所选 provider 和类型化运行时属性配置 TTL 和容量。不要在业务代码中硬编码 provider 特定设置。

对于分布式缓存，定义序列化兼容性、键 namespace/版本控制、网络超时和部分中断期间的行为。对于本地缓存，定义每实例陈旧度和内存限制。

`sync = true` 可以在一个缓存管理器内减少同键并发请求；它不是分布式锁。昂贵或高争用负载可能需要 provider 特定的请求合并或不同的读模型。

不要意外缓存异常。决定 not-found 结果是否可缓存以及缓存多久；负缓存可能隐藏新创建的数据。

## 安全与隐私

切勿缓存原始凭证、bearer token 或包含敏感字段的无限制实体。除非缓存键和缓存对象显式 scope 到已授权 principal/tenant，否则授权必须在返回缓存值之前运行。

## Verification Focus

有用的缓存证据包括：

- 同一键的未命中然后命中行为
- tenant/actor/过滤器键的分离
- 已提交变更之后的失效或更新
- 回滚保留先前的缓存状态
- 拥有时的 TTL/新鲜度行为
- null/not-found/错误策略
- 并发同键加载行为
- provider 不可用回退而不损坏真值来源

## 不安全默认

- 在安全敏感查询上使用隐式键的 `@Cacheable`。
- 缓存 JPA 实体或可变集合。
- 在事务提交之前驱逐。
- 每次写入后全局缓存清除。
- 在没有所选 provider/运行时依赖的情况下引入 Redis 或 Caffeine。
- 当已接受设计说它可选时将缓存可用性视为前提。

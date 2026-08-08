# NestJS DTO、验证与序列化

DTO 定义 HTTP 传输形态。它们验证和转换不受信任的传输值为应用调用边界，并塑造安全响应；它们不是持久化实体或 domain 不变量的替代品。

## 分离 DTO 职责

当契约不同时使用不同的 DTO：

- create/input DTO 用于客户端可写字段
- update DTO 用于 patch/replace 语义
- query DTO 用于过滤、排序、搜索和分页
- response DTO 用于公共字段和表示特定的值
- 当非 HTTP 契约被单独拥有时的 event 或外部适配器 DTO

不要将 ORM 实体复用为请求或响应 DTO。持久化可空性、关系、装饰器、生成列和内部元数据不是 API 契约。

`PartialType`、`PickType`、`OmitType` 和 `IntersectionType` 仅当派生的验证和 schema 保持正确时才有用。PATCH DTO 仍须拒绝不可变/服务端拥有的字段，PUT 替换语义不应建模为不加区分的 partial 类型。

## 验证语义

用 `class-validator` 表达已接受的字段规则：存在性、字符串长度、数字范围、enum 成员资格、UUID/date/email 格式、数组基数和嵌套结构。根据接口契约区分缺失、`null`、空字符串和 false/zero。

对于嵌套对象和数组，将 `@ValidateNested` 与 `@Type(() => ChildDto)` 组合，并在需要时使用 `{ each: true }`。TypeScript 接口上的验证装饰器不起作用；运行时验证需要类和生成的元数据。

将数据库唯一性、归属、生命周期转换和跨记录检查保持在 application/persistence 边界。查询存储的异步自定义验证器可能产生隐藏的 N+1 工作和竞态条件；仅在其局限性已接受的传输局部查找中使用它们。

## 转换与 Query 值

HTTP query 和 path 值以字符串到达。对数字、布尔值、日期、enum 和数组使用显式 `@Type`、聚焦的 `@Transform` 或 parse pipe。当 `"false"`、空字符串、重复 query key 或无效日期等值可能被误解时，避免宽泛的隐式转换。

转换必须是确定性的、无副作用的，并与验证有意排序。不要规范化凭证、不透明标识符、签名或大小写敏感的值。仅对契约声明该行为的字段进行 trim/大小写规范化。

限定分页大小、offset、排序字段和过滤操作符。通过允许列表转换客户端排序 key，而非将任意字符串传给 ORM。

## 全局 ValidationPipe 契约

保留应用已接受的 `ValidationPipe` 设置。面向生产的基线通常考虑：

```typescript
new ValidationPipe({
  whitelist: true,
  forbidNonWhitelisted: true,
  transform: true,
  validationError: { target: false, value: false },
})
```

这些设置改变公共行为，必须与 E2E 测试和错误 envelope 匹配。`whitelist` 仅在 `forbidNonWhitelisted` 为 false 时静默剥离。`transform` 不会使每个隐式转换安全。

如果按路由 pipe 与全局 pipe 不同，在代码中记录有意的契约差异并测试实际路由。

## 响应序列化

将应用结果映射到显式响应 DTO 或已建立的 serializer/interceptor。排除密钥、密码哈希、refresh token、内部授权属性、tenant 内部、软删除标记和 provider 专属列。

对日期、decimal、bigint、enum 和可空序列化保持慎重。`JSON.stringify` 不能直接序列化 `bigint`，ORM decimal 对象可能不匹配已接受的 JSON number/string 表示。

避免依赖 `class-transformer` 排除装饰器，除非真实响应路径调用序列化。返回普通对象或使用 `@Res()` 可能绕过预期的转换行为。

## OpenAPI 一致性

当发布 OpenAPI 时，确保必需/可选状态、enum 值、嵌套数组、格式、默认值、示例和响应类型与运行时验证匹配。从仓库已建立的包（`@nestjs/swagger` 或 `@nestjs/mapped-types`）导入 mapped type，使运行时元数据和生成的 schema 都按预期工作。

## 验证

- 测试有效输入和每个变更的边界：缺失、null、格式错误、超出范围、未知、嵌套和数组案例。
- 证明 PATCH/PUT 对省略、显式 null、不可变和服务端拥有字段的行为。
- 通过真实全局 `ValidationPipe` 执行 query 转换和允许列表。
- 断言精确的验证 envelope，不暴露被拒绝的值或内部目标。
- 在真实响应路径上验证敏感字段排除和 date/decimal/bigint 表示。
- 仅当已发布的 schema 是任务拥有时比较生成的 OpenAPI。

## 交付证据

标识 input/query/response DTO 以及证明验证、转换或序列化的运行时 HTTP 断言。TypeScript 编译和装饰器存在本身不能证明全局 pipe 或响应 serializer 执行。

## 不安全默认

- 一个 DTO 复用于 create、update、持久化和响应。
- `PartialType` 允许不可变或服务端拥有字段。
- 在没有 HTTP 测试的情况下依赖布尔值、数组或日期的隐式转换。
- 数据库检查隐藏在可复用验证器中。
- ORM 实体直接序列化。
- 在不执行响应路径的情况下假设敏感字段排除。

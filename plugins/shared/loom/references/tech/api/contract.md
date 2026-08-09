# API 契约工件

## 在 Loom 中的角色

Loom 的主要 API 契约存在于项目级当前 API 契约中，由已接受的架构工件引用。架构工件记录 `apiContractRef` 加上当前阶段接口引用；下游请求仅接收其任务所需的投影。独立的 OpenAPI 或 schema 文件是可选的，仅在有真实消费者时才应创建。

## 何时创建或更新 OpenAPI

当以下至少一个条件为真时，创建或更新 OpenAPI 文件：

- 用户明确要求 OpenAPI/API 文档
- 仓库已维护 OpenAPI 文件
- 客户端/服务端代码生成依赖它
- API 是外部/公开的，或被另一个独立版本化的应用消费
- 评审或部署需要稳定的契约文件来验证端点

不要仅为满足通用引用规则而创建 OpenAPI。

## 契约文件期望

当需要 OpenAPI 时：

- 优先使用 OpenAPI 3.1，除非仓库已使用其他版本。
- 保持 operation id 稳定且面向资源。
- 使用与业务资源或模块匹配的 tag 对操作分组。
- 当能减少漂移时，复用 component schema 用于请求体、响应、错误、分页和认证。
- 为已接受 API 契约中声明的状态类别包含成功和错误响应 schema。
- 仅当示例能阐明业务行为、重要校验或状态转换时才包含示例。
- 如有项目现有命令则使用其验证；否则记录静态证据和已知差距，而非添加重型工具。

## Operation 对象

- 当仓库、文档或代码生成消费 operationId 时，为每个操作提供稳定、唯一的 `operationId`。在兼容变更中保留已有标识符。
- 在 operation 或 path item 中定义每个 path parameter，标记为 required，并保持其名称与路径模板一致。
- 仅当实现实际提供相应行为时，才描述 query parameter 的接受类型、边界、枚举、数组序列化和默认值。
- 按已接受接口使用 `requestBody.required`。保持 media type 与实现对齐，而非声明不支持的 JSON、multipart、form 或 binary 载荷。
- 使用正确的状态码、头、media type 和 schema 定义每个接受的成功和重要错误响应。当客户端消费 body 时，仅有描述而无响应形态是不够的。
- `201` 创建响应用创建的资源或接受的回读形态表示，当实现暴露规范资源 URI 时包含 `Location` 头。
- 仅当处理异步继续时使用 `202`；文档化客户端实际可用的操作/状态资源、结果查询或回调机制。
- `204` 响应没有响应体。不要为其附加 JSON schema 或示例。

## OpenAPI 3.1 Schema 语义

- 将 OpenAPI 3.1 schema 视为 JSON Schema 2020-12。用 null 联合类型如 `type: [string, "null"]` 或等效的 `oneOf` 建模可空值；不要在 3.1 文档中使用 OpenAPI 3.0 的 `nullable` 关键字。
- 将属性名放入所属对象的 `required` 数组。不要将必需请求体与必需对象属性混淆。
- 对服务端生成的字段使用 `readOnly`，对接受的密钥或只写输入使用 `writeOnly`。当请求和响应 schema 的必需字段或生命周期有实质性差异时，将它们分开。
- 仅当已接受校验和实现执行时才应用 `minLength`、`maxLength`、`pattern`、数值边界、`minItems`、`maxItems` 和 `uniqueItems`。
- 将 `format` 视为互操作性提示，除非所选验证器强制执行。不要声称实现未执行的 email、UUID、URI、date 或 date-time 校验。
- 审慎选择 `additionalProperties`。闭合 DTO 可以拒绝未知字段；扩展映射需要类型化的额外属性 schema。
- 对互斥选项使用 `oneOf`，当客户端需要确定性子类型选择时添加 discriminator。对真正的 schema 组合使用 `allOf`，而非作为不清晰继承的替代。
- 保持枚举值、默认值、示例、可空性和约束与源 DTO、持久化语义和已接受的 API 行为对齐。

## Components、Security 和示例

- 当复用能减少契约漂移时，复用 component schema、parameter、response、header 和 security scheme。不要为仅使用一次的值创建 component，当间接引用使操作更难检查时尤其如此。
- 仅当已接受的认证策略选择了 bearer、API-key、OAuth、cookie 或 mutual-TLS 方案时才声明它们。保持 scope 和操作级要求与实际授权检查对齐。
- 仅对有意公开或以不同方式保护的操作使用操作级 security 覆盖；不要用空 security 数组意外擦除全局要求。
- 示例必须满足其 schema，避免真实凭证或个人数据，并展示有意义的成功、校验或业务冲突行为。
- 保持 tag 与业务资源或模块对齐。不要使用框架包名或数据库表名作为 API 文档分组。

## 验证和生成

- 优先使用仓库已有的 OpenAPI 验证命令和版本。不要仅因为本引用提到验证就安装 Redocly、Spectral、Swagger CLI、Prism 或生成器。
- 当没有验证器时，对可解析的 `$ref` 值、唯一的 `operationId` 值、path parameter 一致性、声明的响应 schema 和示例/schema 一致性执行结构化静态检查。
- 仅当仓库已拥有代码生成或当前任务指名消费者时才生成客户端或服务端桩。记录生成器、版本、输入文件、输出所有权和重新生成命令。
- 当源契约和生成器拥有这些文件时，不要手工编辑生成的客户端或服务端桩。

## 最小专业规范内容

当选择了独立契约文件时，agent 应保留或创建：

- `openapi`、`info` 和 `servers`，仅当 server URL 已知或已建立时
- `paths`，包含 method、path params、query params、request body 和 response 形态
- 稳定且面向 action/resource 的 `operationId` 值
- 用于可复用 DTO 和错误封装的 `components.schemas`
- 当仓库约定支持时，用于共享错误的 `components.responses`
- 仅当当前阶段或已有仓库实际需要认证时才使用 security scheme

不要添加与源代码、已接受 API 契约或测试不一致的生成 OpenAPI 文件，仅为满足文档风格。

## 已有契约文件

如果仓库已拥有 OpenAPI/schema 文件：

- 以已有风格和版本更新它
- 避免重新格式化无关路径
- 保留已有 server/security 约定，除非任务改变了它们
- 在实现证据中引用该文件

## 实现证据

对于涉及 API 契约的任务，实现证据应指明生成或更新的契约文件、验证命令或静态检查，以及任何受影响的生成消费者。如果没有期望的独立契约文件，则引用源代码和测试。

## 运行时和浏览器绑定

已接受的 API 契约是每个面向消费者路径的事实来源。保持 HTTP 接口 `path` 为完整公共路径，包含其公共前缀；不要让下游任务从通用 `/api` 标签重构它。

在契约层面声明一次 API 面绑定：

- `publicExposure.basePath`：部署网关使用的公共代理前缀
- `publicExposure.preservePath`：网关是否原样转发接口路径
- `browserBinding.mode`：`same_origin` 表示浏览器由 Loom 公共入口服务，或 `external_origin` 表示契约中包含明确的外部 origin
- `browserBinding.pathOwnership`：`interface_path`

前端、集成、浏览器验证和部署工作消费这些已接受的值。它必须保留接口路径和请求/响应契约。它不得为已包含公共前缀的路径添加第二个基础前缀。仅当前端源实际使用该变量组合相对后缀时才使用独立的构建时 API 环境变量；它不是通用的部署默认值。

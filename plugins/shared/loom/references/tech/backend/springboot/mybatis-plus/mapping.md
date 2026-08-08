# MyBatis-Plus 实体映射

## 何时使用

当任务拥有 MyBatis-Plus 实体注解、列映射、ID、逻辑删除、乐观锁、enum 或 JSON 字段、或自动审计填充时使用此参考。

## Implementation Focus

- 仅当表、schema、result-map 或排除属性行为与仓库默认不同时才使用 `@TableName`。
- 当 ID 生成契约从数据库和已有配置不明显时使用带显式 `IdType` 的 `@TableId`。
- 对非默认列名、字段策略、填充行为或有意的 TypeHandler 使用 `@TableField`。
- `@TableLogic` 需要确认的删除值、查询行为、恢复行为和保留行的索引/唯一性策略。
- `@Version` 需要将更新计数为零显式处理为并发冲突。

## Enum、JSON 与审计字段

- 保持 enum 的持久化值与其 API label 或本地化显示文本分离。
- 对 JSON、加密或 provider 特定值使用一个项目范围的 TypeHandler 约定。覆盖读取、写入、null、格式错误和历史值。
- 字段级 TypeHandler 可能需要 `autoResultMap = true`；验证实际的 result 映射路径。
- `MetaObjectHandler` 用于已接受的审计字段如创建时间或 actor。它不得隐藏缺失的业务输入或隐式赋值业务状态。
- 实体映射变更必须与迁移和 API 契约保持对齐。

## Verification Focus

在 provider 行为重要的地方，针对所选 provider 测试 ID、字段名、enum、JSON、null、默认值、逻辑删除、自动填充和陈旧版本更新的持久化往返。

## 边界检查

- 当仓库使用 DTO 时，不要将持久化实体直接作为 API 响应模型返回。
- 对 `updateById`、`update(Wrapper)`、导入、job 和管理操作保持字段策略和填充 handler 一致。

## Evidence Focus

命名受影响的实体、表和字段契约、handler、迁移依赖以及变更映射的往返或并发证据。

# MyBatis-Plus 查询与更新 Wrapper

## 何时使用

当任务拥有 MyBatis-Plus Wrapper 构造用于查询、排序、过滤、部分更新、删除或 provider 特定查询边界时使用此参考。

## Implementation Focus

- 对实体支持的列优先使用 `LambdaQueryWrapper` 或 `Wrappers.lambdaQuery`。
- 对可选过滤器使用 wrapper 条件参数而非重复分支。
- 将客户端排序键映射到固定的列允许列表。切勿将用户提供的字段名直接传递给基于字符串的排序。
- 当 Wrapper 变得不透明时，将 join、分组、子查询和 provider 特定 SQL 保留在受控 XML 或专用查询组件中。

## 更新规则

- 对条件部分更新使用 `LambdaUpdateWrapper#set(condition, column, value)`。
- 在决定是否将字段更新为 `null` 之前，区分省略字段和显式 `null`。
- 每个更新和删除需要主键、租户、归属或其他显式 scope 条件。防攻击保护只是最终守卫。
- 当乐观锁、归属或状态转换规则适用时，将更新计数为零视为有意义的。
- `setSql` 仅用于受控表达式如原子增量；绑定值而非拼接它们。

## Verification Focus

覆盖省略与 null 字段、空过滤器、排序允许列表、有 scope 的写入、并发更新以及复杂条件的精确生成结果。

## 查询形态边界

- 对列表路径使用投影并保持所选列与 API 契约对齐。
- 在内存中物化之前保持大结果集有界。
- 显式证明空条件行为；缺失的过滤器不得变成无 scope 的写入或意外的全表读取。

## Evidence Focus

记录 wrapper 方法、输入到列的允许列表、scope 条件、省略与 null 行为以及变更路径的生成查询或集成证据。

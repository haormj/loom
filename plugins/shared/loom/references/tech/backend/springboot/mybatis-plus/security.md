# MyBatis-Plus SQL 安全

## 何时使用

当任务构建或审查可能影响查询 scope、排序、过滤或写入的 MyBatis-Plus Wrapper、XML、注解或 SQL 片段行为时使用此参考。

## Implementation Focus

将 SQL 片段 API 视为高风险边界。

- `apply`、`last`、`inSql`、`notInSql`、`exists`、`notExists` 和 `setSql` 接受 SQL 片段；仅使用固定模板或已验证允许列表。
- 用户输入可以是绑定值，切勿是列名、表名、排序表达式或 SQL 片段。
- 优先使用 lambda 列和类型化条件。通过封闭映射翻译客户端排序/过滤键。
- 审查 XML 和注解 SQL 中的 `${}` 或等价字符串替换。对值使用参数绑定。
- 将租户、归属、授权和审计条件保留在统一 service/plugin 边界中，而非在 controller 中临时添加。
- 防攻击和非法 SQL interceptor 是纵深防御；它们不会使不安全的 SQL 变安全。

## Verification Focus

测试已接受和已拒绝的排序/过滤输入、恶意片段尝试、租户/归属条件、有 scope 的更新/删除以及拒绝输入的稳定 API 错误路径。

## 审查边界

- 同时审查 Mapper XML 和 Wrapper 构造；仅检查 controller 验证是不够的。
- 通过已有 API 错误契约使拒绝输入可观测而不回显原始 SQL 片段。
- 将新的 SQL 片段逃生舱视为需要聚焦证据的安全影响变更。
- 验证授权和租户过滤器不能被替代 Mapper 方法移除。
- 在任务已有的审查证据中保持安全敏感 SQL 变更可见。
- 不要将 SQL 拦截用作显式应用策略的替代。

## Evidence Focus

记录已接受的输入允许列表、受影响的 Mapper 或 XML 语句、授权和租户条件、拒绝输入行为以及聚焦安全的验证结果。

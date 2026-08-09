# Django 与 DRF 到 FastAPI 迁移

此参考仅适用于显式拥有的框架迁移。在引入 FastAPI 原生结构之前保留已接受的行为和数据语义；迁移不是重新设计产品契约的许可。

## 何时使用

当任务显式将 Django/DRF serializer、view/viewset、permission、ORM 访问、中间件、signal、测试或运维 command 移植到 FastAPI 时使用此参考。普通 FastAPI 功能工作不需要迁移指引。

## Implementation Focus

### 构建对齐清单

在更改代码之前盘点源行为：

- 路由 method/path/name 和 trailing-slash 行为
- 请求字段、默认值、read-only/write-only 字段和验证
- 响应 envelope、字段、排序、分页和状态码
- 认证、权限、对象归属和存在性披露
- 过滤器、搜索、排序、queryset 范围限定和 eager loading
- 事务、约束、signal、审计行为和副作用
- 中间件、throttling、异常形态、文件处理和 content type
- 必须保持修复的源测试和生产缺陷行为

将每个差异分类为必需对齐、已接受变更、延迟能力或阻塞。不要静默地将行为规范为偏好的 FastAPI 风格。

### 概念映射

| Django/DRF | FastAPI 边界 |
|---|---|
| Serializer | Pydantic 输入/输出模型加 service 验证 |
| ModelSerializer create/update | 显式应用操作和 SQLAlchemy/repository 映射 |
| ViewSet action | 带显式 method/path/依赖的 `APIRouter` 端点 |
| Permission class | 认证依赖加 permission/归属 service |
| `get_queryset()` 范围限定 | 接收当前 actor/过滤策略的任务拥有查询组件 |
| `select_related`/`prefetch_related` | 显式 SQLAlchemy loader/projection 策略 |
| 中间件 | FastAPI/ASGI 中间件或按 scope 的聚焦依赖 |
| Signal | 具有已定义事务时序的显式 domain/application 事件 |
| Management command | 仓库标准 CLI/job 入口点配以共享应用 service |

映射是语义的，非机械的。保留源组件所保证的，然后将责任放在正确的目标边界。

### Serializer 到 Pydantic 模型

分离 create、replace、patch 和 read 模型。重建字段默认值、别名、enum/date/decimal 行为、嵌套验证、read-only/write-only 语义和敏感字段排除。

查询数据库或依赖请求 identity 的 DRF 验证器移至应用 service 或依赖。`SerializerMethodField` 行为仅在查询成本和加载受控时成为显式响应字段/读模型。

### ViewSet 到 Router 与 Service

将每个标准和自定义 action 映射到其已接受的 method/path/status。保持 router 函数精简，并在应用操作中使事务归属显式。

不要将每个 ViewSet hook 转换为依赖。查询范围限定、变更规则和副作用通常属于 query/application service。为自定义状态转换 action 保留幂等和冲突行为。

### ORM 与数据语义

不要仅按语法重写查询行为。保留：

- 共享或过渡现有 schema 时的表名和列名
- 标识符、序列、enum、小数、时间戳、可空性和默认值
- 外键、唯一/check 约束、级联和删除行为
- 事务/锁行为和完整性错误翻译
- 查询范围限定、annotation/aggregate、确定性排序和分页
- 通过显式加载或投影的 `select_related`/`prefetch_related` 意图

异步 SQLAlchemy 不能在序列化期间安全地执行隐藏懒 I/O。在查询/session 边界内加载所需关系。当两个运行时写入同一 schema 时保持 Django 和目标迁移协调。

### 认证与权限

不要用教程 token 实现替换成熟的 session、JWT、OAuth 或 identity-provider 契约。保留 token/session 格式、issuer/audience、claim、过期、refresh/撤销行为、权限语义和 `401`/`403`/not-found 披露。

对象级权限需要 actor 和目标资源。全局 role 依赖不是等价替代。

### 中间件、Signal 与副作用

显式列出源中间件和 signal。将请求范围的关注移至 ASGI 中间件或依赖，将业务副作用移至应用操作/事件。保留事务时序：Django post-save signal 不会自动转换为安全的 after-commit 或持久投递。

Admin、template、form、上传、后台 worker 和 management command 是独立的表面。显式地保留、替换或延迟它们；不要假设 API 端点对齐覆盖它们。

### 增量替换

当旧和新实现共存时，为每个操作定义路由/数据归属和一个真值来源。避免没有幂等、对账和失败处理的双写。共享数据库访问需要兼容的迁移、事务假设和审计 identity。

仅在对齐测试通过且回滚/归属清晰后按行为切片切换。不要仅因为新 API 服务于正常路径请求就移除源路径、权限、job 或 admin 工作流。

## Verification Focus

- 为成功和有意义的失败路径构建源对目标契约测试。
- 比较验证错误、状态、响应字段、分页/过滤/排序和 trailing-slash 行为。
- 证明认证、宽泛权限、对象归属和受保护资源披露对齐。
- 针对代表性持久化数据验证查询结果、关系加载、约束、事务和副作用时序。
- 为已知缺陷保留源回归测试或等价目标测试。
- 记录并测试每个已接受的行为差异，而非隐藏在实现细节中。

## Evidence Focus

标识源组件、目标组件、对齐义务、已接受差异和证明结果的测试。逐路由的代码翻译或仅目标的正常路径套件不能证明迁移完整性。

## 不安全默认

- 将 DRF serializer 当作直接的 Pydantic 模型翻译，没有 service 规则。
- 用一个 role 检查替换对象权限。
- 异步响应序列化期间懒加载关系。
- 在没有迁移归属的情况下对一个 schema 运行 Django 和 FastAPI 写入。
- 将源 signal 重建为无界的进程内后台任务。
- 在未批准迁移期间重新设计路径、状态、验证或错误 envelope。
- 在没有显式处置的情况下移除 admin、command、worker、上传或中间件行为。

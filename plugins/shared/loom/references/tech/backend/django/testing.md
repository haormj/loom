# Django 与 DRF 测试

使用能证明任务拥有行为的最窄 Django/DRF 测试边界。此参考仅对显式拥有测试实现的任务选择。

## 何时使用

对 Django model/query/migration、serializer、view/router、permission、中间件、settings、admin、management-command 或 DRF API 测试使用此参考。纯 Python 规则仍在 Python 测试参考中。

## Implementation Focus

### 测试边界矩阵

| 行为 | 合适的边界 |
|---|---|
| 纯领域/service 规则 | 尽可能不带 Django 设置的纯 pytest/unittest |
| 模型字段/约束/manager | `TestCase` 或 pytest-django 数据库测试 |
| 事务/提交/锁 | `TransactionTestCase` 或启用事务的 pytest 测试 |
| Serializer 验证/形态 | 聚焦的 serializer 测试 |
| DRF 路由/状态/权限 | `APITestCase`、`APIClient` 或 pytest-django API client |
| 中间件/settings | RequestFactory/client 加 `override_settings` |
| 迁移/数据迁移 | 迁移执行器或仓库迁移测试 helper |
| 查询优化 | 结果断言加查询计数捕获 |

不要为纯计算启动完整 API 栈。不要仅在绕过每个认证/权限组件后才测试受保护端点。

### 数据工厂与隔离

优先使用具有场景相关字段和安全默认值的 factory/builder。保持每个测试的可变数据，避免掩盖归属或造成顺序依赖的巨大共享 fixture。

仅在变更不会在测试间泄漏时对不可变类数据使用 `setUpTestData`。在结果依赖时间、ID 和随机性时使用时区感知的固定时间和确定性值。

在 pytest-django 中数据库访问必须显式。不要默认将每个测试标记为带数据库访问。对与 SQLite 不同的 dialect、锁、JSON、约束、索引或迁移行为使用所选 provider。

### API 与权限测试

当路由 name 是仓库约定的一部分时，使用带 `reverse` 的命名路由。断言精确的状态、body/错误形态、header、分页和数据库副作用。

仅当 token/session 解析不在测试范围时使用 `force_authenticate`。安全测试应通过真实的选择机制获取或构造凭证。

每个变更的受保护操作应包含允许和拒绝案例，包括匿名、权限不足和错误归属者/租户（如适用）。将列表隔离与 detail 对象权限分开测试。

```python
class OrderAccessTests(APITestCase):
    def test_other_requester_cannot_retrieve_order(self):
        self.client.force_authenticate(self.other_user)
        response = self.client.get(reverse("order-detail", args=[self.order.pk]))
        self.assertEqual(response.status_code, status.HTTP_404_NOT_FOUND)
```

### 事务与提交行为

`TestCase` 将测试包装在事务中，可能隐藏真实的提交回调或锁行为。当 post-commit 事件、另一个连接、约束或锁有影响时，使用 `captureOnCommitCallbacks(execute=True)`、`TransactionTestCase` 或集成边界。

断言回滚和持久化回读，而不仅是内存中的模型状态。避免在测试中捕获宽泛异常；断言预期的约束或应用错误。

### 查询与序列化证据

对于 N+1 或查询优化工作，通过 `assertNumQueries` 或查询捕获断言正确结果和查询计数。稳定数据量和 serializer/view 路径，使计数能证明预期的访问模式。

除非 provider 特定的 SQL 是契约，否则不要过度拟合精确 SQL 文本。仅查询计数不能证明正确的范围限定或响应形态。

### 迁移测试

为模型变更运行迁移图/检查命令。用代表性的旧状态行测试数据迁移，并使用历史模型断言升级后的状态。仅在声称可逆时验证反向迁移。

不要在 schema 变更的唯一验证中禁用迁移。

### Async View 与外部边界

对 async view 行为使用 Django 的 async 测试支持/ASGI 路径，并保持同步 ORM 边界的慎重性。在适配器边界 mock 外部端口；当序列化/状态/错误翻译在测试范围时使用 protocol fake。

避免任意 sleep 和共享网络服务。对已接受的背景行为使用完成信号或仓库标准的 worker。

## Verification Focus

- 先运行变更的测试模块/类，当共享 model/router/settings 行为变更时运行所属 Django app 套件。
- 覆盖成功以及与任务相关的验证、not-found、冲突、认证、归属和回滚路径。
- 证明持久化副作用和禁止副作用的缺失。
- 验证所声称 ORM 改进的查询加载/计数。
- 对 schema/数据变更运行迁移检查或聚焦的迁移测试。
- 确保覆盖、settings、凭证、文件、缓存、mail 和全局状态已恢复。

## Evidence Focus

记录测试边界、场景、命令和有意义的断言。仅凭通过的套件或模型工厂创建不能证明路由保护、迁移正确性、事务时序或查询效率。

## 不安全默认

- 为每个纯 Python 分支使用全栈 API 测试。
- 在唯一的认证测试中使用 `force_authenticate`。
- 共享可变 fixture 或测试顺序依赖。
- 将 SQLite 声称为 provider 特定行为的证明。
- 使用 `TestCase` 证明真实提交/锁行为而未调整。
- 查询计数断言没有结果/范围限定断言。
- 为模型变更验证禁用迁移。
- 任意 sleep 或未清理的 settings/依赖覆盖。

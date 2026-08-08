# Django REST Framework View 与 Router

通过保留 queryset 范围限定、权限、serializer、状态和错误契约的最小 DRF/Django view 抽象来实现已接受的 HTTP 行为。

## 何时使用

对 DRF ViewSet、generic/API view、router、自定义 action、queryset 范围限定、按 action 的 serializer/permission、过滤、分页、throttling 集成或 Django async view 使用此参考。

## Implementation Focus

### 选择 View 边界

当已接受的外表确实拥有完整的资源操作集时使用 `ModelViewSet`。对较窄的资源使用只读 viewset 或 generic view，对不适合模型 CRUD 的行为使用 `APIView`/函数 view。

不要仅因为 `ModelViewSet` 提供了 create/update/delete 就暴露它们。Router 生成的路径、name、lookup 字段和 trailing slash 必须与已接受的接口一致。

### Queryset 范围限定

在对象查找之前按 tenant、actor、可见性、生命周期状态和软删除策略限定每个 queryset。当访问因请求而异时，类级 `queryset = Model.objects.all()` 是不安全的。

```python
class OrderViewSet(viewsets.ModelViewSet):
    permission_classes = [IsAuthenticated, OrderPermission]
    filter_backends = [DjangoFilterBackend, OrderingFilter]
    ordering_fields = ["requested_at", "status"]

    def get_queryset(self):
        return (
            Order.objects.visible_to(self.request.user)
            .select_related("requester")
            .prefetch_related("lines")
        )
```

对象权限在对象检索之后运行；它们不会自动过滤列表结果。在需要的地方同时应用 queryset 范围限定和对象级检查。

### 按 Action 契约

当 action 不同时，在 `get_serializer_class`、`get_permissions` 和 parser/renderer 行为中使用显式映射。避免分散在许多 hook 中且没有可见 action 契约的分支。

服务端拥有的字段（如 actor、tenant、审计 identity 和初始状态）在 `perform_create` 中从受信任的请求上下文赋值，或多步行为时由应用 service 赋值。切勿从请求 payload 中信任它们。

### 自定义 Action 与状态变更

自定义 `@action` 端点需要显式的 detail/list scope、method、path、serializer、permission、status、幂等和失败行为。将状态转换和多行写入放在事务性 service 中，而非在 view 中直接变更字段。

仅在语义实际满足时返回 `201`/`202`/`204`。`204` 响应没有 body。当 API 契约声明时保留 location、retry、分页和条件 header。

### 过滤、搜索、排序、分页

使用 `django-filter` 或显式 filter set 进行类型化的允许列表过滤。限制搜索和排序字段；切勿将任意客户端字段传入 ORM 排序。使用确定性默认排序和最大页大小保持列表端点有界。

将 queryset 加载和 annotation 与所选的 list/detail serializer 匹配。避免对不序列化那些关系的 action 应用昂贵的 prefetch。

### 异常与错误

通过聚焦的 exception/handler 将预期的领域/完整性失败转换为已接受的 DRF 错误 envelope。区分验证、not found、冲突、认证、授权、throttling、临时依赖失败和意外错误。

不要在每个 action 中捕获 `Exception` 或暴露数据库/provider 消息。当此任务拥有可观测性时，将意外失败路由到 `tech/code/observability.md` 选择的唯一边界。

### Django Async View

仅当完整 I/O 路径受益且受支持的 API 被正确 await 时才使用 async view。不要将宽泛的 ORM 工作流包装在重复的 `sync_to_async` 调用中并误称为并发。保留线程敏感的数据库行为和事务边界。

根据仓库版本/配置，DRF 支持和中间件可能仍然是同步的。在转换 view 之前进行测量，并将阻塞工作排除在事件循环之外。

## Verification Focus

- 执行任务拥有的成功、验证、not-found、冲突、未认证、禁止和归属路径。
- 断言 list、retrieve、update、delete 和自定义 action 的 queryset 范围限定。
- 验证 router name/path、lookup 字段、action method、serializer、permission 和 status。
- 测试过滤器、搜索、排序允许列表、分页元数据和空结果。
- 对变更的关联对象加载使用查询计数证据。
- 当拥有 async 行为时通过实际 ASGI 路径测试 async view。

## Evidence Focus

标识 view/action、限定范围的 queryset、permission、serializer 以及证明行为的 HTTP 断言。Router 注册或一次成功请求不能证明列表隔离、对象归属、失败映射或查询效率。

## 不安全默认

- 对只读或单 action 外表使用完整的 `ModelViewSet`。
- tenant/用户拥有记录的未限定范围类 queryset。
- 假设对象权限会过滤列表端点。
- 状态转换和外部调用直接在 view 方法中。
- 任意客户端控制的排序/过滤字段。
- 每个 action 都使用一个昂贵的 queryset/prefetch。
- 围绕同步 ORM 工作流包装的 async view，没有明确边界。

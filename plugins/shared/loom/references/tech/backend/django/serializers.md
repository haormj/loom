# Django REST Framework Serializer

DRF serializer 定义 API 输入验证和响应表示。它们不拥有跨入口点的业务不变量、授权、事务策略或无界的数据库工作。

## 何时使用

对 DRF 请求/响应 serializer、`ModelSerializer`、字段/对象验证、嵌套表示或写入、部分更新、关联字段、计算输出以及 serializer 驱动的创建/更新行为使用此参考。

## Implementation Focus

### 分离方向契约

当字段、信任、成本或表示不同时，使用独立的 create、update/patch、list 和 detail serializer。不要通过 `fields = "__all__"` 在公共 API 上暴露每个模型字段。

```python
class OrderCreateSerializer(serializers.Serializer):
    supplier_name = serializers.CharField(max_length=160, trim_whitespace=True)
    lines = OrderLineCreateSerializer(many=True, allow_empty=False)


class OrderReadSerializer(serializers.ModelSerializer):
    requester_name = serializers.CharField(source="requester.username", read_only=True)

    class Meta:
        model = Order
        fields = ["id", "supplier_name", "status", "requester_name", "requested_at"]
        read_only_fields = fields
```

显式标记服务端拥有的、审计、identity、状态和敏感值为 read-only 或省略它们。将密钥标记为 write-only，确保它们不会在 `validated_data` 中保留超过所需时间。

### 验证归属

对局部字段规则使用 `validate_<field>`，对确定性的跨字段请求规则使用 `validate`。数据库唯一性、actor 归属、生命周期资格、库存和多记录不变量还必须在 model/service/transaction 边界中强制执行。

查询数据库的 serializer 验证可能产生竞态，并可能在 list/nested 操作中造成 N+1 行为。仅将其用于有界反馈，同时保留权威约束/写入检查。

保持错误码/字段位置与已接受的 API 错误契约一致。不要从 `create` 或 `update` 返回原始的 model/database 异常。

### 关联字段与表示

从已接受的响应契约中选择 ID、slug、hyperlink、嵌套对象或 side-loaded 数据。避免隐式深度嵌套和无限制的反向关系。

`source="relation.field"` 和嵌套 serializer 需要匹配的 queryset 加载。Serializer 不应是发现查询规划的第一个地方。`SerializerMethodField` 必须保持确定性、有界且没有逐行数据库查询。

### 创建、更新与嵌套写入

仅当保留业务和事务行为时，才在 `ModelSerializer` 中保持简单的模型构造。将多模型工作流和状态转换移至应用 service。

嵌套写入需要显式归属、匹配规则、create/update/delete 语义和 `transaction.atomic()`。除非替换是已接受的契约，否则不要在每次 patch 时删除并重建所有子记录。

对于部分更新，区分省略的字段和显式的 null/空值。`partial=True` 放宽必需字段验证；它不会自动定义业务 patch 语义。

### Serializer 上下文

将 serializer 上下文用于请求/actor、URL 生成、locale 或已计算的值。不要访问模块全局请求状态。保持依赖 actor 的输出与授权和 queryset 范围限定一致。

### 性能与分页

对表格和摘要使用更轻量的 list serializer 或 annotated/projection 字段。默认避免序列化大型 queryset 或文件/blob 字段。分页属于 serializer 评估之前的 view/query 边界。

## Verification Focus

- 测试有效和无效的字段/跨字段输入，验证精确的错误结构。
- 证明 create、完整 update、partial update、显式 null、省略以及不可变/服务端拥有字段的行为。
- 断言敏感/write-only 字段排除以及稳定的 list/detail 响应形态。
- 当拥有时，测试嵌套写入归属、回滚、更新匹配和移除语义。
- 将计算/关联 serializer 与查询计数或 loader 证据配对。
- 当状态或错误 envelope 重要时，通过 API 测试验证 serializer 行为。

## Evidence Focus

标识方向性 serializer、验证规则、嵌套写入策略或表示，以及证明线上和持久化行为的断言。仅凭 serializer `.is_valid()` 不能证明授权、事务、查询计数或端点错误。

## 不安全默认

- 公共 `ModelSerializer` 使用 `fields = "__all__"`。
- 一个 serializer 在 create、patch、list 和 detail 间复用，尽管契约不同。
- 授权或持久业务不变量仅由 serializer 验证强制执行。
- `SerializerMethodField` 发出逐对象查询。
- 嵌套更新静默删除并重建关联记录。
- 通过全局状态访问 request/user。
- 敏感或服务端拥有的模型字段因推断而暴露。

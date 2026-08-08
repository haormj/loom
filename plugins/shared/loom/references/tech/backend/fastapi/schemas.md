# FastAPI Pydantic V2 契约模型

将 Pydantic 模型用作显式的传输和配置契约。它们验证和序列化数据；它们不替代领域规则、持久化模型或授权检查。

## 何时使用

当任务拥有 FastAPI 请求/响应模型、参数模型、Pydantic V2 验证器、序列化别名、ORM 支持的响应映射、settings 模型或生成的 JSON Schema/OpenAPI 形态时使用此参考。

## Implementation Focus

### 分离输入、Patch 与输出模型

显式地建模方向和信任。Create、replace、patch、内部 command 和响应形态通常不同。切勿仅为避免映射代码而复用数据库实体或一个宽泛模型。

```python
class OrderCreate(BaseModel):
    supplier_name: Annotated[str, Field(min_length=1, max_length=160)]
    requested_at: datetime
    lines: Annotated[list[OrderLineCreate], Field(min_length=1)]


class OrderPatch(BaseModel):
    supplier_name: Annotated[str, Field(min_length=1, max_length=160)] | None = None


class OrderRead(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: UUID
    supplier_name: str
    status: OrderStatus
    requested_at: datetime
```

对于 patch 命令，`None`、省略和显式清除可以是不同的操作。仅当已接受的更新语义区分它们时才使用 `model_fields_set` 和 `model_dump(exclude_unset=True)`。

### Pydantic V2 API

使用 `field_validator`、`model_validator`、`ConfigDict`、`model_validate` 和 `model_dump`。不要在 V2 代码库中混入 V1 的 `@validator`、`@root_validator`、`class Config`、`.dict()` 或 `orm_mode`。

字段验证器处理局部规范化和约束。模型验证器处理确定性的跨字段形态规则。验证器不得查询数据库、调用外部服务、读取请求 identity 或执行写入；这些检查属于应用 service 或依赖。

```python
class DateWindow(BaseModel):
    starts_at: datetime
    ends_at: datetime

    @model_validator(mode="after")
    def end_follows_start(self) -> Self:
        if self.ends_at <= self.starts_at:
            raise ValueError("ends_at must be after starts_at")
        return self
```

### 可选性、默认值与类型

`T | None` 表示值可以为 null；默认值决定是否可以省略。不要为消除验证失败而将字段设为可选。在序列化已定义的地方使用受约束的类型、enum、`Decimal`、UUID、时区感知的 datetime 策略、URL 和领域特定的值模型。

对可变集合使用 `default_factory`。除非已接受契约显式使用，否则将类货币值排除在二进制浮点模型之外。

### 序列化与 ORM 映射

仅在从对象有意创建的响应模型上使用 `from_attributes=True`。确保在序列化之前加载所需关系；Pydantic 不得在 session 边界之后触发隐藏的异步懒加载。

从输出模型中排除凭证、哈希、token、内部标志、provider payload 和无限制的关系图。`repr=False` 不是序列化安全边界；控制实际字段和 serializer。

别名必须匹配已接受的线上契约。当输入和输出名称不同时慎重使用 `validation_alias` 和 `serialization_alias`，并保留仓库范围内的大小写约定。

### 自定义序列化与计算字段

对稳定的线上转换使用字段/模型 serializer，而非业务决策。计算字段仅在其成本有界且依赖已加载时才属于响应模型。避免通过运行时条件改变 OpenAPI 形态的动态字段。

### Settings 模型

对类型化的运行时配置使用 `BaseSettings` 和 `SettingsConfigDict`。保持环境名、前缀、大小写敏感、嵌套分隔符和必需值显式。不要在依赖中重复实例化 settings 或将密钥值作为默认值提交。

## Verification Focus

- 验证已接受和已拒绝的 create/replace/patch payload。
- 当拥有时证明省略、显式 null、默认值、别名、enum、datetime、decimal 和嵌套列表行为。
- 测试敏感字段排除和带有所需关系加载的 ORM 支持 `model_validate`。
- 用 `model_dump` 断言稳定的序列化输出，而不仅是模型构造。
- 对变更的公共模型检查生成的 OpenAPI/JSON Schema。
- 独立于进程全局状态测试 settings 默认值、必需值和无效配置。

## Evidence Focus

命名模型方向、验证器、序列化规则、别名或 settings 边界，并展示证明线上形态的断言。仅模型构造不能证明端点状态、错误 envelope、关系加载或敏感字段排除。

## 不安全默认

- 一个模型用于 create、patch、持久化和响应。
- V2 项目中使用 Pydantic V1 语法。
- 验证器中的数据库或网络调用。
- 仅为让无效 payload 通过而添加的可选字段。
- 在没有显式响应模型的情况下序列化 SQLAlchemy 实体或懒加载关系。
- 响应模型中保留的密码、token 或 provider 密钥字段。
- settings 默认值中硬编码的环境特定 URL、origin 或密钥。

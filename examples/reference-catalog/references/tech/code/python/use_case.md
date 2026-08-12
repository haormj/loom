# Python 测试用例规范

本文件是 Acme 企业参考,指导 agent 如何在 Loom 任务中编写符合企业规范的 Python 测试用例。

## When To Use

- 任务包含测试编写(新增测试、补充测试、修复测试)。
- 任务属于测试增量(TaskKind::VerificationIncrement)或包含 `AddOrUpdateTests` 动作。
- 优先使用仓库约定。仅当现有项目已支持时才引入企业测试规范。

## Implementation Focus

- 测试文件放在 `tests/` 目录,镜像 `src/` 结构:
  ```
  src/acme_app/services/user_service.py
  tests/services/test_user_service.py
  ```
  测试文件名以 `test_` 前缀,测试函数以 `test_` 前缀。
- 测试函数命名使用 `test_{行为描述}_{条件/场景}` 模式,描述被测行为而非被测方法:
  ```python
  # 好:描述行为
  def test_import_data_raises_timeout_when_network_unreachable():
      ...

  # 差:描述方法名
  def test_import_data_1():
      ...
  ```
- 使用 `pytest.mark.parametrize` 表达多场景测试,不要复制粘贴多个类似测试:
  ```python
  @pytest.mark.parametrize("input,expected", [
      ("valid@example.com", True),
      ("missing-at-sign", False),
      ("", False),
  ])
  def test_validate_email_returns_expected(input, expected):
      assert validate_email(input) == expected
  ```
- 每个测试遵循 Arrange-Act-Assert 结构:
  - **Arrange**:准备数据、mock、fixture
  - **Act**:调用被测函数
  - **Assert**:验证结果
  不要在 Act 之后再有副作用操作。
- 异常路径测试使用 `pytest.raises` 断言异常类型和消息:
  ```python
  def test_create_user_raises_when_email_exists():
      with pytest.raises(AlreadyExistsError, match="email already registered"):
          user_service.create("dup@example.com")
  ```
- 使用 fixture 共享 setup 逻辑,不要在多个测试中复制 setup 代码。
  fixture 名称描述性表达(`acme_test_app`、`mock_db`),不要用 `setup`/`data`。
- Mock 外部边界(HTTP 客户端、数据库连接、文件系统),不要 mock 内部协作对象。
  内部协作对象的 mock 使测试脆弱,与实现耦合。
- 集成测试标记 `@pytest.mark.integration`,单元测试默认无标记。
  CI 中单元测试每次运行,集成测试仅在部署前运行。

## Verification Focus

- 运行 `uv run pytest` 证明所有测试通过。
- 运行 `uv run pytest --cov=src` 验证覆盖率达标(新增逻辑 ≥80%,异常路径 100%)。
- 验证测试函数命名遵循 `test_{行为}_{条件}` 模式。
- 验证多场景测试使用 `parametrize`,无复制粘贴。
- 验证 Mock 仅限外部边界,内部协作对象未 mock。

## Evidence Focus

- 在证据总结中,说明测试文件镜像 `src/` 结构,命名遵循企业规范。
- 说明使用 `parametrize` 表达多场景,使用 fixture 共享 setup。
- 说明 Mock 仅限外部边界,异常路径使用 `pytest.raises` 断言。
- 说明覆盖率达到企业标准(≥80% 行覆盖,100% 异常路径)。

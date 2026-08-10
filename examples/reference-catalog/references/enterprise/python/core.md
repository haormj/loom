# Acme 企业 Python 核心规范

本文件是企业示例参考,演示如何用 `itemEntries` 将 vendor 的 `python/core`
改写为指向企业内部规范。它替代默认的 `tech/code/python/core.md`。

与 vendor 默认的关键差异:
- **强制 Ruff**:替代 black + isort + flake8,所有 Python 代码必须通过 Ruff 检查
- **强制 strict 类型检查**:公共 API 必须有类型注解,`mypy --strict` 或 `pyright --strict`
- **不用 packaging**:企业统一用 `uv` 管理依赖与虚拟环境,不需要单独的打包参考

## When To Use

- 任务变更了 Python 应用、库、CLI、服务、数据处理、配置或共享模块代码。
- 项目使用 Acme 企业 Python 规范(强制 Ruff + strict 类型检查)。
- 优先使用仓库约定与 Acme 内部编码规范。

## Implementation Focus

- 在新代码中对文件系统路径使用 `pathlib.Path`,除非周围 API 要求字符串。
- 对文件、网络客户端、数据库会话、锁使用上下文管理器(`with`/`async with`)。
- 对结构化领域值使用 dataclass、枚举或 Pydantic v2 模型。不要在业务逻辑中传递
  大型无类型 `dict[str, Any]`。
- 避免可变默认参数。对 dataclass 使用 `field(default_factory=...)`。
- 用可操作的消息抛出显式领域异常。不要使用裸 `except`,不要吞掉异常。
- 在应用/运行时代码中使用 `logging` 而非 `print`。日志器保持模块范围,不记录密钥。
- 配置加载在启动或适配器边界完成,结果以类型化设置向内传递。不要在领域代码中读取环境变量。
- **强制 Ruff**:所有 Python 文件必须通过 `ruff check` 和 `ruff format --check`。
  不要引入 black、isort、flake8、pylint 配置;Ruff 已覆盖这些工具的能力。
- **强制 strict 类型**:公共函数、方法、类属性必须有类型注解。`Any` 仅限外部边界,
  必须在进入业务逻辑前转换为已验证类型。
- 依赖管理使用 `uv`,不要引入 poetry/pipenv/pip-tools 配置。

## Verification Focus

- 运行 `uv run pytest` 证明变更可运行且测试通过。
- 运行 `ruff check` 和 `ruff format --check` 证明代码符合 Acme 规范。
- 运行 `mypy --strict`(或 `pyright --strict`)证明类型正确。
- 为新业务规则添加测试,包括至少一个异常路径。
- 确认没有引入可变默认参数、裸 `except`、密钥日志或分散的配置读取。

## Evidence Focus

- 在证据总结中,说明遵循 Acme Python 规范:Ruff 通过、strict 类型检查通过、
  `pathlib`/资源处理、dataclass/Pydantic 建模、异常契约、日志边界、配置边界。
- 创建新 Python 文件时,说明包结构与 Acme 约定一致。

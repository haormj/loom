# Ruff Lint 与格式化规范

本文件是企业示例参考,演示如何在替换组中新增 item(`ruff`)。
企业强制用 Ruff 替代 black + isort + flake8 + pyupgrade 等分散工具。

## When To Use

- 任务变更了 Python 源码、`pyproject.toml` 的 `[tool.ruff]` 配置、
  或 CI 中的 lint/format 步骤。
- 当需要决定某段代码是否符合 Acme Ruff 规范时使用此参考。

## Implementation Focus

- 所有 Python 文件必须通过 `ruff check`(lint)和 `ruff format --check`(格式化)。
- 在 `pyproject.toml` 中配置 Ruff,不要使用独立的 `.flake8`、`.isort.cfg`、
  `setup.cfg` 或 black 配置。
- 启用 Acme 规定的规则集(在 `[tool.ruff.lint]` 中配置 `select`):
  - `E`/`W` — pycodestyle 错误与警告
  - `F` — pyflakes(未使用导入、变量等)
  - `I` — isort(导入排序)
  - `UP` — pyupgrade(现代化语法)
  - `B` — bugbear(常见陷阱)
  - `SIM` — 代码简化
- 行长度限制跟随项目设置(默认 88,可在 `[tool.ruff]` 的 `line-length` 中覆盖)。
- 使用 `ruff format` 而非 `black` 进行格式化。两者输出高度兼容,但 Ruff 是 Acme 标准。
- 对不可避免的规则违规,使用 `# noqa: <rule>` 并附窄原因。不要全局禁用规则。
- 导入排序由 Ruff 的 `I` 规则处理,不要同时运行 `isort`。

## Verification Focus

- 运行 `ruff check` 证明无 lint 违规。
- 运行 `ruff format --check` 证明格式化合规。
- 如果变更了 `[tool.ruff]` 配置,验证新规则集不会引入大量既有违规。
- 如果使用了 `# noqa`,确认原因窄且准确。

## Evidence Focus

- 在证据总结中,说明 Ruff check 与 format 均通过,或列出已豁免的规则及原因。
- 如果变更了 Ruff 配置,说明新增/移除的规则及其理由。

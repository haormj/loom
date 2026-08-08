# 仓库指南

## 项目结构与模块组织

- `src/rust/`:工作区 crate。`mcp-server` 暴露工具;`planning`、`execution`、`deploy`、`knowledge` 等领域 crate 负责工作流行为;`contracts`、`core`、`state` 提供共享协议。
- `src/python/algorithms/`:内置 BM25、TF-IDF、分词与 worker 代码。
- `tests/rust/` 与 `tests/python/algorithms/`:集成与算法测试,按产品域组织。
- `plugins/opencode/`:agent 适配器。共享的 Loom 与 deploy 指南位于 `plugins/shared/`;避免在适配器文件中重复共享规则。
- `docs/`、`assets/`、`scripts/`:用户文档、README 媒体与本地安装辅助脚本。

## 构建、测试与开发命令

在仓库根目录运行命令:

```bash
npm run rust:test
npm run python:test
cargo fmt --manifest-path src/rust/Cargo.toml --all --check
cargo build --manifest-path src/rust/Cargo.toml -p mcp-server -p setup
```

迭代时使用定向 Rust 测试,例如:

```bash
cargo test --manifest-path src/rust/Cargo.toml -p deploy --test deploy_workflow
```

在运行时或插件变更后,用 `./install.sh --agent opencode --local-build` 刷新本地集成。

## 代码风格与命名约定

使用 Rust 2021 约定并保持代码 `rustfmt` 合规。模块、函数、文件用 `snake_case` 命名;类型与 trait 用 `UpperCamelCase`;常量用 `SCREAMING_SNAKE_CASE`。Python 遵循四空格缩进、`snake_case` 与 `test_*.py` 命名。变更保持在现有领域边界内,优先使用结构化 Serde 模型而非临时 JSON 拼接。

## 文档语言

除非用户为特定文档明确指定其他语言,所有文档(规格、设计文档、README 章节、排障说明)使用中文(简体中文)编写。代码标识符、命令、文件路径与 JSON/协议术语保持英文。

## 测试指南

为行为变更添加聚焦的回归覆盖。Rust 集成测试归入对应的 `tests/rust/<domain>/` 测试套件;本地单元测试可保留在实现代码旁。Python 测试使用 `pytest`。先运行受影响的包或测试目标,再在影响发布的变更前运行两条产品测试通道。修复应能复现此前的失败。

## 参考与契约设计

- 从结构化仓库事实和任务所有权中选择参考;绝不全局默认某个 focus 或 group。
- 在添加字段或参考前审计既有契约;在同一变更中移除已废弃和重复的指导。
- 不要让 agent 编写 MCP 派生的字段。
- 为每条参考路由测试选中与不选中两种情况。
- 不要仅凭语言、框架或前端任务推断测试工作。测试参考与浏览器检查需要明确的任务所属证据。
- 将浏览器启动/运行时失败归类为环境状态;绝不通过通用代码执行修复流程处理。

## 提交与 Pull Request 指南

分支必须匹配 CI 模式,如 `feature/name`、`fix/name` 或 `docs/name`。提交标题使用 Conventional Commits,可带小写 scope:`fix(deploy): validate source roots`。标题保持在 200 字符以内。

PR 应保持聚焦,说明用户可见或契约层面影响,列出验证命令,并链接相关 issue。文档或 UI 可见变更应附截图。不要在 diff 中混入无关的格式化与重构;绝不提交生成的 `.loom/` 运行时状态。

多花时间思考;无需通过评论通道向我汇报进度。不要发送可选评论。

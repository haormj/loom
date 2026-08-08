# Python 部署参考

当实现或修复 Python 项目的 loom deploy 支持时，使用本参考文档。

## 扫描器信号

- Python 项目文件：`requirements.txt`、`pyproject.toml`、`Pipfile`、`uv.lock`、`poetry.lock`、`server.py`、`main.py`、`app.py`、`manage.py`。
- 还需检查 package 源文件和本地文档（如 `README.md` 和 `HTTP_API.md`）中的 entrypoint、路由、`--port` 示例和健康端点。
- 包管理器：
  - `uv.lock` 或 `[tool.uv]` -> uv
  - `poetry.lock` 或 `[tool.poetry]` -> poetry
  - 否则 -> pip
- 框架提示：
  - `fastapi` 或 `uvicorn` -> FastAPI，默认端口 8000。
  - `flask` 或 `gunicorn` -> Flask，默认端口 8000。
  - `django` 或 `manage.py` -> Django，默认端口 8000。
  - `streamlit` -> Streamlit，默认端口 8501。
  - `ThreadingHTTPServer`、`BaseHTTPRequestHandler`、`http.server` 或本地 `run_http_server` helper -> 标准库 HTTP server，默认端口 8000。
- 如果检测到标准库 HTTP server，当该文件包含 HTTP server 信号时，优先选择 `server.py`，然后 `main.py`，然后 `app.py` 作为 entrypoint。
- 对于带 `--host`/`--port` 的 stdlib HTTP entrypoint，生成容器安全的启动命令，如 `python server.py --host 0.0.0.0 --port 8000`。
- 从源代码或文档检测健康路径。优先选择显式路由（如 `/health`、`/healthz`、`/ready`、`/readiness`、`/api/health` 或 `/up`），而非通用的 `/`。

## 依赖解析边界

- 外部 Python 打包库稍后可能对依赖和版本解析有用，但不应在首个部署扫描器路径中要求。
- Entrypoint、host 绑定、端口和健康路由推断应保持确定性和可从本地文件解释，因为这是部署行为而非包解析。

## 模板规则

- 使用 `python:3.12-slim`。
- 设置 `PYTHONDONTWRITEBYTECODE=1` 和 `PYTHONUNBUFFERED=1`。
- 将 `PORT` 设为检测到的容器端口。
- 对于 pip，存在时安装 `requirements.txt`。
- 对于 poetry，安装 Poetry 并运行 `poetry install --only main`，禁用 virtualenv 创建。
- 对于 uv，安装 uv 并优先使用 `uv pip install --system`。

## 修复说明

- 大多数 Python 启动失败是错误的模块名（`main:app` vs `app:app`）、缺失运行时依赖或绑定到 `127.0.0.1`。
- 除非用户批准源代码更改，否则将修复保持在生成的 Dockerfile/Compose 中。

## 扫描器信号到部署事实

在生成文件之前，将 Python 扫描器证据转换为部署事实：

- 依赖清单成为服务根、包管理器、lockfile 和安装命令事实。
- FastAPI/Flask/Django/Streamlit/stdlib HTTP 信号成为运行时框架和端口事实。
- ASGI/WSGI 符号（如 `app = FastAPI()`、`Flask(__name__)`、`application` 和 Django settings）成为启动命令候选。
- `manage.py` 和 Django settings 成为应用根和框架 env 事实。
- Env 示例和 settings 模块成为必需/生成的环境事实。
- SQLAlchemy/Django 数据库设置、Alembic、Redis/Celery 和驱动依赖成为依赖服务事实。
- 没有 HTTP server 信号的纯脚本成为非预览或命令式部署事实，而非虚假 HTTP 应用。

## 生成的资产预期

生成的 Python 资产应显示：

- 除非项目元数据选择了更具体的兼容 Python 版本，否则使用 `python:3.12-slim`。
- 与 pip、uv 或 Poetry 事实匹配的安装步骤。
- 使用检测到的 ASGI/WSGI/stdlib entrypoint 并绑定到 `0.0.0.0` 的运行时命令。
- `PORT` 设为选定的容器端口。
- Django 本地预览 env 仅在检测到 Django 时包含安全的本地密钥/allowed-host 默认值。
- 依赖 URL 使用 Compose 服务 DNS 名和生成的本地凭证。
- Healthcheck 候选优先选择显式的源/文档路径，然后是通用的 `/`。

## 修复边界

在以下情况下修复生成的 Python 部署资产：

- Dockerfile 从错误的 manifest 或包管理器安装。
- Entrypoint/模块名错误但可从源代码证据推断。
- Uvicorn/Gunicorn/Flask/Django 命令绑定到 localhost 或错误端口。
- 生成的 env 遗漏了安全的本地框架默认值。
- 依赖服务 URL 指向 localhost。

在部署资产修复期间不要编辑 Python 源代码、settings 模块、迁移或依赖清单，除非 MCP 操作路由到执行修复。

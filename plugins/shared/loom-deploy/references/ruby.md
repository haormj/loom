# Ruby 部署参考

当实现或修复 Ruby 家族项目的 loom deploy 支持时，使用本参考文档。

## 扫描器信号

- `Gemfile` 标识 Ruby 项目，应在 `package.json` 之前检查，因为 Rails 项目通常包含前端资产。
- `rails`、`railties` 或 `config/application.rb` 信号标识 Rails。
- `sinatra` 信号标识 Sinatra。
- `puma` 信号标识 Rack/Ruby web 应用。
- `Gemfile.lock` 是 Bundler lockfile。
- `.ruby-version` 或 `Gemfile` 中的 `ruby "x.y.z"` 应选择 Ruby 次版本。默认 Ruby 版本为 3.3。

## 模板规则

- v1 使用单容器本地预览模板。
- 使用 `ruby:<minor>-slim`。
- 安装常见的原生构建依赖，如 `build-essential`、`git`、`libpq-dev` 和 `pkg-config`。
- 在源文件之前复制 `Gemfile` 和 `Gemfile.lock`，然后运行 Bundler install。
- 对于 Rails，创建 `tmp/pids`、`tmp/cache`、`log` 和 `storage` 目录。
- Rails 本地预览使用 `bundle exec rails server -b 0.0.0.0 -p ${PORT:-3000}`。
- 对于 Rack/Sinatra 应用，使用 `bundle exec rackup -o 0.0.0.0 -p ${PORT:-3000}`。

## 依赖服务

- 从 `pg`、`postgres`、Rails 数据库配置或连接字符串检测 Postgres。
- 从 `redis`、`sidekiq` 或 Redis 连接字符串检测 Redis。
- 从 `mysql2`、`mysql` 或数据库配置检测 MySQL/MariaDB。
- 从 `mongoid` 或 MongoDB 连接字符串检测 MongoDB。
- 从 gem 名和 env/config 信号检测 RabbitMQ 和 Elasticsearch/OpenSearch。

## 修复说明

- 如果 Bundler 因原生扩展失败，在编辑应用代码之前更新生成的 OS 包安装。
- 如果 Rails 启动但返回 500，检查日志中缺失的 `SECRET_KEY_BASE`、storage 权限、数据库连接错误或待执行迁移。
- 如果启动前需要 assets，未来的提供者应添加 asset 预编译；v1 本地预览优先启动应用而不强制生产资产编译。
- 默认不要将真实 `.env` 文件复制到生成的镜像中；使用 `.env.example` 推断所需变量。

## 扫描器信号到部署事实

在生成文件之前，将 Ruby 扫描器证据转换为部署事实：

- `Gemfile` 路径成为服务根、manifest ref 和 Bundler install 事实。
- `Gemfile.lock` 成为 lockfile ref。
- Rails 配置、`bin/rails`、`config.ru`、Sinatra/Puma gem 和路由文件决定框架/runtime 事实。
- `.ruby-version`、`Gemfile` `ruby` 声明和 lockfile 平台提示选择 Ruby 镜像版本和原生包需求。
- Rails `database.yml`、ActiveRecord adapter、Redis/Sidekiq、Elasticsearch 和 env 示例成为依赖服务事实。
- Rails 项目内的 Node package 文件是次要资产信号，不应覆盖 Ruby 应用角色。

## 生成的资产预期

生成的 Ruby 资产应显示：

- 当 manifest 允许缓存时，在源复制之前进行 Bundler install。
- 与检测到的 gem（如 `pg`、`mysql2`、`sqlite3`、`nokogiri` 或图像处理 gem）匹配的原生构建包。
- Rails 本地预览包含生成的 `SECRET_KEY_BASE`、可写的 `tmp`、`log` 和 `storage` 目录。
- 运行时命令将 Rails/Rack/Sinatra 绑定到 `0.0.0.0` 和选定的端口。
- 依赖 URL/config 使用 Compose 服务 DNS 名和生成的本地凭证。
- 真实 `.env` 值从不复制到镜像中。

## 修复边界

在以下情况下修复生成的 Ruby 部署资产：

- Bundler 原生扩展构建因缺失 OS 包而失败。
- 运行时命令指向错误的 Rack/Rails entrypoint 或错误端口。
- Rails 本地密钥/storage 目录在生成的 env 或文件系统设置中缺失。
- 依赖配置在容器内使用了 localhost。

在部署资产修复期间不要编辑 Ruby 应用代码、Gemfile 依赖、迁移或真实 env 文件，除非 MCP 操作路由到执行修复。

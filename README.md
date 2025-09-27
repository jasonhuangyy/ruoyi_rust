# RuoYi-Rust  高性能 Rust 重构后端 
  ### 丰盛辉煌

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md) 

---
有任何需求，请直接在requests中提出，我有时间会慢慢的补充上
---
---
**RuoYi-Rust** 是一个雄心勃勃的项目，旨在通过现代、高性能的 Rust 语言及其强大的生态系统，完整重写主流 Java Web 框架 [RuoYi](https://gitee.com/y_project/RuoYi) 的后端服务。我们追求的目标不仅是功能对等，更是在性能、资源占用(目前仅几M的内存占用)、安全性及现代化开发体验上实现全面超越，为Rust生态提供一个可复用的管理系统。

本项目后端与 [RuoYi-Vue3(rust后台配套微调版)](https://gitee.com/rustdev/ruo-yi-vue3.git) 前端项目配套使用。

## ✨ 核心亮点 (Why RuoYi-Rust?)

*   🚀 **极致性能**: 基于 `Tokio` 和 `Axum` 框架，原生异步，无 GC 延迟。提供比原生 JVM 应用更快的 API 响应速度和显著降低的内存占用。

*   🔒 **内存安全**: 借助 Rust 强大的所有权和借用检查机制，从编译层面根除空指针、数据竞争等常见的运行时安全隐患，构建坚如磐石的系统。

*   📦 **极简部署**: 整个后端项目可编译为单个可执行文件(内置缓存系统)，不依赖 JRE、Tomcat、Nginx 等任何外部运行时。部署过程从未如此简单：上传文件，启动，仅此而已。

*   🛠️ **现代化开发体验**:
    *   **编译时 SQL 校验**: 采用 `sqlx` 作为数据库交互层，遵循“数据库 Schema 为唯一真实来源”原则。任何不符合数据库表的 SQL 查询都将在编译时被发现，杜绝大量潜在的数据库运行时错误。
    *   **声明式权限控制**: 通过自定义过程宏实现类型安全的声明式 API 权限校验，将权限元数据与业务逻辑高度内聚，彻底告别手写权限字符串带来的风险。
    *   **模块化设计**: 采用清晰的 Cargo Workspace 结构和分层架构，实现高内聚、低耦合，便于维护与扩展。

## 📊 项目状态 (截至 2025-08-28)

我们正在逐步实现 RuoYi 的全部功能。当前已完成的核心模块如下：

-   [x] **核心框架**
    -   [x] 配置加载 (`config-rs`)
    -   [x] 结构化日志 (`tracing`)
    -   [x] 数据库连接池 (`sqlx`)
    -   [x] 统一响应 (`AjaxResult`) 与错误处理 (`AppError`)
    -   [x] JWT 认证中间件 (`jwt-simple`)
    -   [x] 高性能本地缓存 (`moka`)
-   [x] **系统模块 (`system`)**
    -   [x] 登录、获取用户信息、动态路由 (`/login`, `/getInfo`, `/getRouters`)
    -   [x] 菜单管理
    -   [x] 部门管理
    -   [x] 字典管理
    -   [x] 参数管理 
-   [x] **权限体系**
    -   [x] 声明式权限校验过程宏 (`ruoyi-macros`)
-   [ ] **待办核心模块**
    -   [ ] 角色管理
    -   [x] 用户管理
    -   [x] 系统通知
-   [ ] **监控模块 (`monitor`)**
    -   [x] 操作日志
    -   [x] 登录日志
    -   [ ] 在线用户
    -   [x] 定时任务

## 🛠️ 技术栈 (Tech Stack)

| 领域            | 技术选型                | 理由                                                     |
| --------------- | ----------------------- | -------------------------------------------------------- |
| **语言/运行时** | Rust (Stable) / Tokio   | 性能、安全、现代化的异步生态。                           |
| **Web 框架**    | Axum                    | 模块化、符合人体工程学、与 `tower` 生態無縫集成。        |
| **数据库交互**  | SQLx                    | 异步、类型安全、编译时 SQL 检查，完美契合原生 SQL 风格。 |
| **序列化**      | Serde                   | Rust 生态的事实标准，性能卓越，功能强大。                |
| **配置管理**    | `config-rs`             | 支持多种格式，支持环境覆盖，简单易用。                   |
| **日志系统**    | `tracing`               | 结构化日志，与 `tokio` 和 `axum` 深度集成。              |
| **认证/授权**   | `argon2` / `jwt-simple` | 现代密码哈希标准；纯 Rust 实现的 JWT 库，轻量安全。      |
| **缓存**        | `moka`                  | 高性能纯 Rust 本地并发缓存，完美替代单体应用中的 Redis。 |
| **权限宏**      | `syn`, `quote`          | 自定义过程宏，实现声明式、编译时安全的权限校验。         |

## 🚀 快速开始

请确保您的开发环境中已安装以下软件：
*   **Rust**: `1.88` 或更高版本 (通过 `rustup` 安装)
*   **MySQL**: `5.7` 或更高版本
*   **`sqlx-cli`**: (用于编译时 SQL 检查)
    ```bash
    cargo install sqlx-cli
    ```

### 1. 克隆项目

```bash
git clone https://gitee.com/rustdev/ruoyi-rust.git
cd ruoyi-rust
```

### 2. 数据库设置

1.  在您的 MySQL 实例中创建一个新的数据库，例如 `ry-vue`。
2.  将项目根目录下的 `sql/ry_20250808.sql` 文件导入到您刚创建的数据库中。

### 3. 后端配置(重要！重要！重要！)

1.  复制环境配置文件 （如果已有.env则跳过此步骤）：
    ```bash
    cp .env_ .env
    ```
2.  编辑 `.env` 文件，修改 `DATABASE_URL` 以匹配您的数据库连接信息：
    ```
    DATABASE_URL="mysql://your_user:your_password@127.0.0.1:3306/ry-vue"
    ```

3.  编辑 `config/default.toml` 文件，修改 `url` 以匹配您的数据库连接信息：
    ```
    url = "mysql://your_user:your_password@127.0.0.1:3306/ry-vue?connect_timeout=8"
    ```



### 4. 编译时检查与运行

这是 `sqlx` 的关键步骤。它会在编译前连接数据库，验证所有 SQL 查询的正确性，并将元数据保存在 `sqlx-data.json` 文件中。

1.  **生成元数据文件**(可选步骤，如果mysql已经启动且可正常连接，可以跳过此步):
    ```bash
    cargo sqlx prepare --workspace
    ```
    *如果成功，您会在项目根目录看到一个 `sqlx-data.json` 文件。*

2.  **运行后端服务**:
    ```bash
    cargo run -p app
    ```
    *看到 "✅ Server listening on 127.0.0.1:8080" 日志表示后端启动成功。*

### 5. 前端设置

1.  克隆 [RuoYi-Vue3] 前端项目。
    ```bash
    git clone https://gitee.com/rustdev/ruo-yi-vue3.git
    cd ruo-yi-vue3
    ```
2.  进入前端项目目录，安装依赖：
    ```bash
    npm install
    ```
3.  运行前端开发服务器：
    ```bash
    npm run dev
    ```
4.  打开浏览器访问 `http://localhost:80` (或您终端中显示的地址)。
    *   **默认登录账号**: `admin`
    *   **默认密码**: `admin123`

## 🏗️ 项目结构

本项目采用 Cargo Workspace 进行模块化管理，结构清晰，职责分明。

-   `ruoyi-rust/` (Workspace 根目录)
    -   `app`: **主程序入口**。负责组装所有模块、初始化服务、启动 Web 服务器。
    -   `common`: **通用基础库**。存放跨模块共享的组件（如 `AppError`, `AjaxResult`, 共享模型等）。
    -   `framework`: **核心框架库**。提供基础设施服务（如配置、数据库、日志、JWT、中间件）。
    -   `modules/`: **业务模块聚合**。
        -   `system`: **核心系统模块**。实现 RuoYi 的所有核心后台管理功能。
        -   `monitor`: **系统监控模块**。
    -   `ruoyi-macros`: **过程宏库**。存放自定义的过程宏，如 `#[require_permission(...)]`。

##🤝 贡献指南

我们热烈欢迎任何形式的贡献！无论是提交 Issue、修复 Bug 还是实现新功能。请参考我们的 [CONTRIBUTING.md](CONTRIBUTING.md) 文件了解详细的贡献流程和编码规范。

##📄 许可证 (License)

本项目采用 [MIT License](LICENSE) 开源许可证。

## 🙏 鸣谢

*   感谢原项目 [RuoYi-Vue](https://gitee.com/y_project/RuoYi-Vue) 提供的优秀设计和前端实现。
*   感谢 [RuoYi-Vue3](https://gitcode.com/yangzongzhuan/RuoYi-Vue3) 提供的 Vue3 前端版本。
*   演示地址：# RuoYi-Rust  高性能 Rust 重构后端 
  **         丰盛辉煌**

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md) 

---
有任何需求，请直接在requests中提出，我有时间会慢慢的补充上
---
---
**RuoYi-Rust** 是一个雄心勃勃的项目，旨在通过现代、高性能的 Rust 语言及其强大的生态系统，完整重写主流 Java Web 框架 [RuoYi](https://gitee.com/y_project/RuoYi) 的后端服务。我们追求的目标不仅是功能对等，更是在性能、资源占用(目前仅几M的内存占用)、安全性及现代化开发体验上实现全面超越，为Rust生态提供一个可复用的管理系统。

本项目后端与 [RuoYi-Vue3(rust后台配套微调版)](https://gitee.com/rustdev/ruo-yi-vue3.git) 前端项目配套使用。

## ✨ 核心亮点 (Why RuoYi-Rust?)

*   🚀 **极致性能**: 基于 `Tokio` 和 `Axum` 框架，原生异步，无 GC 延迟。提供比原生 JVM 应用更快的 API 响应速度和显著降低的内存占用。

*   🔒 **内存安全**: 借助 Rust 强大的所有权和借用检查机制，从编译层面根除空指针、数据竞争等常见的运行时安全隐患，构建坚如磐石的系统。

*   📦 **极简部署**: 整个后端项目可编译为单个可执行文件(内置缓存系统)，不依赖 JRE、Tomcat、Nginx 等任何外部运行时。部署过程从未如此简单：上传文件，启动，仅此而已。

*   🛠️ **现代化开发体验**:
    *   **编译时 SQL 校验**: 采用 `sqlx` 作为数据库交互层，遵循“数据库 Schema 为唯一真实来源”原则。任何不符合数据库表的 SQL 查询都将在编译时被发现，杜绝大量潜在的数据库运行时错误。
    *   **声明式权限控制**: 通过自定义过程宏实现类型安全的声明式 API 权限校验，将权限元数据与业务逻辑高度内聚，彻底告别手写权限字符串带来的风险。
    *   **模块化设计**: 采用清晰的 Cargo Workspace 结构和分层架构，实现高内聚、低耦合，便于维护与扩展。

## 📊 项目状态 (截至 2025-08-28)

我们正在逐步实现 RuoYi 的全部功能。当前已完成的核心模块如下：

-   [x] **核心框架**
    -   [x] 配置加载 (`config-rs`)
    -   [x] 结构化日志 (`tracing`)
    -   [x] 数据库连接池 (`sqlx`)
    -   [x] 统一响应 (`AjaxResult`) 与错误处理 (`AppError`)
    -   [x] JWT 认证中间件 (`jwt-simple`)
    -   [x] 高性能本地缓存 (`moka`)
-   [x] **系统模块 (`system`)**
    -   [x] 登录、获取用户信息、动态路由 (`/login`, `/getInfo`, `/getRouters`)
    -   [x] 菜单管理
    -   [x] 部门管理
    -   [x] 字典管理
    -   [x] 参数管理 
-   [x] **权限体系**
    -   [x] 声明式权限校验过程宏 (`ruoyi-macros`)
-   [ ] **待办核心模块**
    -   [ ] 角色管理
    -   [x] 用户管理
    -   [x] 系统通知
-   [ ] **监控模块 (`monitor`)**
    -   [x] 操作日志
    -   [x] 登录日志
    -   [ ] 在线用户
    -   [x] 定时任务

## 🛠️ 技术栈 (Tech Stack)

| 领域            | 技术选型                | 理由                                                     |
| --------------- | ----------------------- | -------------------------------------------------------- |
| **语言/运行时** | Rust (Stable) / Tokio   | 性能、安全、现代化的异步生态。                           |
| **Web 框架**    | Axum                    | 模块化、符合人体工程学、与 `tower` 生態無縫集成。        |
| **数据库交互**  | SQLx                    | 异步、类型安全、编译时 SQL 检查，完美契合原生 SQL 风格。 |
| **序列化**      | Serde                   | Rust 生态的事实标准，性能卓越，功能强大。                |
| **配置管理**    | `config-rs`             | 支持多种格式，支持环境覆盖，简单易用。                   |
| **日志系统**    | `tracing`               | 结构化日志，与 `tokio` 和 `axum` 深度集成。              |
| **认证/授权**   | `argon2` / `jwt-simple` | 现代密码哈希标准；纯 Rust 实现的 JWT 库，轻量安全。      |
| **缓存**        | `moka`                  | 高性能纯 Rust 本地并发缓存，完美替代单体应用中的 Redis。 |
| **权限宏**      | `syn`, `quote`          | 自定义过程宏，实现声明式、编译时安全的权限校验。         |

## 🚀 快速开始

请确保您的开发环境中已安装以下软件：
*   **Rust**: `1.88` 或更高版本 (通过 `rustup` 安装)
*   **MySQL**: `5.7` 或更高版本
*   **`sqlx-cli`**: (用于编译时 SQL 检查)
    ```bash
    cargo install sqlx-cli
    ```

### 1. 克隆项目

```bash
git clone https://gitee.com/rustdev/ruoyi-rust.git
cd ruoyi-rust
```

### 2. 数据库设置

1.  在您的 MySQL 实例中创建一个新的数据库，例如 `ry-vue`。
2.  将项目根目录下的 `sql/ry_20250808.sql` 文件导入到您刚创建的数据库中。

### 3. 后端配置(重要！重要！重要！)

1.  复制环境配置文件 （如果已有.env则跳过此步骤）：
    ```bash
    cp .env_ .env
    ```
2.  编辑 `.env` 文件，修改 `DATABASE_URL` 以匹配您的数据库连接信息：
    ```
    DATABASE_URL="mysql://your_user:your_password@127.0.0.1:3306/ry-vue"
    ```

3.  编辑 `config/default.toml` 文件，修改 `url` 以匹配您的数据库连接信息：
    ```
    url = "mysql://your_user:your_password@127.0.0.1:3306/ry-vue?connect_timeout=8"
    ```



### 4. 编译时检查与运行

这是 `sqlx` 的关键步骤。它会在编译前连接数据库，验证所有 SQL 查询的正确性，并将元数据保存在 `sqlx-data.json` 文件中。

1.  **生成元数据文件**(可选步骤，如果mysql已经启动且可正常连接，可以跳过此步):
    ```bash
    cargo sqlx prepare --workspace
    ```
    *如果成功，您会在项目根目录看到一个 `sqlx-data.json` 文件。*

2.  **运行后端服务**:
    ```bash
    cargo run -p app
    ```
    *看到 "✅ Server listening on 127.0.0.1:8080" 日志表示后端启动成功。*

### 5. 前端设置

1.  克隆 [RuoYi-Vue3] 前端项目。
    ```bash
    git clone https://gitee.com/rustdev/ruo-yi-vue3.git
    cd ruo-yi-vue3
    ```
2.  进入前端项目目录，安装依赖：
    ```bash
    npm install
    ```
3.  运行前端开发服务器：
    ```bash
    npm run dev
    ```
4.  打开浏览器访问 `http://localhost:80` (或您终端中显示的地址)。
    *   **默认登录账号**: `admin`
    *   **默认密码**: `admin123`

## 🏗️ 项目结构

本项目采用 Cargo Workspace 进行模块化管理，结构清晰，职责分明。

-   `ruoyi-rust/` (Workspace 根目录)
    -   `app`: **主程序入口**。负责组装所有模块、初始化服务、启动 Web 服务器。
    -   `common`: **通用基础库**。存放跨模块共享的组件（如 `AppError`, `AjaxResult`, 共享模型等）。
    -   `framework`: **核心框架库**。提供基础设施服务（如配置、数据库、日志、JWT、中间件）。
    -   `modules/`: **业务模块聚合**。
        -   `system`: **核心系统模块**。实现 RuoYi 的所有核心后台管理功能。
        -   `monitor`: **系统监控模块**。
    -   `ruoyi-macros`: **过程宏库**。存放自定义的过程宏，如 `#[require_permission(...)]`。

##🤝 贡献指南

我们热烈欢迎任何形式的贡献！无论是提交 Issue、修复 Bug 还是实现新功能。请参考我们的 [CONTRIBUTING.md](CONTRIBUTING.md) 文件了解详细的贡献流程和编码规范。

##📄 许可证 (License)

本项目采用 [MIT License](LICENSE) 开源许可证。

## 🙏 鸣谢

*   感谢原项目 [RuoYi-Vue](https://gitee.com/y_project/RuoYi-Vue) 提供的优秀设计和前端实现。
*   感谢 [RuoYi-Vue3](https://gitcode.com/yangzongzhuan/RuoYi-Vue3) 提供的 Vue3 前端版本。
*   演示 https://demo.ruoyi.vip/index
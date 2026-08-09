# C++ 构建、依赖与工具链配置

## When To Use

仅当任务拥有 CMake/构建配置、语言标准/工具链变更、依赖管理、生成代码、测试/分析目标、安装/导出/打包规则或构建迁移时才使用此参考。

## Implementation Focus

### Discover The Build Graph

在编辑之前检查实际入口点、preset/toolchain 文件、生成器、目标、子目录、选项、包管理器、生成源码、测试、安装/导出、CI 矩阵和受支持的编译器/平台。

不要为单个功能用模板替换连贯的 CMake、Bazel、Meson、Make、IDE、包管理器或嵌入式构建。

保持源码和二进制目录分开，避免提交本地构建树/缓存/生成机器路径。

### Target-Scoped CMake

使用目标级源码、包含目录、编译特性/定义/选项、链接库/选项、生成文件和属性。全局标志/目录会将行为泄露到依赖/测试/消费者。

根据消费者契约表达 PUBLIC/PRIVATE/INTERFACE。公共头依赖/特性/定义必须传播；仅实现细节不得传播。使用 `target_compile_features`、`target_include_directories` 和 `target_link_libraries` 配合匹配的可见性，而非等效的目录全局状态。

优先使用编译特性表示语言标准并保持扩展策略显式。不要将全局 `CMAKE_CXX_STANDARD` 假设与声明另一级别的目标混用。

对配置/编译器/平台特定行为使用生成器表达式，而非错误折叠多配置生成器的配置时分支。

### Presets And Toolchains

保持开发者/CI preset 可复现且无本地绝对路径。Toolchain 文件在项目配置之前拥有编译器/sysroot/目标/包管理器集成。

仅根据仓库矩阵支持 Ninja/Make/Visual Studio/Xcode 或交叉编译。避免在可移植自定义命令中使用 Unix shell 命令/路径。

多配置构建使用 `--config`；单配置构建使用 `CMAKE_BUILD_TYPE`。不要在共享脚本中假设其中一个。

### Dependencies

保留 Conan/vcpkg/FetchContent/系统/submodule/vendor 所有权和锁定/版本策略。不要引入第二个管理器或在配置期间下载未固定的移动分支。

当包提供导入目标时使用它们而非全局 include/link 变量。按平台验证静态/共享/运行时库和传递依赖行为。

在添加包之前检查许可证、受支持的编译器/运行时、C++ ABI/运行时链接和离线/可复现构建要求。

### Generated Code And Custom Commands

声明确切的输入/输出/副产品/依赖，使用目标感知命令，创建目录，并避免并行目标之间的竞争。生成的头需要正确的二进制 include 路径和先构建后使用依赖。

引用路径/列表参数并使用 `VERBATIM`。当构建时依赖应触发重新生成时不要在配置时运行生成器。

### Warnings, Sanitizers, And Analysis

按拥有的目标和编译器应用仓库警告策略。不要意外将第三方头变为警告即错误。

Sanitizer 需要兼容的编译和链接标志、所需的调试信息/帧指针以及专用 preset/选项。不要强制 ASan/UBSan/TSan/MSan 进入发布/分发或组合不兼容的 sanitizer。

在支持的地方为 clang-tidy/IWYU 生成 `compile_commands.json` 或等效文件，并将检查范围限定到项目目标/文件。

### Tests And Benchmarks

通过现有 CTest/Catch2/GoogleTest/自定义约定注册测试，包含工作目录、环境、标签、夹具、资源和超时。

基准测试保持与正确性测试和正常启动分开。分析/测试依赖不应泄露到生产接口中。

### Install, Export, And Packaging

对于可消费的库，保留构建/安装 include 接口、导出命名空间/目标、版本/配置文件、组件/运行时目标、RPATH/运行时 DLL 处理、符号可见性和 CPack/包管理器元数据。

可重定位包不得嵌入源码/构建机器绝对路径。当公共打包变更时通过小型外部消费者测试安装。

## Verification Focus

- 从干净的构建目录使用受影响的 preset/生成器/toolchain 配置。
- 构建确切的变更目标和受影响的编译器/平台/配置矩阵。
- 运行注册的测试/分析/sanitizer 目标并验证发现，而非仅手动可执行调用。
- 在依赖/生成输入变更后重新配置以证明可复现性和并行排序。
- 当公共交付变更时用外部消费者测试安装/导出/打包。

## Evidence Focus

说明目标/属性可见性、toolchain/preset/依赖/生成/安装决策和干净配置/构建/测试证明。增量本地构建成功不证明可复现性、传播、打包或跨平台行为。

## Unsafe Defaults

- 用于目标局部行为的全局编译/链接标志。
- 配置期间下载的移动/未固定依赖。
- preset/缓存/导出文件中提交的机器特定路径。
- 生成输出缺少声明的依赖/副产品。
- 强制进入普通发布构建的 sanitizer。
- 嵌入源码/构建路径的已安装目标。

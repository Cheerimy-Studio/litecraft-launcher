# LiteCraft Launcher

Minecraft 26.3 的 Rust 客户端 **LiteCraft** 的启动器。基于 PCL 源代码的第三方修改版，非 PCL 官方发布。

原作者：龙腾猫跃（PCL）。

## 流程

- 启动、下载、安装、设置、更多等页面使用 PCL 原版页面。
- 原版游戏的版本列表来自 `manifests/versions.json`，不再读取 Mojang 的完整版本列表。
- 配置会依次尝试 GitHub 官方 raw 地址与镜像；启动器内置配置的 SHA-256，内容不一致时拒绝使用，避免镜像劫持。
- 选择 LiteCraft 26.3 后进入 PCL 原版安装页。当前只支持原版安装，Forge、Fabric、OptiFine 等附加组件暂时隐藏。
- 安装时由 PCL 原生流程下载 26.3 的客户端、依赖库和资源文件，进度与 PCL 一致。
- 原版文件下载完成后，安装任务会继续显示“下载 LiteCraft 核心文件”“解压 LiteCraft 核心文件”“准备 LiteCraft 资源”等步骤；核心文件来自 GitHub Release 并校验 SHA-256，资源准备复用 PCL 已下载的客户端、依赖库和资源文件。
- 资源就绪后，LiteCraft 26.3 自动加入版本列表；在启动页选择它即可启动 Rust 客户端。
- 游戏用户名必须填写；为空时不会启动。
- LiteCraft 启动沿用 PCL 的窗口大小和全屏设置；视距在游戏内调整，上限 512 区块。
- 标题栏和图标使用 LiteCraft 品牌。

## 目录

```
pcl/        修改后的 PCL 源码（VB.NET / .NET Framework 4.8）
backend/    LiteCraft 资源准备与启动后端（Rust）
manifests/  26.3 文件清单
```

## 构建

需要 Windows、.NET SDK 9、.NET Framework 4.8 目标包、Visual Studio 的 VB/MSBuild，以及 Rust。

```powershell
./build.ps1
```

产物：

- `pcl/Plain Craft Launcher 2/bin/Plain Craft Launcher 2.exe`
- `backend/launcher/target/release/litecraft-launther.exe`

把 `litecraft.exe` 和准备好的 `resources/26.3` 放到启动器旁的 `LiteCraft/` 目录。

## 26.3 文件更新

`manifests/26.3.json` 记录固定版本的客户端、服务端、资源索引的 URL 和 SHA-1。`.github/workflows/update-26.3.yml` 会定期对照 `backend/research-lock.json` 校验并更新。

`manifests/versions.json` 是启动器读取的版本列表，包含 26.3 的版本 JSON 地址和 LiteCraft 核心文件的下载地址、镜像、SHA-256 与大小。启动器内置这份配置的 SHA-256；修改 `versions.json` 后必须重新构建启动器并发布，否则启动器会按预期拒绝新配置。

发布新核心时：

1. 打包 `litecraft.exe`、`LiteCraft-Launther.exe`、`prepare/`、`version/26.3.json` 为 `LiteCraft-Core-26.3.zip`，不要包含资源或 Minecraft 素材。
2. 上传到 GitHub Release，更新 `manifests/versions.json` 里的地址、SHA-256 和大小。
3. 计算新配置的 SHA-256，更新 `pcl/Plain Craft Launcher 2/Modules/Minecraft/ModLiteCraft.vb` 的 `VersionsConfigSha256`，重新构建启动器。

## 素材

本仓库和发布包不包含 Minecraft 素材。资源准备时从 Mojang 的公开地址下载客户端、依赖、资源与服务端，并在本机运行 `horizon-prepare` 生成 LiteCraft 资源。Minecraft 的代码、素材与商标属于 Mojang 与 Microsoft。

## 许可

- PCL 部分遵循仓库中的 `LICENCE`。
- LiteCraft 新增代码按 MIT 提供，见 `LICENSE-LITECRAFT.txt`。
- 第三方声明见 `NOTICE.md`。

命名说明：PCL 的分发指南要求重度修改版的名称以 Plain Craft Launcher (PCL) 开头。本项目按需求使用 LiteCraft Launcher 作为产品名，并在关于页保留龙腾猫跃的署名与赞助链接；如需严格符合该指南，请将产品名改为 PCL LiteCraft Launcher。

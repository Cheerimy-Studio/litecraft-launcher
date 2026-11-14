# LiteCraft Launcher

Minecraft 26.3 的 Rust 客户端 **LiteCraft** 的启动器。基于 PCL 源代码的第三方修改版，非 PCL 官方发布。

原作者：龙腾猫跃（PCL）。

## 流程

- 启动、下载、安装、附加包选择、设置、更多等页面全部使用 PCL 原版页面。
- 下载页保留原版游戏、Mod、整合包、资源包、光影包、数据包；原版游戏里可选择版本和 Forge/Fabric 等附加包，安装进度与 PCL 一致。
- LiteCraft 26.3 资源已就绪时，启动器自动把它加入版本列表；在启动页选择 LiteCraft 26.3 即可启动 Rust 客户端。
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

## 素材

本仓库和发布包不包含 Minecraft 素材。资源准备时从 Mojang 的公开地址下载客户端、依赖、资源与服务端，并在本机运行 `horizon-prepare` 生成 LiteCraft 资源。Minecraft 的代码、素材与商标属于 Mojang 与 Microsoft。

## 许可

- PCL 部分遵循仓库中的 `LICENCE`。
- LiteCraft 新增代码按 MIT 提供，见 `LICENSE-LITECRAFT.txt`。
- 第三方声明见 `NOTICE.md`。

命名说明：PCL 的分发指南要求重度修改版的名称以 Plain Craft Launcher (PCL) 开头。本项目按需求使用 LiteCraft Launcher 作为产品名，并在关于页保留龙腾猫跃的署名与赞助链接；如需严格符合该指南，请将产品名改为 PCL LiteCraft Launcher。

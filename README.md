# LiteCraft Launcher

Minecraft 26.3 的 Rust 客户端 **LiteCraft** 的启动器。基于 PCL 源代码的第三方修改版，非 PCL 官方发布。

原作者：龙腾猫跃（PCL）。修改与 LiteCraft 相关部分由 LiteCraft 项目维护。

## 功能

- 启动页直接启动 LiteCraft 26.3，使用 PCL 的版本列表、日志和启动流程。
- 游戏用户名必须填写，空名字不会启动。
- 下载页只保留 LiteCraft 26.3，安装后自动加入版本列表；资源已就绪时可直接启动。
- 版本设置中的 LiteCraft 页：视距（2–512）、打开游戏目录、重新准备资源。
- 视距上限 512 区块，游戏内滑块 Shift+点击可跳到最大值。
- 更多页只保留“关于”；Mod、整合包、资源包、光影、数据包、百宝箱、反馈、投票、帮助等入口已隐藏。
- 标题栏与图标使用 LiteCraft 品牌。

## 目录

```
pcl/        修改后的 PCL 源码（VB.NET / .NET Framework 4.8）
backend/    LiteCraft 下载、资源准备与启动后端（Rust）
manifests/  26.3 文件清单与启动器版本信息
```

## 构建

需要 Windows、.NET SDK 9、.NET Framework 4.8 目标包、Visual Studio 的 VB/MSBuild，以及 Rust。

```powershell
./build.ps1
```

产物：

- `pcl/Plain Craft Launcher 2/bin/Plain Craft Launcher 2.exe`
- `backend/launcher/target/release/litecraft-launther.exe`

把 `litecraft.exe` 和准备好的 `resources/26.3` 放到启动器旁的 `LiteCraft/` 目录，或者让启动器在下载页自动下载并准备。

## 26.3 文件更新

`manifests/26.3.json` 记录固定版本的客户端、服务端、资源索引的 URL 和 SHA-1。更新由 `.github/workflows/update-26.3.yml` 定期执行，脚本会对照 `backend/research-lock.json` 校验，不会接受未知的版本文件。

## 素材

本仓库和发布包不包含 Minecraft 素材。启动器首次准备时从 Mojang 的公开地址下载客户端、依赖、资源与服务端，并在本机运行 `horizon-prepare` 生成 LiteCraft 资源。Minecraft 的代码、素材与商标属于 Mojang 与 Microsoft。

## 许可

- PCL 部分遵循仓库中的 `LICENCE`（PCL 分发有限许可与合理使用指南）。
- LiteCraft 新增代码按 MIT 提供，见 `LICENSE-LITECRAFT.txt`。
- 第三方声明见 `NOTICE.md`。

命名说明：PCL 的分发指南要求重度修改版的名称以 Plain Craft Launcher (PCL) 开头。本项目按需求使用 LiteCraft Launcher 作为产品名，并在关于页保留龙腾猫跃的署名与赞助链接；如需严格符合该指南，请将产品名改为 PCL LiteCraft Launcher。

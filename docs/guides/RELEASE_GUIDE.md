# GitHub Actions 自动化发布指南

本文档说明如何使用 GitHub Actions 自动构建和发布热轧精整排产系统的安装包。

---

## 📋 目录

- [工作流说明](#工作流说明)
- [准备工作](#准备工作)
- [发布新版本](#发布新版本)
- [常见问题](#常见问题)
- [故障排查](#故障排查)

---

## 🔄 工作流说明

项目包含两个 GitHub Actions 工作流：

### 1. Release 工作流 ([.github/workflows/release.yml](.github/workflows/release.yml))

**触发条件**：
- 推送版本标签（如 `v1.1.0`）
- 手动触发（在 GitHub Actions 页面）

**功能**：
- 自动构建 Windows、macOS、Linux 三个平台的安装包
- 创建 GitHub Release
- 上传所有安装包到 Release

**生成的安装包**：
- **Windows**: `.exe` (NSIS 安装器)
- **macOS**: `.dmg` (通用二进制，支持 Intel 和 Apple Silicon)
- **Linux**: `.AppImage` 和 `.deb`

### 2. Build & Test 工作流 ([.github/workflows/build.yml](.github/workflows/build.yml))

**触发条件**：
- Pull Request 到 main 分支
- 推送到 main 分支

**功能**：
- 前端测试和类型检查
- 后端 Rust 测试和 Clippy 检查
- 多平台构建验证（调试模式）
- 测试覆盖率报告

---

## 🛠️ 准备工作

### 步骤 1：准备应用图标

在 `icons/` 目录下准备以下图标文件：

```bash
icons/
├── 32x32.png          # 32x32 像素 PNG
├── 128x128.png        # 128x128 像素 PNG
├── 128x128@2x.png     # 256x256 像素 PNG (Retina)
├── icon.icns          # macOS 图标
├── icon.ico           # Windows 图标
└── icon.png           # 源图标（建议 512x512 或更大）
```

#### 图标生成工具

**方法 1：使用 Tauri 图标生成器**
```bash
# 安装 @tauri-apps/cli
npm install -g @tauri-apps/cli

# 从源图标生成所有尺寸
tauri icon icons/icon.png
```

**方法 2：在线工具**
- [Icon Generator](https://icon.kitchen/)
- [App Icon Generator](https://appicon.co/)

**方法 3：手动创建**
- macOS: 使用 `iconutil` 命令
- Windows: 使用 ImageMagick 或在线转换工具

### 步骤 2：验证配置

确认 [tauri.conf.json](tauri.conf.json) 中的图标路径正确：

```json
{
  "tauri": {
    "bundle": {
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ]
    }
  }
}
```

### 步骤 3：本地测试构建

在推送到 GitHub 之前，建议先本地测试构建：

```bash
# 安装依赖
npm install

# 运行测试
npm run test -- --run

# 构建应用（调试模式，速度快）
npm run tauri build -- --debug

# 构建应用（发布模式，生成优化的安装包）
npm run tauri build
```

---

## 🚀 发布新版本

### 方法 1：推送版本标签（推荐）

这是最常用的发布方式，适合正式版本发布。

#### 步骤：

1. **更新版本号**

   编辑以下文件中的版本号：
   - [package.json](package.json) - `version` 字段
   - [tauri.conf.json](tauri.conf.json) - `package.version` 字段
   - [Cargo.toml](Cargo.toml) - `version` 字段
   - [README.md](README.md) - 版本信息
   - [CHANGELOG.md](CHANGELOG.md) - 添加新版本的更新日志

2. **提交版本变更**

   ```bash
   git add package.json tauri.conf.json Cargo.toml README.md CHANGELOG.md
   git commit -m "chore: 发布 v1.2.0 版本"
   ```

3. **创建并推送标签**

   ```bash
   # 创建标签
   git tag -a v1.2.0 -m "Release v1.2.0"

   # 推送提交和标签
   git push origin main
   git push origin v1.2.0
   ```

4. **等待构建完成**

   - 访问 GitHub 仓库的 Actions 页面
   - 查看 "Release" 工作流的运行状态
   - 构建通常需要 15-30 分钟

5. **验证 Release**

   - 访问仓库的 Releases 页面
   - 确认新版本已创建
   - 下载并测试安装包

### 方法 2：手动触发

适合测试或特殊情况下的发布。

#### 步骤：

1. 访问 GitHub 仓库的 Actions 页面
2. 选择 "Release" 工作流
3. 点击 "Run workflow" 按钮
4. 输入版本号（如 `v1.2.0`）
5. 点击 "Run workflow" 确认

---

## 📦 安装包说明

### Windows 安装包

**文件名格式**：`hot-rolling-aps_v1.2.0_windows_x64-setup.exe`

**特点**：
- NSIS 安装器
- 支持静默安装：`setup.exe /S`
- 自动创建桌面快捷方式
- 支持卸载

### macOS 安装包

**文件名格式**：`hot-rolling-aps_v1.2.0_macos_universal.dmg`

**特点**：
- 通用二进制（Universal Binary）
- 同时支持 Intel 和 Apple Silicon (M1/M2/M3)
- 拖拽安装
- 首次运行需要在"系统偏好设置 > 安全性与隐私"中允许

### Linux 安装包

**AppImage 格式**：`hot-rolling-aps_v1.2.0_linux_amd64.AppImage`
- 无需安装，直接运行
- 需要添加执行权限：`chmod +x *.AppImage`

**Debian 格式**：`hot-rolling-aps_v1.2.0_linux_amd64.deb`
- 适用于 Ubuntu、Debian 等系统
- 安装：`sudo dpkg -i *.deb`

---

## ❓ 常见问题

### Q1: 构建失败怎么办？

**A**: 查看 Actions 日志，常见原因：
- 图标文件缺失或格式错误
- 依赖安装失败
- 测试未通过
- Rust 编译错误

### Q2: 如何只构建特定平台？

**A**: 修改 [.github/workflows/release.yml](.github/workflows/release.yml) 中的 `matrix.include`，注释掉不需要的平台。

### Q3: 构建时间太长？

**A**:
- 首次构建需要下载依赖，较慢（15-30分钟）
- 后续构建有缓存，会快很多（5-10分钟）
- 可以只构建当前平台进行测试

### Q4: 如何添加代码签名？

**A**: 需要配置 GitHub Secrets：

**macOS 签名**：
```yaml
# 在 GitHub 仓库设置中添加 Secrets
APPLE_CERTIFICATE: <base64 编码的证书>
APPLE_CERTIFICATE_PASSWORD: <证书密码>
APPLE_ID: <Apple ID>
APPLE_PASSWORD: <应用专用密码>
```

**Windows 签名**：
```yaml
WINDOWS_CERTIFICATE: <base64 编码的证书>
WINDOWS_CERTIFICATE_PASSWORD: <证书密码>
```

然后在工作流中添加签名步骤。

### Q5: 如何发布预发布版本？

**A**:
1. 使用 `-beta`、`-rc` 等后缀的标签：`v1.2.0-beta.1`
2. 修改工作流中的 `prerelease: true`

### Q6: 安装包太大怎么办？

**A**:
- 检查是否包含了不必要的依赖
- 使用 `cargo bloat` 分析二进制大小
- 考虑使用 `strip` 移除调试符号
- 启用 LTO（已在 Cargo.toml 中配置）

---

## 🔧 故障排查

### 构建失败：图标相关错误

**错误信息**：`Error: Icon file not found`

**解决方案**：
1. 确认 `icons/` 目录下有所有必需的图标文件
2. 检查 [tauri.conf.json](tauri.conf.json) 中的路径是否正确
3. 使用 `tauri icon` 命令重新生成图标

### 构建失败：依赖安装错误

**错误信息**：`npm ERR!` 或 `cargo build failed`

**解决方案**：
1. 检查 [package.json](package.json) 和 [Cargo.toml](Cargo.toml) 中的依赖版本
2. 本地运行 `npm ci` 和 `cargo build` 验证
3. 清除缓存后重试

### 构建失败：测试未通过

**错误信息**：`Test failed`

**解决方案**：
1. 本地运行 `npm run test -- --run` 和 `cargo test`
2. 修复失败的测试
3. 确保所有测试通过后再推送

### macOS 安装包无法打开

**错误信息**：`"应用程序"已损坏，无法打开`

**原因**：未签名的应用

**解决方案**：
- 用户：右键点击 > 打开，或在"系统偏好设置"中允许
- 开发者：配置代码签名（见 Q4）

### Windows 安装包被杀毒软件拦截

**原因**：未签名的可执行文件

**解决方案**：
- 用户：添加信任或临时关闭杀毒软件
- 开发者：购买代码签名证书并配置签名

---

## 📚 相关资源

- [Tauri 官方文档](https://tauri.app/)
- [GitHub Actions 文档](https://docs.github.com/actions)
- [Tauri Action](https://github.com/tauri-apps/tauri-action)
- [项目 README](README.md)
- [更新日志](CHANGELOG.md)

---

## 📞 获取帮助

如果遇到问题：

1. 查看 GitHub Actions 的构建日志
2. 搜索 [Tauri Discussions](https://github.com/tauri-apps/tauri/discussions)
3. 在项目中创建 Issue

---

**版本**: 1.0
**最后更新**: 2026-02-10
**维护者**: 项目团队

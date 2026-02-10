# 🚀 GitHub Actions 自动化部署 - 快速开始

## ✅ 已完成的配置

我已经为你创建了完整的 GitHub Actions 自动化部署配置：

### 📁 新增文件

1. **[.github/workflows/release.yml](.github/workflows/release.yml)**
   - 自动构建和发布工作流
   - 支持 Windows、macOS、Linux 三平台
   - 自动创建 GitHub Release

2. **[.github/workflows/build.yml](.github/workflows/build.yml)**
   - PR 和 main 分支的构建测试
   - 前端和后端测试
   - 多平台构建验证

3. **[docs/guides/RELEASE_GUIDE.md](docs/guides/RELEASE_GUIDE.md)**
   - 完整的发布流程文档
   - 常见问题解答
   - 故障排查指南

4. **[icons/ICON_GUIDE.md](icons/ICON_GUIDE.md)**
   - 图标准备指南
   - 图标生成方法
   - 验证步骤

### 🔧 修改的文件

- **[tauri.conf.json](tauri.conf.json)** - 更新了图标配置路径

---

## 📋 后续步骤

### 第 1 步：准备应用图标 ⚠️ **必需**

在推送到 GitHub 之前，必须先准备图标文件。

**快速方法**（推荐）：

```bash
# 如果你有源图标（icon.png），使用 Tauri 自动生成
npm install -g @tauri-apps/cli
tauri icon icons/icon.png
```

**详细说明**：查看 [icons/ICON_GUIDE.md](icons/ICON_GUIDE.md)

**需要的文件**：
- `icons/32x32.png`
- `icons/128x128.png`
- `icons/128x128@2x.png`
- `icons/icon.icns` (macOS)
- `icons/icon.ico` (Windows)

### 第 2 步：提交配置文件

```bash
# 添加所有新文件
git add .github/ docs/guides/RELEASE_GUIDE.md icons/ tauri.conf.json

# 提交
git commit -m "ci: 添加 GitHub Actions 自动化部署配置

- 添加 release.yml 多平台构建发布工作流
- 添加 build.yml PR 构建测试工作流
- 更新 tauri.conf.json 图标配置
- 添加发布流程和图标准备文档"

# 推送到 GitHub
git push origin main
```

### 第 3 步：测试构建（可选但推荐）

在正式发布前，先测试 PR 构建流程：

```bash
# 创建测试分支
git checkout -b test/ci-setup

# 推送到 GitHub
git push origin test/ci-setup

# 在 GitHub 上创建 PR，观察构建是否成功
```

### 第 4 步：发布第一个版本

当一切准备就绪，发布你的第一个版本：

```bash
# 1. 更新版本号（如果需要）
# 编辑 package.json, tauri.conf.json, Cargo.toml, README.md, CHANGELOG.md

# 2. 提交版本变更
git add .
git commit -m "chore: 发布 v1.1.0 版本"

# 3. 创建并推送标签
git tag -a v1.1.0 -m "Release v1.1.0"
git push origin main
git push origin v1.1.0

# 4. 等待构建完成（15-30 分钟）
# 访问 https://github.com/你的用户名/hot-rolling-finish-aps/actions
```

---

## 🎯 工作流触发方式

### 自动触发

1. **Release 工作流**：推送版本标签时自动触发
   ```bash
   git tag -a v1.2.0 -m "Release v1.2.0"
   git push origin v1.2.0
   ```

2. **Build & Test 工作流**：
   - 创建 PR 到 main 分支时
   - 推送到 main 分支时

### 手动触发

在 GitHub Actions 页面手动运行 Release 工作流：
1. 访问仓库的 Actions 页面
2. 选择 "Release" 工作流
3. 点击 "Run workflow"
4. 输入版本号并确认

---

## 📦 生成的安装包

成功构建后，将在 GitHub Release 中看到：

- `hot-rolling-aps_v1.1.0_windows_x64-setup.exe` - Windows 安装器
- `hot-rolling-aps_v1.1.0_macos_universal.dmg` - macOS 安装包（通用）
- `hot-rolling-aps_v1.1.0_linux_amd64.AppImage` - Linux AppImage
- `hot-rolling-aps_v1.1.0_linux_amd64.deb` - Linux Debian 包

---

## ⚠️ 重要提示

### 图标文件是必需的

如果没有准备图标文件，构建会失败。请务必先完成第 1 步。

### 首次构建较慢

- 首次构建需要下载所有依赖，可能需要 15-30 分钟
- 后续构建有缓存，会快很多（5-10 分钟）

### GitHub Actions 配额

- **公开仓库**：无限制使用
- **私有仓库**：每月 2000 分钟免费

### 代码签名（可选）

生成的安装包未签名，用户安装时可能会看到安全警告。如需签名：
- macOS: 需要 Apple Developer 账号
- Windows: 需要代码签名证书

详见 [RELEASE_GUIDE.md](docs/guides/RELEASE_GUIDE.md) 的 Q4 部分。

---

## 📚 完整文档

- **[发布指南](docs/guides/RELEASE_GUIDE.md)** - 详细的发布流程和故障排查
- **[图标指南](icons/ICON_GUIDE.md)** - 图标准备的详细说明
- **[README.md](README.md)** - 项目主文档

---

## 🔍 验证清单

在推送到 GitHub 之前，请确认：

- [ ] 已准备所有必需的图标文件
- [ ] 本地测试构建成功：`npm run tauri build -- --debug`
- [ ] 所有测试通过：`npm run test -- --run`
- [ ] 已更新 CHANGELOG.md
- [ ] 版本号已更新（如果需要）

---

## 📞 获取帮助

如果遇到问题：

1. 查看 [RELEASE_GUIDE.md](docs/guides/RELEASE_GUIDE.md) 的故障排查部分
2. 查看 GitHub Actions 的构建日志
3. 在项目中创建 Issue

---

## 🎉 下一步

配置完成后，你可以：

1. 每次发布新版本时，只需推送标签即可自动构建
2. 每个 PR 都会自动运行测试和构建验证
3. 用户可以直接从 GitHub Releases 下载安装包

**祝发布顺利！** 🚀

---

**创建日期**: 2026-02-10
**维护者**: 项目团队

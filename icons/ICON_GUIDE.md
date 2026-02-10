# 应用图标准备指南

本目录用于存放热轧精整排产系统的应用图标。

---

## 📋 必需的图标文件

请准备以下图标文件：

```
icons/
├── 32x32.png          # 32x32 像素 PNG
├── 128x128.png        # 128x128 像素 PNG
├── 128x128@2x.png     # 256x256 像素 PNG (Retina 显示屏)
├── icon.icns          # macOS 图标包
├── icon.ico           # Windows 图标
└── icon.png           # 源图标（推荐 512x512 或 1024x1024）
```

---

## 🎨 图标设计要求

### 尺寸要求
- **最小尺寸**: 32x32 像素
- **推荐源图尺寸**: 512x512 或 1024x1024 像素
- **格式**: PNG（透明背景）

### 设计建议
- 使用简洁、易识别的图标
- 避免过多细节（小尺寸下难以辨认）
- 使用项目主题色
- 确保在浅色和深色背景下都清晰可见

---

## 🛠️ 图标生成方法

### 方法 1：使用 Tauri 图标生成器（推荐）

如果你有一个高质量的源图标（PNG 格式，512x512 或更大），可以使用 Tauri 自动生成所有需要的图标：

```bash
# 1. 安装 Tauri CLI（如果还没安装）
npm install -g @tauri-apps/cli

# 2. 将源图标放在 icons/icon.png

# 3. 运行图标生成命令
tauri icon icons/icon.png

# 这将自动生成所有需要的图标文件
```

### 方法 2：在线工具

如果没有 Tauri CLI，可以使用在线工具：

1. **Icon Kitchen** - https://icon.kitchen/
   - 上传源图标
   - 选择 "Desktop App" 类型
   - 下载生成的图标包

2. **App Icon Generator** - https://appicon.co/
   - 上传源图标
   - 选择平台（macOS、Windows）
   - 下载并解压到 icons/ 目录

### 方法 3：手动创建

#### macOS 图标 (.icns)

```bash
# 1. 创建 iconset 目录
mkdir icon.iconset

# 2. 准备不同尺寸的 PNG 文件
# 需要以下尺寸：16, 32, 64, 128, 256, 512, 1024
# 每个尺寸需要普通和 @2x 版本

# 3. 使用 iconutil 生成 .icns
iconutil -c icns icon.iconset -o icon.icns
```

#### Windows 图标 (.ico)

使用 ImageMagick：

```bash
# 安装 ImageMagick
brew install imagemagick  # macOS
# 或
sudo apt-get install imagemagick  # Linux

# 生成 .ico 文件（包含多个尺寸）
convert icon.png -define icon:auto-resize=256,128,64,48,32,16 icon.ico
```

或使用在线转换工具：
- https://convertio.co/png-ico/
- https://www.icoconverter.com/

---

## ✅ 验证图标

生成图标后，请验证：

1. **检查文件是否存在**
   ```bash
   ls -lh icons/
   ```

2. **验证图标尺寸**
   ```bash
   # macOS/Linux
   file icons/*.png

   # 或使用 identify (ImageMagick)
   identify icons/*.png
   ```

3. **预览图标**
   - macOS: 在 Finder 中查看
   - Windows: 在资源管理器中查看
   - Linux: 使用图片查看器

4. **测试构建**
   ```bash
   npm run tauri build -- --debug
   ```

---

## 🎯 当前状态

- [x] icon.png - 源图标（已存在）
- [ ] 32x32.png - 需要生成
- [ ] 128x128.png - 需要生成
- [ ] 128x128@2x.png - 需要生成
- [ ] icon.icns - 需要生成
- [ ] icon.ico - 需要生成

---

## 📝 注意事项

1. **透明背景**: 所有 PNG 图标应使用透明背景
2. **版权**: 确保图标设计不侵犯版权
3. **一致性**: 保持图标在不同平台上的视觉一致性
4. **测试**: 在实际设备上测试图标显示效果

---

## 🔗 相关资源

- [Tauri 图标指南](https://tauri.app/v1/guides/features/icons)
- [macOS 图标设计指南](https://developer.apple.com/design/human-interface-guidelines/app-icons)
- [Windows 图标设计指南](https://docs.microsoft.com/windows/apps/design/style/iconography)

---

## 📞 获取帮助

如果在准备图标时遇到问题：

1. 查看 [发布指南](../docs/guides/RELEASE_GUIDE.md)
2. 在项目中创建 Issue
3. 联系项目维护者

---

**最后更新**: 2026-02-10

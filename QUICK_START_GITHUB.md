# GitHub 快速上传指南 🚀

## 方式一：自动化脚本（推荐）

```bash
# 运行自动设置脚本
./scripts/setup_github.sh
```

脚本会引导你完成：
1. ✅ Git 用户配置
2. ✅ 远程仓库设置
3. ✅ 安全检查
4. ✅ 首次提交和推送

---

## 方式二：手动设置

### Step 1: 配置 Git 用户信息

```bash
git config --global user.name "你的GitHub用户名"
git config --global user.email "你的GitHub邮箱"
```

### Step 2: 在 GitHub 上创建仓库

1. 访问 https://github.com/new
2. 仓库名: `ishowtts`
3. 可见性: Public 或 Private
4. **不要勾选** "Initialize this repository with a README"
5. 点击 **Create repository**

### Step 3: 生成 Personal Access Token

1. 访问 https://github.com/settings/tokens
2. 点击 **Generate new token (classic)**
3. 设置:
   - Note: `ishowtts-upload`
   - Expiration: 90 days
   - 勾选: ✅ **repo** (完整权限)
4. 点击 **Generate token**
5. **立即复制 Token**（格式: `ghp_xxxx...`）

### Step 4: 连接并推送

```bash
# 添加远程仓库（替换 YOUR_USERNAME）
git remote add origin https://github.com/YOUR_USERNAME/ishowtts.git

# 添加文件
git add .

# 创建提交
git commit -m "Initial commit: iShowTTS TTS pipeline"

# 推送到 GitHub
git push -u origin main
```

**首次推送时会提示：**
```
Username: 你的GitHub用户名
Password: 粘贴你的Token (ghp_xxxx...)
```

---

## 已自动排除的文件 🛡️

以下文件/目录已通过 `.gitignore` 排除：

✅ **敏感信息**
- `config/ishowtts.toml` (包含 oauth_token)
- `config/*token*`, `config/*secret*`

✅ **大文件**
- `target/` - Rust 编译产物
- `data/cache/` - HuggingFace 模型缓存
- `third_party/*/checkpoints/` - 模型权重
- `.venv/` - Python 虚拟环境
- `crates/frontend-web/dist/` - WASM 构建产物

✅ **日志文件**
- `*.log`, `logs/*.log`

✅ **已包含的文件**
- ✅ `config/ishowtts.example.toml` (示例配置)
- ✅ `data/voices/*.wav` (小参考音频)
- ✅ 所有源代码和文档

---

## 后续更新流程 🔄

```bash
# 1. 查看修改
git status

# 2. 添加修改
git add .

# 3. 提交
git commit -m "feat: 描述你的修改"

# 4. 推送
git push
```

**常用提交前缀：**
- `feat:` - 新功能
- `fix:` - 修复 bug
- `docs:` - 文档更新
- `style:` - 代码格式
- `refactor:` - 重构
- `perf:` - 性能优化

---

## 验证检查 ✓

推送前确认：

```bash
# 检查敏感文件是否被忽略
git check-ignore -v config/ishowtts.toml
# 应输出: .gitignore:XX:pattern config/ishowtts.toml

# 查看将被推送的文件
git ls-files

# 确认 Token 未被提交
git grep -i "oauth_token\|ghp_" -- ':!*.example.toml' ':!GITHUB_SETUP.md'
# 应无输出
```

---

## 常见问题 ❓

### Q: 推送时提示 "authentication failed"
```bash
# 清除旧凭据，重新输入 Token
git credential-cache exit
git push
```

### Q: 忘记排除敏感文件，已经提交怎么办？
```bash
# 从 Git 历史中移除（保留本地文件）
git rm --cached config/ishowtts.toml
git commit -m "chore: remove sensitive config from git"
git push
```

### Q: 如何更新 Token？
```bash
# 使用 Git credential helper
git config --global credential.helper store
# 下次 push 时输入新 Token 即可
```

---

## 完整文档 📚

详细说明请查看: **[GITHUB_SETUP.md](./GITHUB_SETUP.md)**

---

**快速命令卡片：**
```bash
# 初始化（首次）
git config --global user.name "username"
git config --global user.email "email"
git remote add origin https://github.com/username/ishowtts.git
git add .
git commit -m "Initial commit"
git push -u origin main

# 日常更新
git add .
git commit -m "feat: description"
git push
```
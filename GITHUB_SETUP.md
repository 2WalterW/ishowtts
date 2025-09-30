# GitHub 仓库设置指南

## 📋 目录
- [设置 Git 用户信息](#设置-git-用户信息)
- [创建 GitHub 仓库](#创建-github-仓库)
- [配置 Personal Access Token](#配置-personal-access-token)
- [首次上传代码](#首次上传代码)
- [后续更新](#后续更新)

---

## 🔧 设置 Git 用户信息

在首次使用 Git 之前，需要配置用户名和邮箱：

```bash
# 设置全局用户名
git config --global user.name "你的GitHub用户名"

# 设置全局邮箱
git config --global user.email "你的GitHub邮箱"

# 验证配置
git config --global --list
```

**示例：**
```bash
git config --global user.name "johnsmith"
git config --global user.email "john.smith@example.com"
```

---

## 🌐 创建 GitHub 仓库

### 方式 1：通过 GitHub 网页
1. 登录 [GitHub](https://github.com)
2. 点击右上角 `+` → `New repository`
3. 填写仓库信息：
   - **Repository name**: `ishowtts` (或你喜欢的名字)
   - **Description**: "Local TTS pipeline for livestream danmaku on NVIDIA Jetson"
   - **可见性**: Public 或 Private
   - ⚠️ **不要勾选** "Initialize this repository with a README"
4. 点击 `Create repository`

### 方式 2：使用 GitHub CLI (gh)
```bash
# 安装 gh (如果未安装)
# Ubuntu/Debian: sudo apt install gh
# Arch: sudo pacman -S github-cli

# 登录
gh auth login

# 创建仓库
gh repo create ishowtts --public --description "Local TTS pipeline for livestream"
```

---

## 🔑 配置 Personal Access Token (PAT)

从 2021 年起，GitHub 要求使用 Token 而非密码进行 HTTPS 推送。

### 生成 Token

1. 访问 GitHub Settings：
   - 点击头像 → **Settings** → **Developer settings** → **Personal access tokens** → **Tokens (classic)**
   - 或直接访问：https://github.com/settings/tokens

2. 点击 **Generate new token (classic)**

3. 配置 Token：
   - **Note**: `ishowtts-upload` (备注名)
   - **Expiration**: 90 days / No expiration (根据需要)
   - **Select scopes**: 勾选
     - ✅ `repo` (完整仓库权限)
     - ✅ `workflow` (如果使用 GitHub Actions)

4. 点击 **Generate token**

5. ⚠️ **立即复制并保存 Token**（只显示一次！）
   ```
   ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
   ```

### 配置 Token 到 Git

**方式 1：使用 Git Credential Helper（推荐）**
```bash
# 启用凭据存储
git config --global credential.helper store

# 或者使用缓存（15分钟过期）
git config --global credential.helper cache

# 首次 push 时会提示输入用户名和密码
# 用户名：你的 GitHub 用户名
# 密码：粘贴刚才生成的 Token
```

**方式 2：直接在 URL 中嵌入（不推荐，仅限测试）**
```bash
git remote set-url origin https://YOUR_USERNAME:YOUR_TOKEN@github.com/YOUR_USERNAME/ishowtts.git
```

---

## 📤 首次上传代码

### 1. 检查忽略文件状态
```bash
# 查看将被提交的文件
git status

# 确认敏感文件已被忽略
git check-ignore config/ishowtts.toml
git check-ignore data/cache/
```

### 2. 添加远程仓库
```bash
# 替换 YOUR_USERNAME 为你的 GitHub 用户名
git remote add origin https://github.com/YOUR_USERNAME/ishowtts.git

# 验证远程仓库
git remote -v
```

### 3. 添加文件并提交
```bash
# 添加所有文件（.gitignore 会自动过滤）
git add .

# 查看暂存的文件
git status

# 创建首次提交
git commit -m "Initial commit: iShowTTS - TTS pipeline for Jetson Orin

- Rust/Axum backend with F5-TTS and IndexTTS support
- Yew/WASM frontend with real-time danmaku streaming
- Twitch integration for live chat TTS
- Voice reference management and override system"

# 推送到 GitHub
git push -u origin main
```

**首次推送时的凭据提示：**
```
Username for 'https://github.com': 你的用户名
Password for 'https://你的用户名@github.com': 粘贴你的Token
```

---

## 🔄 后续更新

### 日常提交流程
```bash
# 1. 查看修改
git status

# 2. 添加修改的文件
git add crates/backend/src/main.rs
git add README.md
# 或添加所有修改
git add .

# 3. 提交
git commit -m "feat: add new voice synthesis feature"

# 4. 推送
git push
```

### 常用提交消息前缀
```
feat:     新功能
fix:      修复 bug
docs:     文档更新
style:    代码格式（不影响功能）
refactor: 重构
perf:     性能优化
test:     测试相关
chore:    构建/工具配置
```

### 拉取远程更新
```bash
# 拉取最新代码
git pull origin main

# 或先查看远程更改
git fetch origin
git diff main origin/main
git merge origin/main
```

---

## 🛡️ 安全检查清单

在推送前，确认以下内容已被排除：

✅ **敏感信息**
- [ ] `config/ishowtts.toml` 中的 `oauth_token`
- [ ] 任何 API keys、密码、私钥

✅ **大文件**
- [ ] `target/` (Rust 构建产物)
- [ ] `data/cache/` (HuggingFace 模型缓存)
- [ ] `third_party/*/checkpoints/` (模型权重文件)
- [ ] `.venv/` (Python 虚拟环境)

✅ **日志文件**
- [ ] `*.log`
- [ ] `logs/*.log`

✅ **临时文件**
- [ ] `crates/frontend-web/dist/` (WASM 构建产物)
- [ ] `*.tmp`, `*.bak`

### 验证命令
```bash
# 查看将被推送的文件
git ls-files

# 检查特定文件是否被忽略
git check-ignore -v config/ishowtts.toml
git check-ignore -v data/cache/huggingface/

# 查看仓库大小
du -sh .git
```

---

## ⚠️ 常见问题

### Q1: 推送时报 "authentication failed"
**A:** Token 可能过期或权限不足
```bash
# 清除缓存的凭据
git credential-cache exit
# 重新推送时会要求输入新的 Token
```

### Q2: 不小心提交了敏感信息
**A:** 使用 `git filter-branch` 或 BFG Repo-Cleaner 清理历史
```bash
# 从历史中删除文件
git filter-branch --force --index-filter \
  'git rm --cached --ignore-unmatch config/ishowtts.toml' \
  --prune-empty --tag-name-filter cat -- --all

# 强制推送（危险！）
git push origin --force --all
```

### Q3: 仓库太大无法推送
**A:** GitHub 限制单文件 100MB，仓库建议 < 1GB
```bash
# 查找大文件
find . -type f -size +50M | grep -v '.git'

# 使用 Git LFS 管理大文件
git lfs install
git lfs track "*.safetensors"
git lfs track "*.bin"
```

### Q4: 如何撤销最后一次提交
**A:**
```bash
# 撤销但保留修改
git reset --soft HEAD~1

# 撤销并丢弃修改（危险！）
git reset --hard HEAD~1
```

---

## 📚 参考资源

- [GitHub Docs - About authentication](https://docs.github.com/en/authentication)
- [Git Documentation](https://git-scm.com/doc)
- [Conventional Commits](https://www.conventionalcommits.org/)

---

**快速参考卡片：**
```bash
# 初始化
git init && git branch -m main

# 配置
git config --global user.name "username"
git config --global user.email "email@example.com"

# 首次推送
git remote add origin https://github.com/username/repo.git
git add .
git commit -m "Initial commit"
git push -u origin main

# 日常使用
git add .
git commit -m "feat: description"
git push
```
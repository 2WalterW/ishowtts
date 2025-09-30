#!/bin/bash
# GitHub 仓库快速设置脚本
# 使用方法: ./scripts/setup_github.sh

set -e

echo "=========================================="
echo "  iShowTTS GitHub 仓库设置向导"
echo "=========================================="
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查是否已经是 git 仓库
if [ -d ".git" ]; then
    echo -e "${GREEN}✓${NC} Git 仓库已初始化"
else
    echo "初始化 Git 仓库..."
    git init
    git branch -m main
    echo -e "${GREEN}✓${NC} Git 仓库初始化完成"
fi

# 检查 Git 用户配置
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "1. 检查 Git 用户配置"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

GIT_USERNAME=$(git config --global user.name || echo "")
GIT_EMAIL=$(git config --global user.email || echo "")

if [ -z "$GIT_USERNAME" ] || [ -z "$GIT_EMAIL" ]; then
    echo -e "${YELLOW}⚠${NC} Git 用户信息未配置"
    echo ""
    read -p "请输入 GitHub 用户名: " username
    read -p "请输入 GitHub 邮箱: " email

    git config --global user.name "$username"
    git config --global user.email "$email"
    echo -e "${GREEN}✓${NC} Git 用户信息配置完成"
else
    echo -e "${GREEN}✓${NC} 用户名: $GIT_USERNAME"
    echo -e "${GREEN}✓${NC} 邮箱: $GIT_EMAIL"
fi

# 检查远程仓库
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "2. 配置远程仓库"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

REMOTE_URL=$(git remote get-url origin 2>/dev/null || echo "")

if [ -z "$REMOTE_URL" ]; then
    echo -e "${YELLOW}⚠${NC} 远程仓库未配置"
    echo ""
    echo "请先在 GitHub 上创建仓库，然后输入仓库信息："
    read -p "GitHub 用户名: " gh_username
    read -p "仓库名称 (默认: ishowtts): " repo_name
    repo_name=${repo_name:-ishowtts}

    git remote add origin "https://github.com/$gh_username/$repo_name.git"
    echo -e "${GREEN}✓${NC} 远程仓库配置完成: https://github.com/$gh_username/$repo_name"
else
    echo -e "${GREEN}✓${NC} 远程仓库: $REMOTE_URL"
fi

# 检查敏感文件
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "3. 安全检查"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "检查敏感文件是否已被忽略..."

SENSITIVE_FILES=(
    "config/ishowtts.toml"
    "logs/backend.log"
    "data/cache/"
    "target/"
)

all_ignored=true
for file in "${SENSITIVE_FILES[@]}"; do
    if git check-ignore -q "$file" 2>/dev/null; then
        echo -e "${GREEN}✓${NC} $file (已忽略)"
    else
        echo -e "${RED}✗${NC} $file (未忽略)"
        all_ignored=false
    fi
done

if [ "$all_ignored" = false ]; then
    echo ""
    echo -e "${RED}警告: 部分敏感文件未被正确忽略！${NC}"
    echo "请检查 .gitignore 文件"
    exit 1
fi

# 检查配置文件中的敏感信息
echo ""
echo "检查配置文件中的敏感信息..."

if grep -q "oauth_token.*=" config/ishowtts.toml 2>/dev/null; then
    if ! git check-ignore -q config/ishowtts.toml; then
        echo -e "${RED}✗ 警告: config/ishowtts.toml 包含 oauth_token 但未被忽略！${NC}"
        exit 1
    else
        echo -e "${GREEN}✓${NC} config/ishowtts.toml 已被忽略"
    fi
fi

if [ ! -f "config/ishowtts.example.toml" ]; then
    echo -e "${YELLOW}⚠${NC} config/ishowtts.example.toml 不存在"
    echo "建议创建示例配置文件（不含敏感信息）"
else
    echo -e "${GREEN}✓${NC} config/ishowtts.example.toml 存在"
fi

# 查看将被提交的文件
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "4. 预览将要提交的文件"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

git add .
echo ""
echo "以下文件将被提交："
git status --short | head -30
echo ""
TOTAL_FILES=$(git ls-files --cached --others --exclude-standard | wc -l)
echo "总计: $TOTAL_FILES 个文件"

# 确认提交
echo ""
read -p "是否继续创建提交? (y/N): " confirm

if [[ ! $confirm =~ ^[Yy]$ ]]; then
    echo "已取消"
    exit 0
fi

# 创建提交
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "5. 创建提交"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

git commit -m "Initial commit: iShowTTS - TTS pipeline for Jetson Orin

- Rust/Axum backend with F5-TTS and IndexTTS support
- Yew/WASM frontend with real-time danmaku streaming
- Twitch integration for live chat TTS
- Voice reference management and override system
- Enhanced UI with dark theme and responsive layout"

echo -e "${GREEN}✓${NC} 提交创建完成"

# 推送提示
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "6. 推送到 GitHub"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "准备推送到远程仓库..."
echo ""
echo -e "${YELLOW}注意事项:${NC}"
echo "1. 确保已在 GitHub 上创建了仓库"
echo "2. 准备好你的 Personal Access Token (不是密码!)"
echo "3. Token 生成地址: https://github.com/settings/tokens"
echo ""
read -p "是否立即推送? (y/N): " push_confirm

if [[ $push_confirm =~ ^[Yy]$ ]]; then
    echo ""
    echo "推送中..."
    echo "首次推送会要求输入用户名和 Token"
    echo ""

    if git push -u origin main; then
        echo ""
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${GREEN}✓ 成功推送到 GitHub!${NC}"
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo ""
        REMOTE_URL=$(git remote get-url origin)
        echo "仓库地址: ${REMOTE_URL%.git}"
    else
        echo ""
        echo -e "${RED}✗ 推送失败${NC}"
        echo ""
        echo "常见问题:"
        echo "1. Token 权限不足 - 确保勾选了 'repo' 权限"
        echo "2. Token 过期 - 重新生成 Token"
        echo "3. 仓库不存在 - 先在 GitHub 上创建仓库"
        echo ""
        echo "详细指南请查看: GITHUB_SETUP.md"
        exit 1
    fi
else
    echo ""
    echo "稍后手动推送，请运行:"
    echo "  git push -u origin main"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "设置完成! 📦"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "后续更新代码，使用:"
echo "  git add ."
echo "  git commit -m \"描述你的修改\""
echo "  git push"
echo ""
echo "详细文档: GITHUB_SETUP.md"
echo ""
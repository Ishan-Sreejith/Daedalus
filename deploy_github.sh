#!/bin/bash

# CoRe Language WebAssembly Deployment Script for GitHub Pages

echo "🚀 Deploying CoRe Language to GitHub Pages..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    echo -e "${RED}❌ Error: Not in a git repository. Please initialize git first:${NC}"
    echo "   git init"
    echo "   git remote add origin https://github.com/yourusername/your-repo.git"
    exit 1
fi

# Check if pkg directory exists
if [ ! -d "pkg" ]; then
    echo -e "${YELLOW}📦 WebAssembly package not found. Building...${NC}"
    ./setup_wasm.sh
    if [ $? -ne 0 ]; then
        echo -e "${RED}❌ Failed to build WebAssembly package${NC}"
        exit 1
    fi
fi

# Create a deployment directory
echo -e "${BLUE}📁 Creating deployment structure...${NC}"
mkdir -p deploy

# Copy necessary files
cp index.html deploy/
cp -r pkg deploy/
cp WEBASSEMBLY_README.md deploy/README.md

# Create a simple .gitignore for the deployment
cat > deploy/.gitignore << EOF
# Development files
src/
target/
Cargo.toml
Cargo.lock
*.rs
.vscode/
.idea/

# Keep only the web deployment files
!index.html
!pkg/
!README.md
EOF

echo -e "${GREEN}✅ Deployment files ready in 'deploy' directory:${NC}"
ls -la deploy/

echo ""
echo -e "${BLUE}🌐 To deploy to GitHub Pages:${NC}"
echo ""
echo "1. Create a new repository on GitHub for your CoRe Language playground"
echo ""
echo "2. Push the deployment files:"
echo "   cd deploy"
echo "   git init"
echo "   git add ."
echo "   git commit -m \"Deploy CoRe Language WebAssembly playground\""
echo "   git branch -M main"
echo "   git remote add origin https://github.com/YOURUSERNAME/YOURREPO.git"
echo "   git push -u origin main"
echo ""
echo "3. Enable GitHub Pages in your repository settings:"
echo "   - Go to Settings → Pages"
echo "   - Select 'Deploy from a branch'"
echo "   - Choose 'main' branch and '/ (root)' folder"
echo "   - Save"
echo ""
echo -e "${GREEN}🎉 Your CoRe Language playground will be available at:${NC}"
echo "   https://YOURUSERNAME.github.io/YOURREPO/"
echo ""
echo -e "${YELLOW}💡 Tip: You can also test locally by running:${NC}"
echo "   cd deploy && python3 -m http.server 8000"
echo "   Then open http://localhost:8000"

# Requirements

## System Requirements
- **Mac**: Apple silicon
- **macOS**: 26+
- **Apple Container**: 1.0.0+ (auto-detected and validated at startup)
- **Rust**: 1.70+ (for building from source)

## Apple Container Installation
Apple Container is **required** for kina to work. Install it first:

**Option A — Manual installer (recommended):**
1. **Download**: Get the latest signed installer from [Apple Container Releases](https://github.com/apple/container/releases)
2. **Install**: Double-click the `.pkg` file and follow the installer prompts
3. **Start Service**: Run `container system start` to start the API server

**Option B — Homebrew cask:**
```bash
brew install --cask container
container system start
```

**Verify**: Check installation with `container --version`

**Note**: kina requires Apple Container **1.0.0 or later**. The version is automatically detected and validated when kina starts. Run `kina` (no arguments) to see your kina and Apple Container versions.

## Kubernetes Tools
- `kubectl` - Kubernetes command-line tool
- `kubectx` & `kubens` - Context and namespace management (optional)

## Development Tools (Optional)
- [mise](https://mise.jdx.dev/) - Development environment manager with task automation

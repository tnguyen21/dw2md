# Publishing Guide

## Prerequisites

1. **GitHub repository** - Update repository URLs in `Cargo.toml`:
   ```toml
   homepage = "https://github.com/YOUR_USERNAME/dw2md"
   repository = "https://github.com/YOUR_USERNAME/dw2md"
   ```

2. **GitHub secrets** - Add to repository settings:
   - `CARGO_REGISTRY_TOKEN` - Get from https://crates.io/me

3. **Update author** in `Cargo.toml`:
   ```toml
   authors = ["Your Name <your.email@example.com>"]
   ```

## Publishing to crates.io

### Manual publish

```bash
# Verify everything works locally
cargo test
cargo clippy --all-targets --all-features
cargo build --release

# Dry run
cargo publish --dry-run

# Publish
cargo publish
```

### Automated release (recommended)

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` with new version
3. Commit changes
4. Create and push a git tag:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

The GitHub Actions workflow will automatically:
- Build binaries for all platforms
- Create a GitHub release with assets
- Publish to crates.io

## Version Bumping

Follow [semver](https://semver.org/):
- **0.1.0 → 0.1.1** - Bug fixes
- **0.1.0 → 0.2.0** - New features (backwards compatible)
- **0.1.0 → 1.0.0** - Breaking changes

## Distribution Channels

### Homebrew (macOS/Linux)

**Difficulty: Easy** ⭐

Create a Homebrew tap:

1. Create a repo: `homebrew-dw2md`
2. Add a Formula file:
   ```ruby
   class Dw2md < Formula
     desc "Crawl a DeepWiki repository and compile all pages into markdown"
     homepage "https://github.com/tnguyen21/dw2md"
     url "https://github.com/tnguyen21/dw2md/archive/v0.1.0.tar.gz"
     sha256 "..."
     license "MIT"

     depends_on "rust" => :build

     def install
       system "cargo", "install", *std_cargo_args
     end
   end
   ```
3. Users install with: `brew install tnguyen21/dw2md/dw2md`

Alternatively, submit to [homebrew-core](https://github.com/Homebrew/homebrew-core) after gaining traction.

### APT/Debian packages

**Difficulty: Moderate** ⭐⭐

Options:

1. **PPA (Ubuntu)** - Easiest for Debian-based distros:
   - Create Launchpad account
   - Set up PPA: https://launchpad.net/~yourusername/+archive/ubuntu/dw2md
   - Package with `dpkg-buildpackage`
   - Upload with `dput`

2. **cargo-deb** - Generate .deb from Cargo.toml:
   ```bash
   cargo install cargo-deb
   cargo deb
   ```
   Add to `Cargo.toml`:
   ```toml
   [package.metadata.deb]
   maintainer = "Your Name <email@example.com>"
   depends = "$auto"
   section = "utility"
   ```
   Upload .deb files to GitHub releases for manual download.

3. **Debian official** - Hardest but best reach:
   - Package maintainer needed
   - Strict quality requirements
   - See: https://www.debian.org/doc/manuals/maint-guide/

### Other package managers

- **Arch AUR**: Easy - create PKGBUILD, submit to AUR
- **Nix**: Moderate - add to nixpkgs
- **Chocolatey** (Windows): Moderate - create .nuspec
- **Scoop** (Windows): Easy - submit JSON manifest to bucket

**Recommendation**: Start with Homebrew tap (easiest, reaches macOS/Linux users), then add cargo-deb for .deb file generation in CI.

## Checklist Before Release

- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml
- [ ] README reflects any new features
- [ ] Integration tests for new features added

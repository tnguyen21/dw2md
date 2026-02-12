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

## Checklist Before Release

- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml
- [ ] README reflects any new features
- [ ] Integration tests for new features added

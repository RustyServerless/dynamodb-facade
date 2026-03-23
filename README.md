<!-- PROJECT SHIELDS -->
[![crates.io](https://img.shields.io/crates/v/dynamodb-facade.svg)](https://crates.io/crates/dynamodb-facade)
[![docs.rs](https://docs.rs/dynamodb-facade/badge.svg)](https://docs.rs/dynamodb-facade/latest/dynamodb_facade)
[![CI](https://github.com/RustyServerless/dynamodb-facade/workflows/CI/badge.svg)](https://github.com/RustyServerless/dynamodb-facade/actions)
[![License](https://img.shields.io/github/license/RustyServerless/dynamodb-facade.svg)](https://github.com/RustyServerless/dynamodb-facade/blob/main/LICENSE)

# dynamodb-facade
A typed facade over aws-sdk-dynamodb with expression builders and batch/transaction support

🚧🚧🚧🚧🚧🚧🚧🚧🚧🚧🚧

🔨🔨🔨🔨🔨🔨🔨🔨🔨🔨🔨

# ⚠️ WORK IN PROGRESS ⚠️

🔨🔨🔨🔨🔨🔨🔨🔨🔨🔨🔨

🚧🚧🚧🚧🚧🚧🚧🚧🚧🚧🚧

## Minimum Supported Rust Version (MSRV)

This crate requires Rust version 1.85.0 or later.

## Contributing

We welcome contributions! Here's how you can help:

1. Report bugs by opening an issue
2. Suggest new features or improvements
3. Submit pull requests for bug fixes or features
4. Improve documentation
5. Share example code and use cases

Please review our contributing guidelines before submitting pull requests.

### Git Hooks

This project uses git hooks to ensure code quality. The hooks are automatically installed when you enter a development shell using `nix flakes` and `direnv`.

The following checks are run before each commit:
- Code formatting (cargo fmt)
- Linting (cargo clippy)
- Doc generation (cargo doc)
- Tests (cargo test)

If any of these checks fail, the commit will be aborted. Fix the issues and try committing again.

To manually install the hooks:
```bash
./scripts/install-hooks.sh
```

Note: Any changes that have not passed local tests will result in CI failures, as GitHub Actions performs identical verification checks.

## Issues

Before reporting issues, please check:

1. Existing issues to avoid duplicates
2. The documentation to ensure it's not a usage error
3. The FAQ for common problems

When opening a new issue, include:

- A clear title and description
- Steps to reproduce bugs
- Expected vs actual behavior
- Code samples if relevant

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Authors

- Jérémie RODON ([@JeremieRodon](https://github.com/JeremieRodon))

If you find this crate useful, please star the repository and share your feedback!

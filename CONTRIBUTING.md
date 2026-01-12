## Contributing

- **Do not include official game data or files in contributions**
- All pull requests should be made against the `dev` branch and any AI usage should be clearly noted.
- `main` branch should always be buildable and reasonably stable.
- Format code using `cargo fmt` or `rustfmt --edition 2024` and use `cargo clippy -- -Dwarnings` for linting.
- Document notable changes (example: bug fixes or feature changes) in the `[Unreleased]` section of the [changelog](CHANGELOG.md).
- Prefer multiple small commits instead of individual large commits when possible. Squash when appropriate such as multiple commits fixing typos.

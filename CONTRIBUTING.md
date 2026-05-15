# Contributing

Thanks for your interest. `evidence` is pre-alpha — the design is still moving, so opening an issue before writing code is the fastest way to get something merged.

## Before you start

- For anything beyond a typo, open an issue describing the problem. For non-trivial changes, agree on the approach before writing code.
- Run these locally — CI enforces all three:
  ```sh
  cargo fmt --all
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  ```

## Pull requests

- One concern per PR. Smaller is easier to review.
- Conventional commit titles: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`, `perf:`.
- Add tests for behavior changes.
- Update `CHANGELOG.md` under `[Unreleased]`.

## Security

See [SECURITY.md](SECURITY.md). Do not file public issues for vulnerabilities.

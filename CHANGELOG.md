# Changelog

## [0.11.0] - 2025-11-09

- Fixed crash after generating tokens
- Update dependencies

## [0.10.0] - 2024-12-30

- Upgrade wry to 0.47
- Build binary for `aarch64-apple-darwin`

## [0.9.0] - 2023-11-25

- Require `libxdo` on Linux
- Upgrade `wry` to 0.35

## [0.8.0] - 2023-05-23

- Do not clear browsing data at startup by default as it can cause issues on macOS. See `tesla_auth --help`

## [0.7.0] - 2023-05-05

- Clear browsing data at startup (can be disabled with the `-k` flag. See `tesla_auth --help`)

## [0.6.3] - 2023-03-15

- Allow opening the dev tools in release mode

## [0.6.2] - 2023-02-20

- Sync `Cargo.lock` file

## [0.6.1] - 2023-02-19

- Bump `wry` to 0.27.0

## [0.6.0] - 2023-02-03

- Upgrade `wry` to 0.26.0
  - On Linux this requires webkit2gtk-4.1 as dependency from now on (see [README](https://github.com/adriankumpf/tesla_auth#platform-specific-dependencies))
- Remove unnecessary indicator package
- Update GitHub action workflows

## [0.5.4] - 2022-12-16

- Update dependencies

## [0.5.3] - 2022-09-11

- Upgrade `wry` to 0.21.1
- Update instructions for Debian 11 (#56)

## [0.5.2] - 2022-08-02

- Upgrade `wry` to 0.20

## [0.5.1] - 2022-06-14

- Update dependencies

## [0.5.0] - 2022-04-09

- Remove ability to generate Owner API tokens as Tesla does not support them anymore
- Upgrade `wry` to 0.15

## [0.4.2] - 2022-02-10

- Upgrade `wry` to 0.13

## [0.4.1] - 2022-01-01

- Close window if login was canceled

## [0.4.0] - 2021-11-01

- Add support for Chinese auth servers
- Reduce width of token fields

## [0.3.0] - 2021-11-01

- Fetch the short-lived SSO tokens by default
- Add flag (`-o`, `--owner-api-token`) for enabling SSO token exchange to long-lived Owner API tokens
- Log URL changes

## [0.2.0] - 2021-09-17

- Render tokens an callback page
- Print token expiration date
- Parse command line arguments
- Print tokens to stdout

## [0.1.1] - 2021-09-17

- Statically link VCRuntime when using the MSVC toolchain
- Update release profile

## [0.1.0] - 2021-09-17

[0.10.0]: https://github.com/adriankumpf/tesla_auth/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/adriankumpf/tesla_auth/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/adriankumpf/tesla_auth/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/adriankumpf/tesla_auth/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/adriankumpf/tesla_auth/compare/v0.6.3...v0.7.0
[0.6.3]: https://github.com/adriankumpf/tesla_auth/compare/v0.6.2...v0.6.3
[0.6.2]: https://github.com/adriankumpf/tesla_auth/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/adriankumpf/tesla_auth/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/adriankumpf/tesla_auth/compare/v0.5.4...v0.5.0
[0.5.4]: https://github.com/adriankumpf/tesla_auth/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/adriankumpf/tesla_auth/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/adriankumpf/tesla_auth/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/adriankumpf/tesla_auth/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/adriankumpf/tesla_auth/compare/v0.4.2...v0.5.0
[0.4.2]: https://github.com/adriankumpf/tesla_auth/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/adriankumpf/tesla_auth/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/adriankumpf/tesla_auth/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/adriankumpf/tesla_auth/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/adriankumpf/tesla_auth/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/adriankumpf/tesla_auth/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/adriankumpf/tesla_auth/compare/bd52d8b...v0.1.0

# Repository Guidelines

## Project Description

This is a Rust-powered GPUI application for users to import images to their screenshot library on Steam via the Steamworks API.

## Implementation Preferences

- Tests should protect meaningful user-facing behavior, security properties, or difficult edge cases. Avoid redundant API smoke tests, tests of code from other crates, and tests that only preserve removed behavior.
- Be idiomatic. If your Rust code looks like a JavaScript dev wrote it then it's bad Rust code.
- Use gpui & gpui-component principles and conventions. Use components, icons and assets from gpui-component where available.

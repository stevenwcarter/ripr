# 0007: Release Target Matrix

## Status
Accepted

## Context
Binary releases should cover the major desktop operating systems and common Linux deployment targets (including Alpine Linux and Docker scratch images, which require static binaries). The release pipeline should be automated, reproducible, and not require manual steps per platform.

## Decision
Six target triples are released:

| Target                        | Notes                                      |
|-------------------------------|--------------------------------------------|
| `x86_64-unknown-linux-gnu`    | Native runner                              |
| `x86_64-unknown-linux-musl`   | Built via `cross 0.2.5 --locked`           |
| `aarch64-unknown-linux-gnu`   | Built via `cross 0.2.5 --locked`           |
| `x86_64-apple-darwin`         | Native runner                              |
| `aarch64-apple-darwin`        | Native Apple Silicon runner                |
| `x86_64-pc-windows-msvc`      | Native runner                              |

Stripping: `strip` on Linux, `strip -x` on macOS (to preserve Mach-O structure). Cross-compiled targets are not stripped (no host cross-strip toolchain available in CI). Archives: `.tar.gz` on Unix targets, `.zip` on Windows. A SHA256 checksum file is produced per archive.

`cross` is pinned to version `0.2.5 --locked` to prevent version drift in CI. The release pipeline is triggered by tags matching `v*.*.*`. Tags containing `-` (e.g., `v0.2.0-beta.1`) are marked as pre-release on GitHub. macOS aarch64 uses a native Apple Silicon GitHub runner rather than cross-compilation.

## Consequences
musl builds are fully statically linked — suitable for Alpine Linux, Docker scratch images, and any environment without a glibc runtime. Release artifacts are produced automatically on version tags with no manual steps. `cross` pinning ensures identical build environments across CI runs. SHA256 checksums allow consumers to verify binary integrity.

## Alternatives considered
- **cargo-dist**: More opinionated and magical; less control over stripping, archive format, and pre-release tagging behavior. Harder to customize for the musl and aarch64-linux targets.
- **GNU/Windows only (no musl, no Apple Silicon)**: Misses the Alpine/Docker use case and the growing Apple Silicon user base.
- **Universal macOS binary (`lipo`)**: Doubles the macOS build time for a single combined artifact. Separate per-arch downloads are simpler and more widely supported by package managers and install scripts.

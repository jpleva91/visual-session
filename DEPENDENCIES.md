# Dependency provenance

The renderer has one direct dependency, pinned to `serde_json = "=1.0.151"` from the standard crates.io registry. [Serde JSON upstream](https://github.com/serde-rs/json) supplies the JSON parser; all validation, HTML construction, escaping, file updates, and CLI behavior are implemented locally in safe Rust. No frontend package, remote script, font, analytics library, or runtime network client is used.

The active dependency tree from `cargo tree --offline --locked` is:

```text
visual-session 0.1.0
└── serde_json 1.0.151
    ├── itoa 1.0.18
    ├── memchr 2.8.3
    ├── serde_core 1.0.229
    └── zmij 1.0.23
```

| Crate | Upstream | Declared license |
| --- | --- | --- |
| serde_json, serde_core | [serde-rs](https://github.com/serde-rs) | MIT OR Apache-2.0 |
| itoa | [dtolnay/itoa](https://github.com/dtolnay/itoa) | MIT OR Apache-2.0 |
| memchr | [BurntSushi/memchr](https://github.com/BurntSushi/memchr) | Unlicense OR MIT |
| zmij | [dtolnay/zmij](https://github.com/dtolnay/zmij) | MIT |

The lockfile also contains optional/target resolution entries not compiled in this default graph. Cargo metadata reports their upstream repositories and licenses. The renderer is `publish = false`; there is no package-registry publishing workflow.

Development verification used already cached registry crates and the installed Rust/Cargo 1.96.1 toolchain. Cargo's locked offline build and tests passed with the recorded registry checksums. The direct upstream identity and permissive licensing were checked; no new toolchain or infrastructure was installed. This is a provenance and build check, not a comprehensive vulnerability advisory audit or a reproducible compiler bootstrap claim.

To inspect the graph and provenance yourself:

```sh
cargo tree --offline --locked --manifest-path skills/visual-session/scripts/renderer/Cargo.toml
cargo metadata --offline --locked --manifest-path skills/visual-session/scripts/renderer/Cargo.toml --format-version 1
```

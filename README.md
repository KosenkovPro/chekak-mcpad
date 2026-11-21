# chekak-mcpad

AI-first Rust text editor and local-first knowledge base — a modern Notepad++ & Obsidian alternative with native MCP server, Whisper voice dictation, and .canvas boards.

## Status

Wave 0 (foundation) and wave 1 (the editor) are done: the native shell opens a vault, the tree, the
tabs, the split and the themes work, and the session with the drafts survives a restart. The search
panel of MCPad replaces the panel of CodeMirror, so it speaks the language of the application and
follows the active theme. Wave 2 (the MCP server) is done as well: the server speaks the Model
Context Protocol over stdio and over a local HTTP port, works read-only by default and asks the user
before every change.

| Part | State |
| --- | --- |
| `crates/mcpad-core` | vault layout, paths, config, `.mcpadignore`, atomic writes, theme model — 38 tests |
| `crates/mcpad-text` | encodings, line endings, rope buffer, lazy access to huge files — 28 tests |
| `crates/mcpad-app` | service layer shared by the shell, the CLI and the MCP server, full text search — 28 tests |
| `crates/mcpad-mcp` | the MCP server: protocol, tools, stdio and HTTP transports, the confirmation gate — 70 tests |
| `theme/` | JSON schema, generator, three built-in themes, validator (schema + parity + contrast) |
| `apps/desktop` | Tauri v2 shell, Vite + React + CodeMirror 6, theme engine, icon registry, search panel, MCP panel — 34 tests |
| `.github/workflows` | fmt, clippy, nextest on Windows, macOS and Linux, theme checks, licence check |

### The editor

* Search and replace with regular expressions, whole words and case sensitivity, with the number
  of the matches. `Ctrl+F`, `Ctrl+H`, `F3`, `Shift+F3`.
* Go to a line from the status bar or with `Ctrl+G`.
* The status bar shows the position of the caret, the size of the selection and the size of the
  document.
* A file above 8 MB opens partially: the editor is read only until the user loads the file
  completely, so a part of a note can never be written back over the whole file.

### The MCP server

The server speaks the Model Context Protocol and offers seven tools: `list_files`, `read_file`,
`read_file_full`, `search`, `vault_info`, `write_file` and `delete_file`. Every file of the vault
is also a resource under the address `mcpad://vault/<path>`.

* It works **read-only by default**. A change is only made after the user confirmed it in the
  panel of the application, where the tool, the path and a preview of the new content are shown.
* The server listens on `127.0.0.1` only and asks for a token, so no other program of the machine
  and nothing from the network reaches the vault.
* The revisions of the protocol `2024-11-05`, `2025-03-26`, `2025-06-18` and `2025-11-25` are
  understood; the revision of the client is answered with the same one.

Two ways to run it:

```bash
# inside the application: the panel "MCP" shows the address and the token
apps/desktop/src-tauri/target/debug/mcpad-desktop.exe /path/to/vault

# as a program, for a client that starts the server itself
cargo run -p mcpad-mcp -- stdio --vault /path/to/vault
cargo run -p mcpad-mcp -- http  --vault /path/to/vault --port 17171 --token secret
```

The vault is taken from `--vault`, from the variable `MCPAD_VAULT` or from the vault of the previous
run of the application. `mcpad-mcp --help` lists every option; the changes are only made without a
confirmation when the program is started with `--allow-writes`.

## Running it

```bash
# the vault is taken from the argument, otherwise the vault of the previous run is reopened
apps/desktop/src-tauri/target/debug/mcpad-desktop.exe /path/to/vault
```

## Repository layout

```
crates/mcpad-core/    Vault layout, platform paths, configuration, ignore rules, atomic writes
crates/mcpad-text/    Encodings, line endings, rope buffers, lazy access to huge files
crates/mcpad-app/     Service layer: files, drafts, session, themes, full text search
crates/mcpad-mcp/     The MCP server: protocol, tools, transports and the confirmation gate
theme/schema/         JSON schema of a theme document
theme/source/         Generator that keeps the built-in themes in sync
theme/themes/         Generated theme files (dark, light, high-contrast)
apps/desktop/         Tauri v2 shell and the React frontend
scripts/              Repository level checks and the installer of the icons
```

## Icons and licences

The artwork of the icons is not part of MCPad. The licence of the project covers
the source code, not the drawings of somebody else, so the repository keeps only
the links to the sources and never the files themselves:

* `scripts/icons.manifest.json` names the source repository, the pinned revision
  and the SHA-256 of every file.
* `npm run icons:install` downloads the files, checks the checksums and writes
  them next to the code that uses them. `npm run icons:check` only verifies what
  is installed.
* The downloaded files are ignored by Git. `npm run icons:app` turns the source
  icon of the application into the platform specific set.
* `THIRD_PARTY_NOTICES.md` carries the licence and the copyright of the authors,
  so the notice travels with every build.

The interface falls back to the own hand written set of icons when the files are
not installed, so a fresh clone still runs.

## Principles

* A vault is a plain folder with plain `.md` and `.canvas` files. No proprietary database.
* The `.mcpad/` service directory is local only: it never synchronises and never shows up in the tree.
  The `.mcpadignore` rule file is portable, exactly like `.gitignore` in a Git repository.
* Writes are atomic, so a crash can never leave a half written note behind.
* Colours, icons and typography live in design tokens, not in the components. The frontend turns
  every token into a `--mcpad-*` CSS custom property, the core reads the same tokens through serde.
* A broken theme never breaks the editor: every invalid token falls back to the default value.
* The artwork of third party icons never enters the repository: only links, revisions and checksums.
* The MCP server is read-only by default; every write has to be confirmed in the UI.

## Requirements

* Rust stable with `rustfmt`, `clippy` and `cargo-nextest`
* Node.js 22 or newer

The toolchain of this machine lives in `D:\app\Rust`, with `RUSTUP_HOME=D:\app\Rust\rustup`
and `CARGO_HOME=D:\app\Rust\cargo`.

## Development

```bash
# Rust workspace
cargo build --workspace
cargo test --workspace
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all

# Frontend and themes
npm install
npm run themes:build     # regenerate theme/themes from theme/source/themes.mjs
npm run themes:validate  # schema, token parity, contrast and generator sync
npm run icons:install    # download the icons of the source repository
npm run icons:check      # verify the installed icons against the manifest
npm test                 # theme engine, icon registry and search engine
npm run dev --workspace @mcpad/desktop
```

## License

Apache-2.0 for the code of MCPad. The core only depends on MIT, Apache-2.0, BSD and ISC licensed
crates. The icons come from their own authors under their own licences, see `THIRD_PARTY_NOTICES.md`.

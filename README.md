# Shard

**Shard** is an MCP server that lets your agents read and write notes in your Obsidian vault.
It doesn't use the Obsidian CLI. Instead, it works directly with the vault directory you point it at.

## Features

- **Direct vault access.** Works on the vault directory itself. No Obsidian CLI, no running Obsidian instance, no plugins.
- **Read and write notes.** Create, overwrite, prepend/append, and delete notes. Missing parent directories are created automatically.
- **Targeted edits.** `patch_note` replaces an exact string in the note body, so agents don't have to rewrite whole files.
- **Frontmatter-aware.** Read and update YAML properties separately from the body, without touching note content. Key order is preserved on update.
- **Cheap reads.** Get only the frontmatter when the body isn't needed, and list directories non-recursively.
- **Safe by default.** All paths are relative to the vault root; absolute paths, `..`, `.` and `:` are rejected. Symlinks are resolved and must stay inside the vault. Hidden entries (`.obsidian`, `.git`, `.trash`, dotfiles) and `node_modules` are not accessible.
- **Atomic writes.** Notes are written to a temp file and renamed into place, so Obsidian and sync tools never see a half-written note.

## Installation

Download the binary for your platform from [GitHub Releases](https://github.com/kostya-zero/shard/releases) and put it somewhere permanent (for example, a directory in your `PATH`).

Shard talks to the client over stdio and takes a single argument: the path to your vault.

```
shard <path-to-vault>
```

The path must be an existing directory. Logs go to stderr (stdout carries the MCP protocol); the default level is `info`, override it with the `RUST_LOG` environment variable (for example `RUST_LOG=debug`).

<details>
<summary><b>Claude Code</b></summary>

Add Shard with the CLI:

```
claude mcp add shard -- /path/to/shard /path/to/vault
```

Use `--scope user` to make it available in all projects, or `--scope project` to write it to `.mcp.json` in the current project.

Or add it to `.mcp.json` manually:

```json
{
  "mcpServers": {
    "shard": {
      "command": "/path/to/shard",
      "args": ["/path/to/vault"]
    }
  }
}
```

</details>

<details>
<summary><b>Claude Desktop</b></summary>

Open **Settings → Developer → Edit Config** and add Shard to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "shard": {
      "command": "C:\\path\\to\\shard.exe",
      "args": ["C:\\path\\to\\vault"]
    }
  }
}
```

On macOS and Linux use plain paths (`/path/to/shard`, `/path/to/vault`). Restart Claude Desktop after saving.

</details>

<details>
<summary><b>Codex</b></summary>

Add Shard with the CLI:

```
codex mcp add shard -- /path/to/shard /path/to/vault
```

Or add it to `~/.codex/config.toml` manually:

```toml
[mcp_servers.shard]
command = "/path/to/shard"
args = ["/path/to/vault"]
```

On Windows, use single-quoted TOML strings for paths (`'C:\path\to\shard.exe'`) so backslashes aren't treated as escapes.

</details>

<details>
<summary><b>OpenCode</b></summary>

Add Shard to `opencode.json` in your project, or to `~/.config/opencode/opencode.json` to make it available everywhere:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "shard": {
      "type": "local",
      "command": ["/path/to/shard", "/path/to/vault"],
      "enabled": true
    }
  }
}
```

Note that OpenCode takes the executable and its arguments as a single `command` array.

</details>

## Implemented tools

### File Operations

- [x] `write_note`: Create a note, fully replace it (`mode=overwrite`), or add text to its start/end (`mode=prepend`/`append`). Missing parent directories are created. Prepend/append require an existing note; if `frontmatter` is given it is merged into the existing properties (nested objects merge recursively, null removes a key), otherwise the original frontmatter is left untouched. To change a small part of a note, prefer `patch_note`; to change only properties, use `update_frontmatter`.
- [x] `read_note`: Read a note and return its raw text: the YAML frontmatter block (if any) followed by the markdown body. To get only the properties, use `get_frontmatter` instead.
- [x] `patch_note`: Replace an exact string in the body of an existing note without rewriting the whole file. Frontmatter is preserved and not searched. Fails if the string is not found, or is found several times while `replace_all` is false.
- [x] `delete_note`: Permanently delete a note from the vault. This cannot be undone.
- [x] `list_directory`: List the files and subdirectories directly inside a vault directory (non-recursive). Returns `dirs` and `files` as sorted name lists. Symlinks are followed; broken entries are skipped. Use an empty path for the vault root.
- [x] `move_file`: Move or rename a single file inside the vault (directories are not supported). Missing parent directories of the destination are created. Fails if the destination exists unless `overwrite=true`. Does not update links in other notes.

### Metadata

- [x] `get_frontmatter`: Get only the frontmatter (properties) of a note as a JSON object, or null if it has none. Cheaper than `read_note` when the body is not needed.
- [x] `update_frontmatter`: Change the frontmatter (properties) of a note without touching its body. By default merges into existing properties (nested objects merge recursively, null value deletes a key); with `merge=false` replaces all properties, and null removes the frontmatter entirely.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

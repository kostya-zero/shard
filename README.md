# Shard

**Shard** is an MCP server that lets your agents read and write notes in your Obsidian vault.
It doesn't use the Obsidian CLI. Instead, it works directly with the vault directory you point it at.

## Features

- **Direct vault access.** Works on the vault directory itself. No Obsidian CLI, no running Obsidian instance, no plugins.
- **Read and write notes.** Create, overwrite, prepend/append, and delete notes. Missing parent directories are created automatically.
- **Targeted edits.** `patch_note` replaces an exact string in the note body, so agents don't have to rewrite whole files.
- **Frontmatter-aware.** Read and update YAML properties separately from the body, without touching note content. Key order is preserved on update.
- **Cheap reads.** Get only the frontmatter when the body isn't needed, and list directories non-recursively.
- **Safe by default.** All paths are relative to the vault root; absolute paths, `..` and `.` are rejected. Hidden entries (`.obsidian`, `.git`, `.trash`, dotfiles) and `node_modules` are not accessible.

## Installation

TBW

## Implemented tools

### File Operations

- [x] `write_note`
- [x] `read_note`
- [x] `patch_note`
- [x] `delete_note`
- [x] `list_directory`
- [x] `move_file`

### Metadata

- [x] `get_frontmatter`
- [x] `update_frontmatter`
- [ ] `get_notes_info`

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

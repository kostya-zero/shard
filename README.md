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

- [x] `write_note`: Create a note, fully replace it (`mode=overwrite`), or add text to its start/end (`mode=prepend`/`append`). Missing parent directories are created. Prepend/append require an existing note and merge `frontmatter` into its existing properties. To change a small part of a note, prefer `patch_note`; to change only properties, use `update_frontmatter`.
- [x] `read_note`: Read a note and return its raw text: the YAML frontmatter block (if any) followed by the markdown body. To get only the properties, use `get_frontmatter` instead.
- [x] `patch_note`: Replace an exact string in the body of an existing note without rewriting the whole file. Frontmatter is preserved and not searched. Fails if the string is not found, or is found several times while `replace_all` is false.
- [x] `delete_note`: Permanently delete a note from the vault. This cannot be undone.
- [x] `list_directory`: List the files and subdirectories directly inside a vault directory (non-recursive). Use an empty path for the vault root.
- [x] `move_file`: Move or rename a file inside the vault. Missing parent directories of the destination are created. Fails if the destination exists unless `overwrite=true`. Does not update links in other notes.

### Metadata

- [x] `get_frontmatter`: Get only the frontmatter (properties) of a note as a JSON object, or null if it has none. Cheaper than `read_note` when the body is not needed.
- [x] `update_frontmatter`: Change the frontmatter (properties) of a note without touching its body. By default merges into existing properties (null value deletes a key); with `merge=false` replaces all properties.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

# File Tools Reference

Reference documentation for the core file manipulation tools: `Read`, `Edit`, and `Write`.

---

## Read

Reads file content from the local filesystem.

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `file_path` | `string` | Yes | Absolute path to the file. Must be absolute, not relative. |
| `offset` | `number` | No | Line number to start reading from (1-indexed). |
| `limit` | `number` | No | Maximum number of lines to read. |
| `pages` | `string` | No | For PDF files only. Page range, e.g., `"1-5"`, `"3"`, `"10-20"`. Max 20 pages per request. |

### Returns

- File content in `cat -n` format (line numbers starting at 1).
- For images: visual description of the image.
- For PDFs: extracted text from the specified pages.
- For Jupyter notebooks (`.ipynb`): all cells with outputs.
- Error if the file does not exist.

### Constraints

- Path must be absolute.
- For large files, use `offset` and `limit` to read in chunks.
- Do not use `cat`, `head`, or `tail` commands when `Read` is available.

### Example

```json
{
  "file_path": "/Users/gehao/Programs/openclaw/src/index.ts",
  "offset": 1,
  "limit": 50
}
```

---

## Edit

Performs exact string replacement in an existing file.

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `file_path` | `string` | Yes | Absolute path to the file to modify. |
| `old_string` | `string` | Yes | The exact text to be replaced. Must match the file content precisely, including indentation. |
| `new_string` | `string` | Yes | The replacement text. |
| `replace_all` | `boolean` | No | If `true`, replaces all occurrences of `old_string`. Default is `false`. |

### Returns

- Success confirmation if the replacement is applied.
- Error if `old_string` is not found, or if it is not unique (unless `replace_all` is `true`).

### Constraints

- **You must call `Read` on the file before calling `Edit`.**
- `old_string` must be unique in the file unless `replace_all` is enabled.
- Preserve exact indentation (spaces/tabs) as shown after the line number prefix in `Read` output.
- Do not include the line number prefix in `old_string` or `new_string`.
- The edit will fail if `old_string` does not match exactly.
- Prefer `Edit` over `sed` or `awk`.

### Example

```json
{
  "file_path": "/Users/gehao/Programs/openclaw/src/index.ts",
  "old_string": "const VERSION = '1.0.0';",
  "new_string": "const VERSION = '1.1.0';"
}
```

---

## Write

Writes content to a file, creating it if it does not exist or overwriting it if it does.

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `file_path` | `string` | Yes | Absolute path to the file. |
| `content` | `string` | Yes | The full content to write to the file. |

### Returns

- Success confirmation after writing.
- Error if the file already exists but was not read first.

### Constraints

- **If the file already exists, you must call `Read` on it before calling `Write`.**
- This tool overwrites the entire file. Use `Edit` for partial modifications.
- Do not use `echo` or redirection (`>`) when `Write` is available.
- Do not create documentation files (`.md`) or `README` files unless explicitly requested.

### Example

```json
{
  "file_path": "/Users/gehao/Programs/openclaw/config.json",
  "content": "{\n  \"port\": 3000,\n  \"debug\": false\n}\n"
}
```

---

## Usage Workflow

1. **Reading**: Always start with `Read` to inspect the current state.
2. **Editing**: Use `Edit` for targeted, surgical changes to existing files.
3. **Writing**: Use `Write` for creating new files or performing complete rewrites.

### Decision Matrix

| Goal | Tool to Use |
|------|-------------|
| View file content | `Read` |
| Change a few lines in an existing file | `Edit` |
| Create a new file | `Write` |
| Completely rewrite an existing file | `Read` then `Write` |
| Batch rename across files | `Edit` with `replace_all: true` |

---

## Common Pitfalls

- **Forgetting to `Read` first**: Both `Edit` and `Write` require prior `Read` calls for existing files.
- **Mismatched indentation**: When copying from `Read` output, ensure spaces/tabs after the line number prefix are preserved exactly.
- **Non-unique `old_string`**: If the string appears multiple times, `Edit` will fail unless `replace_all` is used.
- **Using relative paths**: Always provide absolute paths.

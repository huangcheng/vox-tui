## Repository Map

A full codemap is available at `codemap.md` in the project root.

Before working on any task, read `codemap.md` to understand:
- Project architecture and entry points
- Directory responsibilities and design patterns
- Data flow and integration points between modules

For deep work on a specific folder, also read that folder's `codemap.md`.

## Security Rules

- **NEVER** display, log, or include in output: API keys, tokens, passwords, credentials, or secrets of any kind
- **ALWAYS** mask secrets when referencing config values (show first 4 chars + `***`)
- **NEVER** commit files containing real credentials. Only placeholder values (e.g., `sk-your-api-key-here`)
- User config (`~/.config/vox/`) is outside the repo. Never read or display its contents
- `config.example.toml` must only contain placeholder values

# Zoxi VS Code Extension

This package provides:

- Zoxi syntax highlighting for `.zo`
- language configuration for brackets, comments, and auto-closing pairs
- a built-in language server with diagnostics, hover, completions, document symbols, and go-to-definition for functions

## Development

Install dependencies:

```bash
bun install
```

Build the extension:

```bash
bun run build
```

Run type-checking only:

```bash
bun run check
```

Package a `.vsix` for installation:

```bash
bun run package
```

## Use In VS Code

1. Run `bun run package` in `packages/vscode`.
2. In VS Code, open Extensions.
3. Choose `Install from VSIX...`.
4. Select the generated `.vsix` file.

## Language Server Features

- bracket and string diagnostics
- keyword and snippet completions
- hover docs for Zoxi keywords and built-ins
- document symbols for `fn` declarations
- go-to-definition for functions declared in the workspace's open Zoxi files

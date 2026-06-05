# @kontrocode/cli

The headless KontroCode agent. Run it from the terminal:

```bash
npx @kontrocode/cli info
npx @kontrocode/cli ask "build me a Flutter auth screen"
```

## Commands

| Command | Description |
|---------|-------------|
| `info`  | Print version, Node version, and config path |
| `ask <prompt>` | Send a prompt to the agent |
| `config` | Print the resolved configuration |

## Status

**Phase 1 scaffold.** In Phase 2, `ask` will spawn the Rust agent binary
and stream the response. Until then it prints the input and a banner.

## License

MIT

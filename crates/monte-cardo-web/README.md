## Development

The solver worker must currently be built in release mode. Debug WASM
builds can exceed the worker's available stack during search.

```bash
trunk serve --release
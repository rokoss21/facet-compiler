# fct-wasm

WebAssembly bindings for FACET v2.0 Compiler - enabling deterministic AI agent behavior compilation in the browser and Node.js.

## Installation

### NPM
```bash
npm install @facet/fct-wasm
```

### CDN (Browser)
```html
<script type="module">
  import init, { FacetCompiler } from 'https://cdn.skypack.dev/@facet/fct-wasm';
</script>
```

## Usage

### Browser

```javascript
import init, { FacetCompiler } from '@facet/fct-wasm';

// Initialize the WASM module
await init();

// Create compiler instance
const compiler = new FacetCompiler();

// Parse FACET code
const parseResult = compiler.parse(`
  @system {
    role: "helpful assistant"
  }
  
  @user {
    query: "Hello, world!"
  }
  
  @assistant {
    response: "Hello! How can I help you today?"
  }
`);

if (parseResult.success) {
  console.log('Parsed AST:', parseResult.ast);
  
  // Validate the AST
  const validationResult = compiler.validate(parseResult.ast);
  
  if (validationResult.success) {
    // Render to final output
    const renderResult = compiler.render(parseResult.ast);
    
    if (renderResult.success) {
      console.log('Rendered output:', renderResult.output);
    }
  }
}
```

### Node.js

```javascript
const wasm = require('@facet/fct-wasm');

async function main() {
  // Initialize
  await wasm.init();
  
  // One-shot compilation
  const result = wasm.compile(`
    @vars {
      name: "Alice"
    }
    
    @system {
      role: "helpful assistant"
    }
  `, {
    // Optional context variables
    context: { user_type: "premium" }
  });
  
  if (result.success) {
    console.log('AST:', result.ast);
    console.log('Rendered:', result.rendered);
  } else {
    console.error('Errors:', result.errors);
  }
}

main().catch(console.error);
```

### TypeScript Support

Full TypeScript definitions are included:

```typescript
interface ParseResult {
  success: boolean;
  ast?: any;
  error?: string;
}

interface ValidationResult {
  success: boolean;
  errors: string[];
}

interface RenderResult {
  success: boolean;
  output?: any;
  error?: string;
}

interface CompileResult {
  success: boolean;
  ast?: any;
  rendered?: any;
  errors: string[];
}

declare class FacetCompiler {
  constructor();
  parse(source: string): ParseResult;
  validate(ast: any): ValidationResult;
  render(ast: any, context?: any): RenderResult;
  compile(source: string, context?: any): CompileResult;
}

export function init(): Promise<void>;
export function compile(source: string, context?: any): CompileResult;
export function version(): string;
```

## API Reference

### `init(): Promise<void>`
Initializes the WebAssembly module. Must be called before using any other functions.

### `FacetCompiler`
Main compiler class for working with FACET documents.

#### Methods

- `parse(source: string): ParseResult`
  Parses FACET source code into an AST.
  
- `validate(ast: any): ValidationResult`
  Validates a parsed AST.
  
- `render(ast: any, context?: any): RenderResult`
  Renders an AST to final output.
  
- `compile(source: string, context?: any): CompileResult`
  One-shot: parse → validate → render.

### `compile(source: string, context?: any): CompileResult`
Convenience function for one-shot compilation without creating a compiler instance.

### `version(): string`
Returns the FACET compiler version.

## Building from Source

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
cd crates/fct-wasm
wasm-pack build --target web --out-dir pkg

# Build for Node.js
wasm-pack build --target nodejs --out-dir pkg-node

# Build for bundlers
wasm-pack build --target bundler --out-dir pkg-bundler
```

## Features

- ✅ Parse FACET documents in the browser
- ✅ Validate types and constraints
- ✅ Execute R-DAG and render output
- ✅ TypeScript definitions
- ✅ Tree-shakable ES modules
- ✅ Small footprint (~150KB gzipped)
- ✅ Fast compilation (sub-millisecond for typical documents)

## Browser Support

- Chrome 57+
- Firefox 52+
- Safari 11+
- Edge 16+

## Node.js Support

- Node.js 14+ (ESM)
- Node.js 12+ (CommonJS with bundler)

## Local Development

To run the example locally:

```bash
# Build WASM package
cd crates/fct-wasm
chmod +x build.sh
./build.sh

# Serve the example directory
cd example
python -m http.server 8000
# Or use any static file server
```

Then open http://localhost:8000 in your browser.

### CORS Note

When opening the HTML file directly (`file://`), browsers block WebAssembly loading due to CORS policy. Use a local web server as shown above.

## License

MIT OR Apache-2.0
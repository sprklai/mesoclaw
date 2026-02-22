//! Module templates for quick-start scaffolding.
//!
//! Provides template files (main.py, requirements.txt, Dockerfile, etc.)
//! for creating new modules with working boilerplate code.

use serde::{Deserialize, Serialize};

/// Available module templates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleTemplate {
    /// Empty module with minimal manifest.
    Empty,
    /// Python sidecar tool with stdin/stdout JSON protocol.
    PythonTool,
    /// Python ML tool with pandas/numpy support.
    PythonMl,
    /// Python HTTP service.
    PythonService,
    /// Node.js sidecar tool.
    NodeTool,
}

impl Default for ModuleTemplate {
    fn default() -> Self {
        Self::Empty
    }
}

/// Template file content with optional variable substitution.
#[derive(Debug, Clone)]
pub struct TemplateFile {
    pub filename: String,
    pub content: String,
}

/// Generate template files for a given module configuration.
pub fn generate_template_files(
    template: &ModuleTemplate,
    module_id: &str,
    module_name: &str,
    description: &str,
    container_image: Option<&str>,
) -> Vec<TemplateFile> {
    match template {
        ModuleTemplate::Empty => vec![],
        ModuleTemplate::PythonTool => python_tool_template(module_id, description, container_image),
        ModuleTemplate::PythonMl => python_ml_template(module_id, description, container_image),
        ModuleTemplate::PythonService => {
            python_service_template(module_id, module_name, description, container_image)
        }
        ModuleTemplate::NodeTool => node_tool_template(module_id, description, container_image),
    }
}

/// Get the default container image for a template.
pub fn default_image_for_template(template: &ModuleTemplate) -> Option<&'static str> {
    match template {
        ModuleTemplate::Empty => None,
        ModuleTemplate::PythonTool => Some("python:3.12-slim"),
        ModuleTemplate::PythonMl => Some("python:3.12-slim"),
        ModuleTemplate::PythonService => Some("python:3.12-slim"),
        ModuleTemplate::NodeTool => Some("node:20-slim"),
    }
}

/// Get the default command for a template.
pub fn default_command_for_template(
    template: &ModuleTemplate,
    runtime_is_container: bool,
) -> String {
    match template {
        ModuleTemplate::Empty => String::new(),
        ModuleTemplate::PythonTool | ModuleTemplate::PythonMl => {
            if runtime_is_container {
                "python3".to_string()
            } else {
                "python3".to_string()
            }
        }
        ModuleTemplate::PythonService => {
            if runtime_is_container {
                "python3".to_string()
            } else {
                "python3".to_string()
            }
        }
        ModuleTemplate::NodeTool => {
            if runtime_is_container {
                "node".to_string()
            } else {
                "node".to_string()
            }
        }
    }
}

/// Get the default args for a template.
pub fn default_args_for_template(template: &ModuleTemplate) -> Vec<String> {
    match template {
        ModuleTemplate::Empty => vec![],
        ModuleTemplate::PythonTool | ModuleTemplate::PythonMl => vec!["/app/main.py".to_string()],
        ModuleTemplate::PythonService => vec!["/app/main.py".to_string()],
        ModuleTemplate::NodeTool => vec!["/app/main.js".to_string()],
    }
}

// ─── Python Tool Template ──────────────────────────────────────────────────────

fn python_tool_template(
    module_id: &str,
    description: &str,
    _container_image: Option<&str>,
) -> Vec<TemplateFile> {
    let main_py = format!(
        r##"#!/usr/bin/env python3
"""
{description}

This module communicates via stdin/stdout using JSON-RPC-like messages.
Each request is a JSON object on a single line, and responses are JSON on stdout.
"""

import json
import sys
from typing import Any


def handle_request(request: dict[str, Any]) -> dict[str, Any]:
    """
    Process an incoming request and return the result.

    Override this method to implement your module's logic.
    """
    method = request.get("method", "unknown")
    params = request.get("params", {{}})

    # Example: echo the params back
    if method == "echo":
        return {{"echo": params}}

    # Example: process text
    if method == "process":
        text = params.get("text", "")
        return {{
            "original": text,
            "length": len(text),
            "uppercase": text.upper(),
            "lowercase": text.lower(),
        }}

    # Default: unknown method
    raise ValueError(f"Unknown method: {{method}}")


def main():
    """
    Main loop: read JSON lines from stdin, process, write to stdout.
    """
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
            request_id = request.get("id", "unknown")

            try:
                result = handle_request(request)
                response = {{
                    "id": request_id,
                    "result": result,
                }}
            except Exception as e:
                response = {{
                    "id": request_id,
                    "error": {{
                        "code": -1,
                        "message": str(e),
                    }},
                }}

            print(json.dumps(response), flush=True)

        except json.JSONDecodeError as e:
            error_response = {{
                "id": None,
                "error": {{
                    "code": -32700,
                    "message": f"Parse error: {{e}}",
                }},
            }}
            print(json.dumps(error_response), flush=True)


if __name__ == "__main__":
    main()
"##
    );

    let requirements_txt = r#"# Python dependencies for this module
# Add your dependencies here, e.g.:
# requests>=2.28.0
# numpy>=1.24.0
"#
    .to_string();

    let readme_md = format!(
        r#"# {module_id}

{description}

## Usage

This module communicates via stdin/stdout using JSON-RPC-like messages.

### Request Format

```json
{{"id": "req-1", "method": "process", "params": {{"text": "Hello World"}}}}
```

### Response Format

```json
{{"id": "req-1", "result": {{"original": "Hello World", "length": 11, ...}}}}
```

## Development

1. Edit `main.py` to implement your logic
2. Add dependencies to `requirements.txt`
3. Test with: `echo '{{"id":"1","method":"echo","params":{{}}}}' | python3 main.py`

## Container Usage

When using Docker/Podman runtime, the module directory is mounted at `/app`.
"#
    );

    vec![
        TemplateFile {
            filename: "main.py".to_string(),
            content: main_py,
        },
        TemplateFile {
            filename: "requirements.txt".to_string(),
            content: requirements_txt,
        },
        TemplateFile {
            filename: "README.md".to_string(),
            content: readme_md,
        },
    ]
}

// ─── Python ML Template ────────────────────────────────────────────────────────

fn python_ml_template(
    module_id: &str,
    description: &str,
    _container_image: Option<&str>,
) -> Vec<TemplateFile> {
    let main_py = format!(
        r##"#!/usr/bin/env python3
"""
{description}

Python ML tool with pandas/numpy support for data analysis and machine learning.
"""

import json
import sys
from typing import Any

try:
    import pandas as pd
    import numpy as np
    HAS_ML = True
except ImportError:
    HAS_ML = False


def handle_request(request: dict[str, Any]) -> dict[str, Any]:
    """
    Process an incoming request and return the result.
    """
    if not HAS_ML:
        raise RuntimeError("pandas/numpy not installed. Add them to requirements.txt")

    method = request.get("method", "unknown")
    params = request.get("params", {{}})

    if method == "analyze":
        # Example: analyze a list of numbers
        data = params.get("data", [])
        arr = np.array(data)

        return {{
            "count": len(arr),
            "mean": float(np.mean(arr)) if len(arr) > 0 else None,
            "std": float(np.std(arr)) if len(arr) > 0 else None,
            "min": float(np.min(arr)) if len(arr) > 0 else None,
            "max": float(np.max(arr)) if len(arr) > 0 else None,
        }}

    if method == "describe":
        # Example: descriptive statistics for structured data
        rows = params.get("rows", [])
        if not rows:
            return {{"error": "No data provided"}}

        df = pd.DataFrame(rows)
        desc = df.describe().to_dict()

        # Convert numpy types to native Python types
        result = {{}}
        for col, stats in desc.items():
            result[col] = {{k: float(v) if not pd.isna(v) else None for k, v in stats.items()}}

        return {{"statistics": result, "columns": list(df.columns), "shape": list(df.shape)}}

    raise ValueError(f"Unknown method: {{method}}")


def main():
    """Main loop: read JSON lines from stdin, process, write to stdout."""
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
            request_id = request.get("id", "unknown")

            try:
                result = handle_request(request)
                response = {{"id": request_id, "result": result}}
            except Exception as e:
                response = {{"id": request_id, "error": {{"code": -1, "message": str(e)}}}}

            print(json.dumps(response), flush=True)

        except json.JSONDecodeError as e:
            print(json.dumps({{"id": None, "error": {{"code": -32700, "message": f"Parse error: {{e}}"}}}}), flush=True)


if __name__ == "__main__":
    main()
"##
    );

    let requirements_txt = r#"# Python ML dependencies
pandas>=2.0.0
numpy>=1.24.0
"#
    .to_string();

    let readme_md = format!(
        r#"# {module_id} - ML Tool

{description}

## Features

- Data analysis with pandas
- Numerical computing with numpy
- JSON-RPC over stdin/stdout

## Example Usage

### Analyze a list of numbers

```json
{{"id": "1", "method": "analyze", "params": {{"data": [1, 2, 3, 4, 5]}}}}
```

### Describe structured data

```json
{{"id": "2", "method": "describe", "params": {{"rows": [{{"a": 1, "b": 2}}, {{"a": 3, "b": 4}}]}}}}
```

## Installation

The `requirements.txt` includes pandas and numpy. When running in a container,
these will be installed automatically if you add a pip install step to your Dockerfile.
"#
    );

    vec![
        TemplateFile {
            filename: "main.py".to_string(),
            content: main_py,
        },
        TemplateFile {
            filename: "requirements.txt".to_string(),
            content: requirements_txt,
        },
        TemplateFile {
            filename: "README.md".to_string(),
            content: readme_md,
        },
    ]
}

// ─── Python Service Template ───────────────────────────────────────────────────

fn python_service_template(
    module_id: &str,
    module_name: &str,
    description: &str,
    _container_image: Option<&str>,
) -> Vec<TemplateFile> {
    let main_py = format!(
        r##"#!/usr/bin/env python3
"""
{description}

Long-running HTTP service with health check and execute endpoints.
"""

import json
import os
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Any
import threading
import signal
import sys


# Configuration from environment
PORT = int(os.environ.get("PORT", "8080"))
HOST = os.environ.get("HOST", "0.0.0.0")


class ServiceHandler(BaseHTTPRequestHandler):
    """HTTP request handler for the service."""

    def log_message(self, format: str, *args: Any) -> None:
        """Suppress default logging."""
        pass

    def _send_json(self, status: int, data: dict[str, Any]) -> None:
        """Send a JSON response."""
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())

    def do_GET(self) -> None:
        """Handle GET requests."""
        if self.path == "/health":
            self._send_json(200, {{"status": "healthy", "service": "{module_name}"}})
        else:
            self._send_json(404, {{"error": "Not found"}})

    def do_POST(self) -> None:
        """Handle POST requests."""
        if self.path == "/execute":
            try:
                content_length = int(self.headers.get("Content-Length", 0))
                body = self.rfile.read(content_length).decode()
                params = json.loads(body) if body else {{}}

                result = self._execute(params)
                self._send_json(200, {{"result": result}})

            except json.JSONDecodeError as e:
                self._send_json(400, {{"error": f"Invalid JSON: {{e}}"}})
            except Exception as e:
                self._send_json(500, {{"error": str(e)}})
        else:
            self._send_json(404, {{"error": "Not found"}})

    def _execute(self, params: dict[str, Any]) -> dict[str, Any]:
        """
        Execute the service logic.

        Override this method to implement your service's functionality.
        """
        # Example: echo with processing
        action = params.get("action", "echo")
        data = params.get("data", {{}})

        if action == "echo":
            return {{"echo": data, "processed": True}}

        if action == "transform":
            # Example: transform string values to uppercase
            transformed = {{}}
            for key, value in data.items():
                if isinstance(value, str):
                    transformed[key] = value.upper()
                else:
                    transformed[key] = value
            return {{"transformed": transformed}}

        return {{"action": action, "data": data}}


def run_server():
    """Start the HTTP server."""
    server = HTTPServer((HOST, PORT), ServiceHandler)
    print(f"Service '{module_name}' listening on http://{{HOST}}:{{PORT}}", file=sys.stderr)
    print(f"Health endpoint: http://{{HOST}}:{{PORT}}/health", file=sys.stderr)
    print(f"Execute endpoint: http://{{HOST}}:{{PORT}}/execute", file=sys.stderr)
    server.serve_forever()


if __name__ == "__main__":
    run_server()
"##
    );

    let requirements_txt = r#"# Python dependencies for HTTP service
# Add your dependencies here
"#
    .to_string();

    let readme_md = format!(
        r#"# {module_id} - HTTP Service

{description}

## Endpoints

### Health Check

```
GET /health
```

Returns:
```json
{{"status": "healthy", "service": "..."}}
```

### Execute

```
POST /execute
Content-Type: application/json

{{"action": "echo", "data": {{"key": "value"}}}}
```

Returns:
```json
{{"result": {{"echo": {{"key": "value"}}, "processed": true}}}}
```

## Configuration

- `PORT` - HTTP port (default: 8080)
- `HOST` - Bind address (default: 0.0.0.0)

## Running

```bash
python3 main.py
# Or with custom port:
PORT=3000 python3 main.py
```
"#
    );

    vec![
        TemplateFile {
            filename: "main.py".to_string(),
            content: main_py,
        },
        TemplateFile {
            filename: "requirements.txt".to_string(),
            content: requirements_txt,
        },
        TemplateFile {
            filename: "README.md".to_string(),
            content: readme_md,
        },
    ]
}

// ─── Node.js Tool Template ─────────────────────────────────────────────────────

fn node_tool_template(
    module_id: &str,
    description: &str,
    _container_image: Option<&str>,
) -> Vec<TemplateFile> {
    let main_js = format!(
        r##"#!/usr/bin/env node
/**
 * {description}
 *
 * Node.js sidecar tool communicating via stdin/stdout JSON-RPC.
 */

const readline = require('readline');

/**
 * Process an incoming request and return the result.
 * Override this to implement your module's logic.
 */
function handleRequest(request) {{
    const method = request.method || 'unknown';
    const params = request.params || {{}};

    // Example: echo
    if (method === 'echo') {{
        return {{ echo: params }};
    }}

    // Example: process text
    if (method === 'process') {{
        const text = params.text || '';
        return {{
            original: text,
            length: text.length,
            uppercase: text.toUpperCase(),
            lowercase: text.toLowerCase(),
        }};
    }}

    throw new Error(`Unknown method: ${{method}}`);
}}

/**
 * Main loop: read JSON lines from stdin, process, write to stdout.
 */
async function main() {{
    const rl = readline.createInterface({{
        input: process.stdin,
        output: process.stdout,
        terminal: false,
    }});

    for await (const line of rl) {{
        const trimmed = line.trim();
        if (!trimmed) continue;

        try {{
            const request = JSON.parse(trimmed);
            const requestId = request.id || 'unknown';

            try {{
                const result = handleRequest(request);
                console.log(JSON.stringify({{ id: requestId, result }}));
            }} catch (e) {{
                console.log(JSON.stringify({{
                    id: requestId,
                    error: {{ code: -1, message: e.message }},
                }}));
            }}
        }} catch (e) {{
            console.log(JSON.stringify({{
                id: null,
                error: {{ code: -32700, message: `Parse error: ${{e.message}}` }},
            }}));
        }}
    }}
}}

main().catch(console.error);
"##
    );

    let package_json = format!(
        r#"{{
  "name": "{module_id}",
  "version": "1.0.0",
  "description": "{description}",
  "main": "main.js",
  "type": "commonjs",
  "scripts": {{
    "start": "node main.js"
  }},
  "dependencies": {{}}
}}
"#
    );

    let readme_md = format!(
        r#"# {module_id}

{description}

## Usage

This module communicates via stdin/stdout using JSON-RPC-like messages.

### Request Format

```json
{{"id": "req-1", "method": "process", "params": {{"text": "Hello World"}}}}
```

### Response Format

```json
{{"id": "req-1", "result": {{"original": "Hello World", "length": 11, ...}}}}
```

## Development

1. Edit `main.js` to implement your logic
2. Add dependencies to `package.json`
3. Test with: `echo '{{"id":"1","method":"echo","params":{{}}}}' | node main.js`

## Installation

```bash
npm install
```
"#
    );

    vec![
        TemplateFile {
            filename: "main.js".to_string(),
            content: main_js,
        },
        TemplateFile {
            filename: "package.json".to_string(),
            content: package_json,
        },
        TemplateFile {
            filename: "README.md".to_string(),
            content: readme_md,
        },
    ]
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_tool_template() {
        let files = generate_template_files(
            &ModuleTemplate::PythonTool,
            "test-module",
            "Test Module",
            "A test module",
            None,
        );

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.filename == "main.py"));
        assert!(files.iter().any(|f| f.filename == "requirements.txt"));
        assert!(files.iter().any(|f| f.filename == "README.md"));
    }

    #[test]
    fn test_empty_template() {
        let files = generate_template_files(&ModuleTemplate::Empty, "test", "Test", "Test", None);

        assert!(files.is_empty());
    }

    #[test]
    fn test_default_image_for_template() {
        assert_eq!(default_image_for_template(&ModuleTemplate::Empty), None);
        assert_eq!(
            default_image_for_template(&ModuleTemplate::PythonTool),
            Some("python:3.12-slim")
        );
        assert_eq!(
            default_image_for_template(&ModuleTemplate::NodeTool),
            Some("node:20-slim")
        );
    }

    #[test]
    fn test_default_command_for_template() {
        let cmd = default_command_for_template(&ModuleTemplate::PythonTool, true);
        assert_eq!(cmd, "python3");

        let cmd = default_command_for_template(&ModuleTemplate::NodeTool, true);
        assert_eq!(cmd, "node");
    }

    #[test]
    fn test_python_ml_includes_pandas() {
        let files = generate_template_files(
            &ModuleTemplate::PythonMl,
            "ml-tool",
            "ML Tool",
            "ML tool",
            None,
        );

        let main_py = files.iter().find(|f| f.filename == "main.py").unwrap();
        assert!(main_py.content.contains("pandas"));
        assert!(main_py.content.contains("numpy"));
    }

    #[test]
    fn test_python_service_has_http_endpoints() {
        let files = generate_template_files(
            &ModuleTemplate::PythonService,
            "http-service",
            "HTTP Service",
            "HTTP service",
            None,
        );

        let main_py = files.iter().find(|f| f.filename == "main.py").unwrap();
        assert!(main_py.content.contains("/health"));
        assert!(main_py.content.contains("/execute"));
    }
}

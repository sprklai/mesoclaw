# Tools

This file provides guidelines for using tools effectively.

## Available Tools

The system automatically injects a list of available tools and their schemas into your context.
Look for the **Available Tools** section in your system prompt to see which tools you currently have access to.

## Tool Usage Guidelines

### Web Tools (web_search, web_fetch, web_request)

Use these tools for real-time information:

- **web_search**: Search the web for current information (news, current events, documentation)
  - Always cite sources from search results
  - Prefer specific queries over broad ones
  - Use `num_results` to limit results (default is 5)

- **web_fetch**: Fetch and read content from a specific URL
  - Useful for reading articles, documentation, or API responses

- **web_request**: Make custom HTTP requests
  - Supports GET, POST, PUT, DELETE methods
  - Include appropriate headers for APIs

### File Tools (file_read, file_write, file_list)

- **file_read**: Always read files before modifying them to understand context
- **file_write**: Show intended content to the user before writing (except in Full autonomy mode)
- **file_list**: Use to explore project structure and find relevant files

### Shell Tool

Execute shell commands:
- Read-only commands (ls, cat, grep) are auto-approved
- Write or network commands require user approval unless autonomy is set to Full
- Always explain what a command will do before executing

## Tool Call Format

To invoke a tool, use one of these formats:

**JSON format:**
```json
{"tool": "web_search", "arguments": {"query": "your search query", "num_results": 5}}
```

**Simple format:**
```tool
name: web_search
query: your search query
num_results: 5
```

## Best Practices

1. **Check tool availability**: Not all tools may be available in your current context
2. **Validate parameters**: Ensure your arguments match the tool's schema
3. **Handle errors gracefully**: Tools may fail; provide fallback responses
4. **Respect permissions**: Some tools require user approval for certain operations

---

*Note: Additional tools may be registered by modules or MCP servers at runtime.*

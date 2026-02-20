# Enhanced Skill System - Phase 3 Implementation

## Overview

This document describes the implementation of the enhanced skill system with structured metadata and three-tier loading, based on the OpenClaw research.

## Architecture

### Three-Tier Loading System

Skills are loaded from three sources with the following precedence (highest to lowest):

1. **Workspace Skills** - `<workspace>/.mesoclaw/skills/`
   - Agent-specific skills
   - Highest precedence
   - Override global and bundled skills

2. **Global Skills** - `~/.mesoclaw/skills/`
   - User-installed skills
   - Available to all agents
   - Managed by user

3. **Bundled Skills** - Built into application
   - Always available
   - Updated with app releases
   - Lowest precedence

### Core Types

#### SkillSnapshot
```rust
pub struct SkillSnapshot {
    pub version: String,              // Timestamp for cache invalidation
    pub skills: HashMap<String, Skill>, // All loaded skills
}
```

#### Skill
```rust
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub metadata: SkillMetadata,
    pub enabled: bool,
    pub template: String,
    pub parameters: Vec<TemplateParameter>,
    pub source: SkillSource,
    pub file_path: Option<String>,
}
```

#### SkillMetadata
```rust
pub struct SkillMetadata {
    pub emoji: Option<String>,
    pub requires: Option<SkillRequirements>,
    pub tool_schemas: Option<Vec<ToolSchema>>,
    pub extra: HashMap<String, serde_json::Value>,
}
```

#### SkillRequirements
```rust
pub struct SkillRequirements {
    pub any_bins: Option<Vec<String>>,  // At least one must be present
    pub all_bins: Option<Vec<String>>,  // All must be present
    pub api_keys: Option<Vec<String>>,  // Must be configured
    pub env: Option<Vec<String>>,       // Environment variables
}
```

### File Format

Skills are defined in Markdown files with YAML frontmatter:

```markdown
---
id: my-skill
name: My Skill
description: What this skill does
category: coding
metadata:
  emoji: "🔧"
  requires:
    anyBins: ["python", "python3"]
    allBins: ["git"]
    apiKeys: ["OPENAI_API_KEY"]
  toolSchemas:
    - name: analyze_code
      description: Analyze code quality
      parameters:
        type: object
        properties:
          path:
            type: string
        required: ["path"]
parameters:
  - name: input
    description: Input text
    required: true
---

You are a code analysis expert. Analyze {{input}} for quality.

Provide actionable recommendations with file locations.
```

## Implementation Files

### Backend (Rust)

- `src-tauri/src/agent/skills/mod.rs` - Main skill registry and loading logic
- `src-tauri/src/agent/skills/skill_metadata.rs` - Type definitions
- `src-tauri/src/agent/skills/tests.rs` - Unit tests

### Frontend (TypeScript)

- `src/lib/tauri/skills/types.ts` - Enhanced type definitions matching Rust backend

## Key Features

### 1. Structured Metadata

- **Emoji icons** for UI display
- **Tool requirements** (binaries, API keys, environment variables)
- **Tool schemas** for defining tool interfaces
- **Extensible metadata** via `extra` field

### 2. Requirement Validation

The system can check if skill requirements are met:

```rust
let registry = SkillRegistry::new(template_registry);
let skill = registry.get("my-skill").await.unwrap();

match registry.check_requirements(&skill) {
    Ok(satisfied) => println!("Requirements met: {:?}", satisfied),
    Err(missing) => println!("Missing requirements: {:?}", missing),
}
```

### 3. Three-Tier Loading

```rust
let registry = SkillRegistry::new(template_registry);
let snapshot = registry.build_skill_snapshot(Some(workspace_path)).await?;

// Skills are automatically loaded with correct precedence
for (id, skill) in snapshot.skills {
    println!("Loaded skill: {} from {:?}", id, skill.source);
}
```

### 4. Template Rendering

Skills use Tera templating engine for parameter substitution:

```rust
let mut vars = HashMap::new();
vars.insert("input".to_string(), "src/main.rs".to_string());

let rendered = registry.render("code-analyzer", &vars).await?;
```

## Usage Examples

### Creating a Skill

Create a file in `~/.mesoclaw/skills/my-skill.md`:

```markdown
---
id: test-generator
name: Test Generator
description: Generates unit tests for code
category: testing
metadata:
  emoji: "🧪"
  requires:
    anyBins: ["node", "python"]
parameters:
  - name: code
    description: Code to generate tests for
    required: true
---

Generate comprehensive unit tests for:

```
{{code}}
```

Include edge cases, error handling, and integration tests.
```

### Using from Frontend

```typescript
import { getSkillDetails } from "@/lib/tauri/skills";

const skill = await getSkillDetails("test-generator");

console.log(skill.metadata.emoji); // "🧪"
console.log(skill.metadata.requires?.anyBins); // ["node", "python"]
console.log(skill.parameters); // [{ name: "code", ... }]
```

## Benefits Over Previous System

1. **Rich Metadata** - Skills can specify requirements, icons, and tool schemas
2. **Three-Tier Loading** - Workspace skills override global/bundled skills
3. **Requirement Validation** - Check if binaries/API keys are available before use
4. **Tool Schemas** - Define structured tool interfaces
5. **Extensible** - Extra metadata fields for custom use cases

## Testing

Unit tests verify:
- Frontmatter parsing (full and minimal)
- Metadata extraction
- Requirement validation
- Source precedence
- Template rendering

Run tests:
```bash
cd src-tauri
cargo test --lib agent::skills
```

## Future Enhancements

1. **Skill Installation** - Install skills from git repositories or npm
2. **Skill Watching** - Hot-reload skills when files change
3. **Skill Registry** - Central repository for discovering and sharing skills
4. **Skill Versioning** - Semantic versioning for skills
5. **Skill Dependencies** - Skills that depend on other skills

## Integration Points

The enhanced skill system integrates with:

- **Agent Loop** - Skills can be selected based on task requirements
- **Session Management** - Skill snapshots are cached per session
- **Tool Execution** - Tool schemas define callable interfaces
- **Frontend UI** - Rich metadata drives skill browser UI

## Example Skill Files

See `~/.mesoclaw/skills/code-analyzer.md` for a complete example.

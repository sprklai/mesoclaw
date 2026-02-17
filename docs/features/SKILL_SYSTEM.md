> **STATUS: DEPRECATED — Scheduled for replacement in Phase 0 (Slim Down)**
>
> The skills system documented below (2,631 lines across parser, composer, selector, executor)
> is being replaced with simple prompt templates using `tera` or `handlebars` (~150 lines).
> See `docs/mesoclaw-gap-analysis.md` item S1 for details.
>
> **What replaces it**: Markdown files with `{{variable}}` placeholders, loaded by a single
> `PromptTemplate` struct. No custom parser, no composer, no selector, no executor.
>
> **Why**: The current system is 29% of the entire backend codebase for what amounts to
> string interpolation with YAML frontmatter. This is over-engineering for a desktop app.
>
> **Timeline**: Phase 0, before any new feature work begins.

# AI Skill System

The Skill System provides a modular, composable approach to AI-powered database analysis. Skills are specialized AI capabilities that can be enabled/disabled per workspace, composed together, and extended with custom prompts.

## Table of Contents

- [Overview](#overview)
- [Skill Locations](#skill-locations)
- [Available Skills](#available-skills)
  - [Understanding Category](#understanding-category)
  - [Performance Category](#performance-category)
  - [Security Category](#security-category)
  - [Documentation Category](#documentation-category)
- [Embedded Skills Reference](#embedded-skills-reference)
  - [Schema Explainer](#schema-explainer)
  - [Relationship Mapper](#relationship-mapper)
  - [Query Explainer](#query-explainer)
  - [Query Comparer](#query-comparer)
  - [Query Optimizer](#query-optimizer)
  - [Collection Explainer](#collection-explainer)
  - [Security Audit](#security-audit)
  - [Onboarding Generator](#onboarding-generator)
- [Creating Custom Skills](#creating-custom-skills)
- [Modifying Skills](#modifying-skills)
- [Skill Examples](#skill-examples)
- [Best Practices](#best-practices)
- [Tips and Tricks](#tips-and-tricks)
- [Usage](#usage)
- [API Reference](#api-reference)
- [Testing Skills](#testing-skills)
- [Troubleshooting](#troubleshooting)

---

## Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     SKILL ENGINE                            │
│  Loader → Selector → Composer → Executor                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   APPLICATION ADAPTER                       │
│  Provides: database context, tools, output mapping          │
└─────────────────────────────────────────────────────────────┘
```

### Key Concepts

| Concept                 | Description                                                        |
| ----------------------- | ------------------------------------------------------------------ |
| **Skill**               | A specialized AI capability defined as markdown + YAML frontmatter |
| **Skill Engine**        | Orchestrates skill selection, composition, and execution           |
| **Application Adapter** | Provides database-specific context and tools to skills             |
| **Skill Registry**      | Central index of all available skills                              |
| **Skill Settings**      | Per-workspace configuration for enabling/disabling skills          |

---

## Skill Locations

Skills are loaded from three sources in order of priority:

### 1. Embedded Skills (Built-in)

**Location:** `src-tauri/src/skills_embedded/`

These skills are compiled directly into the application binary and are always available.

```
src-tauri/src/skills_embedded/
├── mod.rs                      # Skill loader module
├── schema_explainer.md         # Schema explanation skill
├── relationship_mapper.md      # Relationship mapping skill
├── query_optimizer.md          # Query optimization skill
├── query_explainer.md          # Query explanation skill
├── query_comparer.md           # Query comparison skill
├── collection_explainer.md     # MongoDB collection skill
├── security_audit.md           # Security audit skill
└── onboarding_generator.md     # Onboarding documentation skill
```

### 2. Local Skills (User-Created)

**Location:** `~/.config/<skillsConfigDirName>/skills/` (Linux/macOS) or `%APPDATA%\<skillsConfigDirName>\skills\` (Windows)

Create custom skills by adding markdown files to this directory:

```
~/.config/<skillsConfigDirName>/skills/
├── my-custom-skill.md          # Your custom skill
├── team-conventions.md         # Team-specific conventions
└── domain-specific/            # Subdirectories are supported
    ├── healthcare-hipaa.md
    └── finance-pci.md
```

### 3. Remote Skills (Future)

**Location:** Remote registry URL (not yet implemented)

Community skills that can be downloaded from a central registry.

### Finding Your Skills Directory

```bash
# Linux/macOS
ls ~/.config/<skillsConfigDirName>/skills/

# Windows (PowerShell)
dir $env:APPDATA\<skillsConfigDirName>\skills\

# Create the directory if it doesn't exist
mkdir -p ~/.config/<skillsConfigDirName>/skills/  # Linux/macOS
```

---

## Available Skills

### Understanding Category

| Skill ID               | Name                 | Description                                            | Default |
| ---------------------- | -------------------- | ------------------------------------------------------ | ------- |
| `schema-explainer`     | Schema Explainer     | Explains database tables, columns, and their purposes  | Enabled |
| `relationship-mapper`  | Relationship Mapper  | Maps and explains database relationships               | Enabled |
| `query-explainer`      | Query Explainer      | Explains SQL queries in plain language                 | Enabled |
| `query-comparer`       | Query Comparer       | Compares two SQL queries structurally and semantically | Enabled |
| `collection-explainer` | Collection Explainer | Explains MongoDB collections and document structure    | Enabled |

### Performance Category

| Skill ID          | Name            | Description                                            | Default |
| ----------------- | --------------- | ------------------------------------------------------ | ------- |
| `query-optimizer` | Query Optimizer | Analyzes queries and suggests performance improvements | Enabled |

### Security Category

| Skill ID         | Name           | Description                                           | Default |
| ---------------- | -------------- | ----------------------------------------------------- | ------- |
| `security-audit` | Security Audit | Identifies potential security issues in schema design | Enabled |

### Documentation Category

| Skill ID               | Name                 | Description                                    | Default |
| ---------------------- | -------------------- | ---------------------------------------------- | ------- |
| `onboarding-generator` | Onboarding Generator | Creates comprehensive onboarding documentation | Enabled |

---

## Embedded Skills Reference

This section provides detailed documentation for each built-in skill, including their full configuration, capabilities, and prompt content.

### Schema Explainer

**ID:** `schema-explainer`
**Category:** Understanding
**Priority:** 90

Provides clear, educational explanations of database tables, columns, and their purposes.

#### Configuration

```yaml
requires:
  context: [schema, database_type]
  tools: [introspect_schema, get_sample_data]

triggers:
  task_types: [explain, understand, describe, what]
  entity_types: [table, column, schema]

compose:
  compatible_with: [relationship-mapper]
  conflicts_with: []
  mode: merge
```

#### Capabilities

This skill analyzes database schemas to provide:

1. **Naming Analysis**
   - Interprets table name conventions (singular/plural, prefixes)
   - Identifies domain context (users, orders, products, etc.)
   - Recognizes pattern suffixes (`_log`, `_history`, `_archive`, `_temp`)

2. **Column Analysis**
   - Primary key identification and type
   - Foreign key relationships
   - Common column patterns:
     - `created_at`, `updated_at` → Audit timestamps
     - `is_*`, `has_*` → Boolean flags
     - `*_id` → Foreign key references
     - `status`, `state` → State machine columns

3. **Design Pattern Recognition**
   - Junction/bridge tables for many-to-many
   - Self-referential hierarchies
   - Polymorphic associations
   - Soft delete patterns
   - Audit trail patterns

#### Output Format

- **Purpose**: One-sentence description
- **Role**: Transaction | Reference | Junction | Audit | Config | Cache
- **Key Columns**: Table with column, type, and purpose
- **Relationships**: List of related tables
- **Design Notes**: Notable patterns
- **Confidence**: High/Medium/Low

---

### Relationship Mapper

**ID:** `relationship-mapper`
**Category:** Understanding
**Priority:** 85

Maps and explains database relationships, including implicit relationships inferred from naming patterns.

#### Configuration

```yaml
requires:
  context: [schema]
  tools: [introspect_schema]

triggers:
  task_types: [relationships, connections, links, references]
  entity_types: [table, schema]

compose:
  compatible_with: [schema-explainer]
  conflicts_with: []
  mode: merge
```

#### Capabilities

This skill identifies and explains:

1. **Explicit Relationships**
   - Foreign key constraints
   - Junction tables for many-to-many

2. **Implicit Relationships (Inferred)**
   - Naming conventions (`user_id` likely references `users.id`)
   - Pattern matching (`author_id` likely references users/authors)
   - Common prefixes (tables with shared prefixes often relate)

3. **Analysis Process**
   - Catalogs explicit FKs
   - Identifies implicit FKs (`*_id` columns without constraints)
   - Detects junction tables (tables with only foreign keys)
   - Maps cardinality (1:1, 1:N, M:N)
   - Identifies hierarchies (self-referential via `parent_id`)

#### Output Format

- **Explicit Relationships**: Table with from/to columns and constraint types
- **Inferred Relationships**: Table with confidence levels and evidence
- **Relationship Graph**: ASCII diagram showing connections
- **Notable Patterns**: Hierarchies, polymorphic, audit trails

---

### Query Explainer

**ID:** `query-explainer`
**Category:** Understanding
**Priority:** 95

Explains what SQL queries do in plain language, breaking down their structure and logic.

#### Configuration

```yaml
requires:
  context: [query, schema, database_type]
  tools: [parse_query, explain_query]

triggers:
  task_types: [explain, understand, describe, what, how]
  entity_types: [query, sql]

compose:
  compatible_with: [schema-explainer]
  conflicts_with: [query-optimizer]
  mode: merge
  prompt_position: append
```

#### Capabilities

This skill provides comprehensive query explanations:

1. **Query Parsing**
   - Identifies query type (SELECT, INSERT, UPDATE, DELETE)
   - Breaks down each clause

2. **Component Explanation**
   - FROM/JOIN: Tables involved and relationships
   - WHERE: Filters applied
   - GROUP BY: Aggregation logic
   - HAVING: Post-aggregation filters
   - ORDER BY/LIMIT: Sorting and pagination

3. **Data Modification Analysis**
   - INSERT: What data is added and where
   - UPDATE: Which fields change, which rows affected
   - DELETE: Which rows removed

4. **Subquery/CTE Analysis**
   - Purpose of subquery
   - Relationship to outer query
   - Data flow contribution

#### Output Format

- **Query Summary**: Business-level description
- **Query Type**: Classification
- **Tables Involved**: With roles and aliases
- **Step-by-Step Breakdown**: Numbered clause explanations
- **Filters and Conditions**: Description of each condition
- **Result Set**: Columns, row count estimate, ordering
- **Key Observations**: Patterns, issues, performance notes

#### Verbosity Levels

- **Concise**: Summary and key points only
- **Balanced**: Summary + breakdown + observations
- **Detailed**: Full analysis with examples

---

### Query Comparer

**ID:** `query-comparer`
**Category:** Understanding
**Priority:** 85

Compares two SQL queries to identify structural and semantic differences.

#### Configuration

```yaml
requires:
  context: [query, schema, database_type]
  tools: [parse_query]

triggers:
  task_types: [compare, diff, difference, versus, vs]
  entity_types: [query, sql]

compose:
  compatible_with: [query-explainer]
  conflicts_with: []
  mode: merge
  prompt_position: append
```

#### Capabilities

This skill performs multi-dimensional comparison:

1. **Structural Comparison**
   - Query type (SELECT, UPDATE, etc.)
   - Tables involved
   - JOIN types and conditions
   - WHERE clause conditions
   - Columns selected or modified
   - Aggregation and grouping
   - Ordering and limits

2. **Semantic Comparison**
   - Do they return the same data?
   - Do they modify the same rows?
   - Edge cases where results differ
   - Superset/subset relationships

3. **Performance Comparison**
   - Which might be faster and why
   - Index utilization differences
   - Join order differences

4. **Pattern Detection**
   - Refactoring patterns (subquery → JOIN)
   - Optimization patterns (reordered JOINs)
   - Behavioral differences (NULL handling, type coercion)

#### Output Format

- **Quick Summary**: One-sentence relationship (Equivalent/Similar/Different/Unrelated)
- **Side-by-Side Analysis**: Table comparing each aspect
- **Key Differences**: Detailed impact analysis
- **Semantic Analysis**: Result set comparison
- **Recommendations**: Which to prefer and why

---

### Query Optimizer

**ID:** `query-optimizer`
**Category:** Performance
**Priority:** 100

Analyzes SQL queries and suggests performance improvements based on schema and indexes.

#### Configuration

```yaml
requires:
  context: [query, schema, database_type]
  tools: [explain_query, get_table_stats]

triggers:
  task_types: [optimize, performance, slow, analyze]
  entity_types: [query]

compose:
  compatible_with: [security-audit]
  conflicts_with: []
  mode: merge
```

#### Capabilities

This skill identifies and fixes performance issues:

1. **Index-Related Issues**
   - Missing indexes on WHERE clause columns
   - Missing indexes on JOIN columns
   - Missing composite indexes for multi-column filters
   - Unused or redundant indexes

2. **Query Structure Issues**
   - SELECT \* when specific columns would suffice
   - Unnecessary subqueries that could be JOINs
   - OR conditions that prevent index usage
   - LIKE patterns starting with wildcards
   - Implicit type conversions

3. **Join Optimization**
   - Inefficient join order
   - Missing join conditions (accidental cross joins)
   - Suboptimal join types

4. **Aggregation/Sorting Issues**
   - Missing indexes for ORDER BY
   - GROUP BY on non-indexed columns
   - Unnecessary DISTINCT

#### Output Format

- **Summary**: One-line efficiency assessment
- **Issues Found**: Bulleted with severity indicators
  - 🔴 **Critical**: Major performance problem
  - 🟡 **Warning**: Should be addressed
  - 🟢 **Minor**: Nice to fix
- **Recommendations**: Numbered with SQL examples
- **Expected Impact**: Qualitative improvement estimate

#### Database-Specific Notes

- **PostgreSQL**: EXPLAIN ANALYZE, partial indexes, CTEs
- **MySQL**: EXPLAIN FORMAT=JSON, covering indexes, query hints
- **SQLite**: EXPLAIN QUERY PLAN, virtual tables

---

### Collection Explainer

**ID:** `collection-explainer`
**Category:** Understanding
**Priority:** 90

Explains MongoDB collections, their document structure, and purpose.

#### Configuration

```yaml
requires:
  context: [collection, schema, database_type]
  tools: [get_collection_info, get_sample_documents]

triggers:
  task_types: [explain, understand, describe, what]
  entity_types: [collection, document, mongodb]

compose:
  compatible_with: [relationship-mapper]
  conflicts_with: []
  mode: merge
  prompt_position: append
```

#### Capabilities

This skill analyzes MongoDB collections:

1. **Document Structure Analysis**
   - Field categories: identity, data, embedded, arrays, references, metadata
   - Type inference from sample documents
   - Required vs optional fields

2. **Pattern Recognition**
   - Embedding patterns (related data nested)
   - Referencing patterns (ObjectId links)
   - Bucketing (time-series/grouped data)
   - Outlier pattern (separate storage for large fields)
   - Schema versioning
   - Polymorphism (type discriminators)

3. **Relationship Mapping**
   - Embedded document purposes
   - Reference relationships to other collections
   - Cardinality (1:1, 1:N, M:N)

#### Output Format

- **Purpose**: One-sentence description
- **Document Pattern**: Flat | Embedded | Referenced | Hybrid
- **Estimated Size**: Based on sample analysis
- **Document Structure**: JavaScript object with field descriptions
- **Field Details**: Table with type, required, purpose, indexed
- **Embedded Documents**: Path, purpose, pattern
- **References**: Field, target collection, relationship type
- **Indexes**: Observed or recommended with purposes
- **Design Observations**: Strengths, considerations, query patterns

#### MongoDB-Specific Considerations

- Atomicity: Single-document operations are atomic
- 16MB Limit: Document size considerations
- Sharding: ID or shard key distribution
- TTL Indexes: Time-based expiration
- Change Streams: Real-time tracking

---

### Security Audit

**ID:** `security-audit`
**Category:** Security
**Priority:** 95

Identifies potential security issues in database schema and queries.

#### Configuration

```yaml
requires:
  context: [schema, database_type]
  tools: [introspect_schema]

triggers:
  task_types: [security, audit, vulnerability, pii, sensitive]
  entity_types: [table, column, schema, query]

compose:
  compatible_with: [query-optimizer, schema-explainer]
  conflicts_with: []
  mode: merge
```

#### Capabilities

This skill performs comprehensive security analysis:

1. **Sensitive Data Identification**

   **Personal Identifiable Information (PII)**
   - Names, emails, phone numbers
   - Addresses, government IDs
   - Financial data, dates of birth

   **Protected Health Information (PHI)**
   - Medical records, health conditions
   - Treatment data

   **Authentication Data**
   - Passwords and hashes
   - API keys, tokens
   - Security questions

2. **Schema Security Issues**
   - Sensitive columns stored in plaintext
   - Password fields without hash indicator
   - Missing unique constraints on identifiers
   - Missing data lifecycle columns

3. **Query Security Issues**
   - SQL injection vulnerabilities
   - String concatenation in queries
   - Dynamic table/column names
   - SELECT \* exposing unnecessary columns
   - Missing WHERE clauses

#### Output Format

- **Risk Level**: 🔴 Critical | 🟡 Medium | 🟢 Low
- **Sensitive Data Inventory**: Table with sensitivity classification
- **Security Findings**: Organized by severity
- **Recommendations**: Categorized with affected items and actions

#### Compliance Considerations

- **GDPR**: Right to erasure, data minimization
- **HIPAA**: PHI protection requirements
- **PCI-DSS**: Cardholder data protection
- **SOC 2**: Access control, audit logging

---

### Onboarding Generator

**ID:** `onboarding-generator`
**Category:** Documentation
**Priority:** 80

Creates comprehensive onboarding documentation for new team members joining a database project.

#### Configuration

```yaml
requires:
  context: [schema, database_type]
  tools: [introspect_schema, get_sample_data]

triggers:
  task_types: [onboard, document, overview, guide, introduction]
  entity_types: [database, schema]

compose:
  compatible_with: [schema-explainer, relationship-mapper]
  conflicts_with: []
  mode: merge
  prompt_position: append
```

#### Capabilities

This skill generates complete documentation:

1. **Database Overview**
   - Database type and version
   - Primary domain/purpose
   - Overall schema size (tables, relationships)
   - Key conventions used

2. **Core Entities**
   - 5-10 most important tables identified
   - What each represents
   - Key columns and meanings
   - Important constraints

3. **Relationship Map**
   - Primary entity relationships
   - Junction tables and purposes
   - Hierarchy/tree structures

4. **Common Patterns**
   - Naming conventions
   - Audit/timestamp patterns
   - Soft delete patterns
   - Status/enum patterns

5. **Getting Started Queries**
   - 3-5 example queries for newcomers
   - Basic data exploration
   - Common joins
   - Typical filtering patterns

6. **Tips & Gotchas**
   - Performance considerations
   - Data integrity rules
   - Common mistakes to avoid

#### Output Format

Generates well-structured markdown documentation with:

- Clear headers for each section
- Tables for structured data
- SQL code blocks for examples
- Bullet points for lists
- Bold for important terms
- Friendly, welcoming tone

---

## Creating Custom Skills

### Step 1: Create the Skill File

Create a new `.md` file in your local skills directory:

```bash
# Create directory if needed
mkdir -p ~/.config/<skillsConfigDirName>/skills/

# Create your skill file
touch ~/.config/<skillsConfigDirName>/skills/my-custom-skill.md
```

### Step 2: Add YAML Frontmatter

Every skill starts with YAML frontmatter between `---` markers:

```yaml
---
id: my-custom-skill
version: 1.0.0
name: My Custom Skill
description: Brief description of what this skill does

feature:
  category: understanding # understanding, performance, security, documentation
  default_enabled: true # Whether enabled by default for new workspaces

requires:
  context: [schema, database_type] # What context this skill needs
  tools: [introspect_schema] # What tools this skill may use

triggers:
  task_types: [analyze, check] # Keywords that trigger this skill
  entity_types: [table, schema] # Entity types this skill handles

compose:
  compatible_with: [] # Skills that can be combined with this one
  conflicts_with: [] # Skills that cannot run together
  priority: 50 # Higher priority = selected first (0-100)
  mode: merge # merge, chain, or parallel
  prompt_position: append # prepend, append, or replace
---
```

### Step 3: Write the Prompt Content

After the frontmatter, write your skill's prompt in markdown:

```markdown
# Your Skill Title

You are an expert at [your specialty]. Your goal is to [primary objective].

## Your Approach

1. First, [step one]
2. Then, [step two]
3. Finally, [step three]

## What to Look For

- [Pattern or issue 1]
- [Pattern or issue 2]
- [Pattern or issue 3]

## Output Format

Provide your analysis as:

### Summary

[One-line summary]

### Findings

[Detailed findings]

### Recommendations

[Actionable recommendations]
```

### Step 4: Reload Skills

After creating your skill, reload the skill registry:

```typescript
import { reloadSkills } from "@/lib/tauri/skills";

await reloadSkills();
```

Or restart the application.

---

## Modifying Skills

### Modifying Embedded Skills

Embedded skills are compiled into the application. To modify them:

1. **Locate the skill file**

   ```
   src-tauri/src/skills_embedded/<skill-name>.md
   ```

2. **Edit the skill content**
   - Modify YAML frontmatter for configuration changes
   - Edit markdown content for prompt changes

3. **Rebuild the application**

   ```bash
   cd src-tauri
   cargo build
   ```

4. **Test your changes**
   ```bash
   cargo test --lib skills::
   ```

### Modifying Local Skills

Local skills can be modified directly:

1. **Find your skills directory**

   ```bash
   # Linux/macOS
   cd ~/.config/<skillsConfigDirName>/skills/

   # Windows
   cd %APPDATA%\<skillsConfigDirName>\skills\
   ```

2. **Edit the skill file**
   Open the `.md` file in any text editor.

3. **Reload skills**
   ```typescript
   await reloadSkills();
   ```
   Or restart the application.

### Common Modifications

#### Adding Trigger Words

Expand when a skill activates:

```yaml
triggers:
  task_types: [explain, understand, describe, what, how, tell] # Added 'how', 'tell'
  entity_types: [table, column, schema, entity] # Added 'entity'
```

#### Changing Priority

Adjust selection order:

```yaml
compose:
  priority: 100 # Higher = selected first (was 85)
```

#### Adding Compatibility

Allow combination with other skills:

```yaml
compose:
  compatible_with: [schema-explainer, relationship-mapper, security-audit]
```

#### Enhancing the Prompt

Add new sections to the prompt:

```markdown
## Additional Checks

### Data Privacy

- Check for encrypted storage of sensitive fields
- Verify PII handling compliance

### Scalability

- Identify potential bottlenecks
- Suggest partitioning strategies
```

#### Adding Tool Requirements

Request additional tools:

```yaml
requires:
  tools: [introspect_schema, get_sample_data, get_table_stats] # Added get_table_stats
```

### Creating Overrides

To customize an embedded skill without modifying source code:

1. **Create a local skill with the same ID**

   ```bash
   touch ~/.config/<skillsConfigDirName>/skills/schema-explainer.md
   ```

2. **Copy and modify the content**
   The local version will take precedence over the embedded version.

3. **Or use inheritance**

   ```yaml
   ---
   id: enhanced-schema-explainer
   compose:
     extends: schema-explainer # Inherit from base
     priority: 95 # Override priority
   ---
   # Additional instructions to prepend/append

   Also check for these patterns specific to our team...
   ```

### Version Control Tips

1. **Track local skills in git**

   ```bash
   # Create a skills repo
   cd ~/.config/<skillsConfigDirName>/skills/
   git init
   git add .
   git commit -m "Initial skills"
   ```

2. **Use semantic versioning**

   ```yaml
   version: 1.2.0 # Bump appropriately
   ```

3. **Add a changelog comment**
   ```yaml
   ---
   id: my-skill
   version: 1.2.0
   # Changelog:
   # 1.2.0 - Added new trigger words, enhanced output format
   # 1.1.0 - Added compatibility with security-audit
   # 1.0.0 - Initial version
   ---
   ```

---

## Skill Examples

### Example 1: Data Quality Checker

A skill that identifies potential data quality issues:

```markdown
---
id: data-quality-checker
version: 1.0.0
name: Data Quality Checker
description: Identifies potential data quality issues in table schemas

feature:
  category: understanding
  default_enabled: true

requires:
  context: [schema, table, database_type]
  tools: [introspect_schema, get_sample_data]

triggers:
  task_types: [quality, validate, check, audit]
  entity_types: [table, schema]

compose:
  compatible_with: [schema-explainer]
  conflicts_with: []
  priority: 75
  mode: merge
  prompt_position: append
---

# Data Quality Expert

You are a data quality specialist. Analyze database schemas to identify potential data quality issues.

## Quality Dimensions to Check

### Completeness

- Missing NOT NULL constraints on required fields
- Tables without primary keys
- Orphaned records potential (missing foreign keys)

### Consistency

- Inconsistent naming conventions
- Mixed data types for similar concepts
- Denormalized data that could drift

### Accuracy

- Missing check constraints for valid ranges
- Enum columns without constraints
- Date columns without reasonable bounds

### Timeliness

- Missing timestamp columns (created_at, updated_at)
- No audit trail capability
- Stale data indicators

## Output Format

### Data Quality Score

Rate overall quality: Excellent / Good / Fair / Poor

### Issues Found

For each issue:

- **Severity**: Critical / Warning / Info
- **Dimension**: Completeness / Consistency / Accuracy / Timeliness
- **Location**: Table and column
- **Description**: What the issue is
- **Recommendation**: How to fix it

### Quick Wins

List 3-5 easy improvements that would have the biggest impact.
```

### Example 2: Naming Convention Enforcer

A skill focused on naming standards:

```markdown
---
id: naming-convention-checker
version: 1.0.0
name: Naming Convention Checker
description: Validates schema naming against common conventions

feature:
  category: understanding
  default_enabled: false

requires:
  context: [schema, database_type]
  tools: [introspect_schema]

triggers:
  task_types: [naming, convention, standards, lint]
  entity_types: [schema, table, column]

compose:
  compatible_with: [schema-explainer, data-quality-checker]
  conflicts_with: []
  priority: 60
  mode: merge
  prompt_position: append
---

# Naming Convention Specialist

You analyze database schemas for naming convention compliance.

## Conventions to Check

### Table Names

- Should be plural (users, orders, products)
- Should be snake_case
- Should not have prefixes like "tbl\_"
- Junction tables: table1_table2 format

### Column Names

- Should be snake_case
- Primary keys: "id" or "table_id"
- Foreign keys: "referenced_table_id"
- Booleans: "is*", "has*", "can\_" prefixes
- Timestamps: "\_at" suffix (created_at, updated_at)
- Counts: "\_count" suffix

### Index Names

- Format: idx_table_column(s)
- Unique: uq_table_column

### Constraint Names

- Primary key: pk_table
- Foreign key: fk_table_referenced
- Check: ck_table_column

## Output Format

### Compliance Score

X% of names follow conventions

### Violations

| Entity | Current Name | Suggested Name | Rule |
| ------ | ------------ | -------------- | ---- |
| ...    | ...          | ...            | ...  |

### Patterns Detected

Describe any consistent patterns (good or bad) found.
```

### Example 3: Performance Anti-Pattern Detector

A specialized performance skill:

```markdown
---
id: perf-antipattern-detector
version: 1.0.0
name: Performance Anti-Pattern Detector
description: Identifies common database performance anti-patterns

feature:
  category: performance
  default_enabled: true

requires:
  context: [schema, database_type]
  tools: [introspect_schema, get_table_stats]

triggers:
  task_types: [performance, slow, optimize, antipattern]
  entity_types: [schema, table]

compose:
  compatible_with: [query-optimizer]
  conflicts_with: []
  priority: 85
  mode: merge
  prompt_position: append
---

# Performance Anti-Pattern Specialist

You identify database design patterns that commonly cause performance issues.

## Anti-Patterns to Detect

### Schema Anti-Patterns

1. **God Table**: Tables with 50+ columns
2. **Missing Indexes**: Foreign keys without indexes
3. **Over-Indexing**: Tables with more indexes than columns
4. **Wide Primary Keys**: Composite PKs with 3+ columns
5. **No Partitioning Candidates**: Large tables without date columns

### Data Type Anti-Patterns

1. **String IDs**: Using VARCHAR for IDs instead of INT/UUID
2. **Oversized Columns**: VARCHAR(4000) for short strings
3. **Text for Enums**: Using TEXT instead of ENUM/CHECK
4. **Precision Loss**: Using FLOAT for currency

### Relationship Anti-Patterns

1. **Missing FKs**: Implied relationships without constraints
2. **Circular Dependencies**: A→B→C→A relationships
3. **Deep Hierarchies**: Self-referential > 5 levels common

## Output Format

### Risk Assessment

Overall performance risk: High / Medium / Low

### Anti-Patterns Found

For each pattern:
```

Pattern: [Name]
Location: [Table/Column]
Impact: [What performance problem this causes]
Fix: [How to resolve it]
Effort: [Easy/Medium/Hard]

```

### Priority Fixes
Top 3 changes that would most improve performance.
```

### Example 4: Domain-Specific Skill (E-Commerce)

A skill tailored for a specific domain:

```markdown
---
id: ecommerce-schema-reviewer
version: 1.0.0
name: E-Commerce Schema Reviewer
description: Reviews schemas for e-commerce best practices

feature:
  category: understanding
  default_enabled: false

requires:
  context: [schema, database_type]
  tools: [introspect_schema, get_sample_data]

triggers:
  task_types: [ecommerce, shop, store, commerce, retail]
  entity_types: [schema]

compose:
  compatible_with: [schema-explainer, security-audit]
  conflicts_with: []
  priority: 70
  mode: merge
  prompt_position: append
---

# E-Commerce Database Specialist

You are an expert in e-commerce database design. Review schemas for e-commerce best practices.

## Essential E-Commerce Tables

Check for presence and proper design of:

### Core Entities

- **Products**: SKU, name, description, price, inventory
- **Categories**: Hierarchical category structure
- **Customers**: Contact info, addresses, account status
- **Orders**: Order status, totals, timestamps
- **Order Items**: Line items with price snapshots

### Supporting Entities

- **Addresses**: Shipping/billing address management
- **Payments**: Payment method, transaction records
- **Inventory**: Stock levels, warehouse locations
- **Reviews**: Product reviews and ratings
- **Wishlists**: Saved items

### Audit & History

- **Price History**: Track price changes
- **Order Status History**: Status transitions
- **Inventory Movements**: Stock adjustments

## E-Commerce Best Practices

1. **Price Snapshots**: Store prices at order time, not references
2. **Soft Deletes**: Never hard delete products/customers
3. **Address Versioning**: Keep address history for orders
4. **Tax Handling**: Separate tax calculation storage
5. **Currency Support**: Multi-currency considerations
6. **Stock Reservations**: Inventory hold during checkout

## Security Considerations

- PCI compliance for payment data
- PII protection for customer data
- Encryption for sensitive fields

## Output Format

### E-Commerce Readiness Score

Rate: Production Ready / Needs Work / Major Gaps

### Missing Components

Tables or features that should be added.

### Design Issues

Problems with current implementation.

### Recommendations

Prioritized list of improvements.
```

---

## Best Practices

### Skill Design

#### 1. Single Responsibility

Each skill should do one thing well. Don't create a "do everything" skill.

```yaml
# Good: Focused skill
id: query-optimizer
description: Analyzes queries and suggests performance improvements

# Bad: Too broad
id: database-helper
description: Helps with all database tasks
```

#### 2. Clear Trigger Words

Choose trigger words that users naturally use:

```yaml
# Good: Natural language triggers
triggers:
  task_types: [optimize, slow, performance, speed, faster]

# Bad: Technical jargon only
triggers:
  task_types: [perf-tune, idx-opt]
```

#### 3. Specific Output Format

Define exactly what output looks like:

```markdown
## Output Format

### Summary

One sentence describing the main finding.

### Issues (use this exact format)

- **[Severity]**: [Description] in `[location]`

### Recommendations

1. [First action]
2. [Second action]
```

#### 4. Request Only Needed Context

Don't request context you won't use:

```yaml
# Good: Only what's needed
requires:
  context: [query, database_type]

# Bad: Everything just in case
requires:
  context: [query, schema, table, column, database_type, workspace_id]
```

### Prompt Engineering

#### 1. Start with Role Definition

```markdown
# Query Optimization Expert

You are an expert database query optimizer with 15 years of experience
optimizing queries for PostgreSQL, MySQL, and SQLite.
```

#### 2. Provide Step-by-Step Instructions

```markdown
## Your Approach

1. **Parse the Query**: Identify tables, joins, and conditions
2. **Check Indexes**: Verify indexes exist for filtered columns
3. **Analyze Execution**: Review the query plan
4. **Suggest Improvements**: Provide specific, actionable fixes
```

#### 3. Include Examples in Output Format

```markdown
## Output Format

### Issues Found

Example:

- 🔴 **Critical**: Missing index on `users.email` used in WHERE clause
- 🟡 **Warning**: SELECT \* could be replaced with specific columns
```

#### 4. Handle Edge Cases

```markdown
## Special Cases

- If the query is already optimal, say so and explain why
- If you can't determine optimization without more context, ask for it
- If the query has syntax errors, point them out first
```

### Composition Guidelines

#### 1. Declare Compatibility Thoughtfully

```yaml
compose:
  compatible_with: [schema-explainer] # Complements this skill
  conflicts_with: [query-optimizer] # Would duplicate work
```

#### 2. Use Priority to Control Order

```yaml
# Security should run before optimization
# security-audit
compose:
  priority: 95

# query-optimizer
compose:
  priority: 85
```

#### 3. Choose the Right Mode

```yaml
# Merge: Combine prompts for comprehensive analysis
mode: merge

# Chain: Sequential processing where output feeds next skill
mode: chain

# Parallel: Independent analyses combined at the end
mode: parallel
```

---

## Tips and Tricks

### Effective Skill Selection

#### Tip 1: Use Task Hints for Better Selection

```typescript
// Good: Provide hints
await executeWithSkills(workspaceId, request, providerId, apiKey, {
  entityType: "query",
  taskHint: "optimize",
});

// Less optimal: No hints
await executeWithSkills(workspaceId, request, providerId, apiKey);
```

#### Tip 2: Get Suggestions Before Executing

```typescript
// Show users what skills will be used
const suggestions = await suggestSkills(workspaceId, userQuery, databaseType);
const skillNames = suggestions.map((s) => s.skillName).join(", ");
console.log(`Using skills: ${skillNames}`);
```

#### Tip 3: Execute Specific Skills for Known Tasks

```typescript
// When you know exactly what's needed
await executeSkill(workspaceId, "query-optimizer", query, providerId, apiKey);

// Instead of
await executeWithSkills(workspaceId, `Optimize: ${query}`, providerId, apiKey);
```

### Debugging Skills

#### Tip 4: Check Skill Loading

```typescript
// Verify your skill was loaded
const skills = await listAvailableSkills();
const mySkill = skills.find((s) => s.id === "my-custom-skill");
if (!mySkill) {
  console.error("Skill not found! Check file location and syntax.");
}
```

#### Tip 5: Validate YAML Syntax

```bash
# Use a YAML linter
yamllint ~/.config/<skillsConfigDirName>/skills/my-skill.md

# Or check in Python
python -c "import yaml; yaml.safe_load(open('my-skill.md').read().split('---')[1])"
```

#### Tip 6: Test Triggers

```typescript
// Test if your triggers work
const suggestions = await suggestSkills(
  workspaceId,
  "check data quality", // Should match your trigger words
  "postgresql",
);
console.log(suggestions);
```

### Performance Optimization

#### Tip 7: Minimize Context Requirements

Skills run faster with less context to gather:

```yaml
# Fast: Minimal context
requires:
  context: [database_type]

# Slower: Needs to introspect full schema
requires:
  context: [schema, database_type]
```

#### Tip 8: Use Appropriate Tools

Only request tools you'll actually call:

```yaml
# Good: Only what's needed
requires:
  tools: [get_table_stats]

# Wasteful: Tools you won't use
requires:
  tools: [introspect_schema, execute_query, explain_query, get_table_stats]
```

### Workspace Management

#### Tip 9: Initialize Defaults for New Workspaces

```typescript
// When creating a new workspace
await initializeSkillDefaults(newWorkspaceId);
```

#### Tip 10: Per-Workspace Customization

Different workspaces can have different skills enabled:

```typescript
// Production database: Enable security, disable experimental
await setSkillEnabled(prodWorkspaceId, "security-audit", true);
await setSkillEnabled(prodWorkspaceId, "experimental-skill", false);

// Dev database: Enable experimental
await setSkillEnabled(devWorkspaceId, "experimental-skill", true);
```

### Advanced Techniques

#### Tip 11: Chain Skills for Complex Analysis

Create a skill that triggers a chain:

```yaml
compose:
  mode: chain
  compatible_with: [schema-explainer, security-audit, query-optimizer]
```

#### Tip 12: Create Domain-Specific Skill Sets

Group related skills in subdirectories:

```
~/.config/<skillsConfigDirName>/skills/
├── healthcare/
│   ├── hipaa-compliance.md
│   └── phi-detector.md
├── finance/
│   ├── pci-compliance.md
│   └── transaction-patterns.md
└── general/
    └── data-quality.md
```

#### Tip 13: Version Your Skills

Use version numbers and track changes:

```yaml
id: my-skill
version: 1.2.0 # Bump when you update
```

#### Tip 14: Test Skills Incrementally

Start simple, add complexity:

```markdown
# Version 1: Basic

You analyze tables. List the columns.

# Version 2: Add structure

You analyze tables.

## Steps

1. List columns
2. Identify types

# Version 3: Full implementation

[Complete prompt]
```

---

## Usage

### Frontend Integration

#### Loading Skills

```typescript
import { useSkillStore } from "@/stores/skillStore";

function SkillsPanel() {
  const { loadSkills, loadSettings, skillsByCategory, isLoading } = useSkillStore();

  useEffect(() => {
    loadSkills();
    loadSettings(workspaceId);
  }, [workspaceId]);

  return (
    <div>
      {Object.entries(skillsByCategory).map(([category, skills]) => (
        <div key={category}>
          <h3>{category}</h3>
          {skills.map(skill => (
            <SkillCard key={skill.id} skill={skill} />
          ))}
        </div>
      ))}
    </div>
  );
}
```

#### Toggling Skills

```typescript
import { useSkillStore } from "@/stores/skillStore";

function SkillToggle({ skillId }: { skillId: string }) {
  const { toggleSkill, settings } = useSkillStore();
  const isEnabled = settings?.enabledSkills.includes(skillId) ?? false;

  return (
    <Switch
      checked={isEnabled}
      onCheckedChange={(checked) => toggleSkill(skillId, checked)}
    />
  );
}
```

#### Executing Skills

```typescript
import { executeWithSkills, executeSkill } from "@/lib/tauri/skills";

// Automatic skill selection based on request
const result = await executeWithSkills(
  workspaceId,
  "Explain the users table",
  providerId,
  apiKey,
  { entityType: "table", taskHint: "explain" },
);

// Execute a specific skill
const result = await executeSkill(
  workspaceId,
  "query-optimizer",
  "SELECT * FROM users WHERE status = 'active'",
  providerId,
  apiKey,
);
```

#### Getting Skill Suggestions

```typescript
import { suggestSkills } from "@/lib/tauri/skills";

// Get skill suggestions for a user request
const suggestions = await suggestSkills(
  workspaceId,
  "How can I optimize this query?",
  "postgresql",
);

// suggestions = [
//   { skillId: "query-optimizer", skillName: "Query Optimizer", relevanceScore: 0.8, ... },
//   { skillId: "security-audit", skillName: "Security Audit", relevanceScore: 0.2, ... }
// ]
```

### Backend Integration

#### Using the Skill Engine

```rust
use crate::skills::{get_or_init_registry, SkillEngine, SelectionContext};
use crate::adapters::AiboilerplateAdapter;

// Get the shared registry
let registry = get_or_init_registry().await;

// Create adapter with context
let adapter = Arc::new(
    AiboilerplateAdapter::new(connection_manager, workspace_id)
        .with_database_type("postgresql")
        .with_query("SELECT * FROM users")
);

// Create engine
let engine = SkillEngine::new(adapter, registry);

// Build selection context
let context = SelectionContext {
    request: "Explain this query".to_string(),
    database_type: Some("postgresql".to_string()),
    entity_type: Some("query".to_string()),
    task_hint: Some("explain".to_string()),
    available_context: vec!["query".into(), "schema".into()],
    enabled_skills: settings.enabled_skills,
};

// Execute with automatic skill selection
let output = engine.execute(&request, context, llm_provider).await?;
```

---

## Skill Definition Format

### Complete Reference

```yaml
---
# Required fields
id: my-skill # Unique identifier (kebab-case)
version: 1.0.0 # Semantic version
name: My Skill # Display name
description: What it does # Brief description

# Feature configuration
feature:
  category: understanding # understanding, performance, security, documentation
  default_enabled: true # Default state for new workspaces

# Requirements
requires:
  context: # Context keys needed
    - schema
    - database_type
  tools: # Tools the skill may use
    - introspect_schema
    - get_sample_data

# Trigger configuration
triggers:
  task_types: # Keywords that trigger this skill
    - explain
    - understand
  entity_types: # Entity types handled
    - table
    - column

# Composition settings
compose:
  extends: base-skill # Optional: inherit from another skill
  compatible_with: # Skills that can be combined
    - schema-explainer
  conflicts_with: # Skills that cannot run together
    - competing-skill
  priority: 50 # Selection priority (0-100, higher = first)
  mode: merge # merge, chain, or parallel
  prompt_position: append # prepend, append, or replace
---
# Prompt Content (Markdown)

Your skill's prompt goes here...
```

### Available Context Keys

| Key             | Type   | Description                                              |
| --------------- | ------ | -------------------------------------------------------- |
| `database_type` | string | Database engine (postgresql, mysql, sqlite, mongodb)     |
| `schema`        | object | Full schema metadata (tables, relationships)             |
| `query`         | string | SQL query being analyzed                                 |
| `table`         | object | Current table metadata (columns, indexes, relationships) |
| `collection`    | object | MongoDB collection metadata (fields, samples)            |
| `workspace_id`  | string | Current workspace identifier                             |

### Available Tools

| Tool                   | Description                       | Parameters            |
| ---------------------- | --------------------------------- | --------------------- |
| `introspect_schema`    | Get full database schema metadata | None                  |
| `execute_query`        | Execute a SQL query               | `query`, `limit`      |
| `explain_query`        | Get query execution plan          | `query`               |
| `get_sample_data`      | Fetch sample rows from a table    | `table`, `limit`      |
| `get_table_stats`      | Get table statistics              | `table`               |
| `parse_query`          | Parse SQL and return structure    | `query`               |
| `get_collection_info`  | Get MongoDB collection metadata   | `collection`          |
| `get_sample_documents` | Fetch sample documents            | `collection`, `limit` |

---

## API Reference

### Tauri Commands

| Command                             | Description                            |
| ----------------------------------- | -------------------------------------- |
| `list_available_skills_command`     | List all available skills              |
| `get_skill_details_command`         | Get full skill definition              |
| `get_skill_settings_command`        | Get workspace skill settings           |
| `set_skill_enabled_command`         | Enable/disable a skill                 |
| `update_skill_config_command`       | Update skill configuration             |
| `initialize_skill_defaults_command` | Initialize default settings            |
| `execute_with_skills_command`       | Execute with automatic skill selection |
| `execute_skill_command`             | Execute a specific skill               |
| `reload_skills_command`             | Reload skills from all sources         |
| `list_skills_by_category_command`   | List skills grouped by category        |
| `set_skill_auto_select_command`     | Toggle auto-select mode                |
| `suggest_skills_command`            | Get skill suggestions for a request    |

### TypeScript Types

```typescript
interface SkillInfo {
  id: string;
  name: string;
  description: string;
  category: string;
  defaultEnabled: boolean;
  source: "Embedded" | "Local" | "Remote";
}

interface SkillDefinition {
  id: string;
  version: string;
  name: string;
  description: string;
  feature: FeatureConfig;
  requires: SkillRequirements;
  triggers: SkillTriggers;
  compose: ComposeConfig;
  promptContent: string;
}

interface SkillOutput {
  skillId: string;
  content: string;
  structuredData?: unknown;
  toolCallsMade: ToolCallRecord[];
  fromCache: boolean;
}

interface SkillSuggestion {
  skillId: string;
  skillName: string;
  description: string;
  relevanceScore: number;
  matchedTriggers: string[];
}
```

---

## Testing Skills

### Backend Unit Tests

Run all skill-related tests:

```bash
cd src-tauri
cargo test --lib skills::
```

Run with output for debugging:

```bash
cargo test --lib skills:: -- --nocapture
```

Test specific modules:

```bash
# Loader tests
cargo test --lib skills::loader::tests

# Selector tests
cargo test --lib skills::selector::tests

# Composer tests
cargo test --lib skills::composer::tests

# Executor tests
cargo test --lib skills::executor::tests
```

### Test Coverage

| Module     | Tests | Description                  |
| ---------- | ----- | ---------------------------- |
| `loader`   | 2     | Skill parsing and loading    |
| `selector` | 1     | Skill selection and matching |
| `composer` | 4     | Skill composition            |
| `executor` | 2     | Skill execution              |
| `registry` | 1     | Registry initialization      |
| `settings` | 2     | Settings management          |
| `state`    | 1     | State management             |
| `types`    | 2     | Type serialization           |
| `mod`      | 1     | Module exports               |

### Frontend Tests

```bash
pnpm run test
```

Test the skill store:

```bash
pnpm run test -- skillStore
```

### Testing Custom Skills

#### Step 1: Validate YAML Syntax

Before loading, verify your YAML is valid:

```bash
# Using Python
python3 -c "
import yaml
content = open('my-skill.md').read()
frontmatter = content.split('---')[1]
yaml.safe_load(frontmatter)
print('YAML is valid!')
"

# Using yamllint (if installed)
yamllint my-skill.md

# Using online tools
# Copy the YAML frontmatter to https://yaml-online-parser.appspot.com/
```

#### Step 2: Check Required Fields

Ensure all required fields are present:

```yaml
# Required fields checklist
id: my-skill # ✓ Unique identifier
version: 1.0.0 # ✓ Semantic version
name: My Skill # ✓ Display name
description: ... # ✓ Brief description
feature:
  category: ... # ✓ Category (understanding, performance, security, documentation)
```

#### Step 3: Load and Verify

```typescript
import { reloadSkills, listAvailableSkills } from "@/lib/tauri/skills";

// Reload to pick up changes
await reloadSkills();

// Verify skill was loaded
const skills = await listAvailableSkills();
const mySkill = skills.find((s) => s.id === "my-skill");

if (mySkill) {
  console.log("✓ Skill loaded successfully");
  console.log(`  Name: ${mySkill.name}`);
  console.log(`  Category: ${mySkill.category}`);
  console.log(`  Source: ${mySkill.source}`);
} else {
  console.error("✗ Skill not found! Check file location and YAML syntax.");
}
```

#### Step 4: Test Trigger Matching

```typescript
import { suggestSkills } from "@/lib/tauri/skills";

// Test various trigger phrases
const testPhrases = [
  "check data quality", // Should match your task_types
  "analyze the users table", // Should match your entity_types
  "optimize performance", // May or may not match
];

for (const phrase of testPhrases) {
  const suggestions = await suggestSkills(workspaceId, phrase, "postgresql");
  const match = suggestions.find((s) => s.skillId === "my-skill");

  if (match) {
    console.log(`✓ "${phrase}" → matched (score: ${match.relevanceScore})`);
    console.log(`  Matched triggers: ${match.matchedTriggers.join(", ")}`);
  } else {
    console.log(`✗ "${phrase}" → no match`);
  }
}
```

#### Step 5: Test Execution

```typescript
import { executeSkill } from "@/lib/tauri/skills";

try {
  const result = await executeSkill(
    workspaceId,
    "my-skill",
    "Test input for my skill",
    providerId,
    apiKey,
  );

  console.log("✓ Skill executed successfully");
  console.log(`  Output length: ${result.content.length} chars`);
  console.log(`  Tools called: ${result.toolCallsMade.length}`);
  console.log(`  From cache: ${result.fromCache}`);

  // Verify output structure matches expected format
  if (result.content.includes("### Summary")) {
    console.log("✓ Output contains expected sections");
  }
} catch (error) {
  console.error("✗ Execution failed:", error);
}
```

#### Step 6: Test Composition

If your skill declares compatibility, test composition:

```typescript
import { getSkillDetails } from "@/lib/tauri/skills";

const details = await getSkillDetails("my-skill");
console.log("Compatible with:", details.compose.compatibleWith);
console.log("Conflicts with:", details.compose.conflictsWith);

// Test combined execution
const result = await executeWithSkills(
  workspaceId,
  "Explain and check quality of users table", // Triggers multiple skills
  providerId,
  apiKey,
  { entityType: "table" },
);
```

### Interactive Testing Script

Create a test script for rapid iteration:

```typescript
// test-skill.ts
import { invoke } from "@tauri-apps/api/core";

async function testSkill(skillId: string) {
  console.log(`\n=== Testing Skill: ${skillId} ===\n`);

  // 1. Reload
  await invoke("reload_skills_command");
  console.log("✓ Skills reloaded");

  // 2. Check loaded
  const skills = (await invoke("list_available_skills_command")) as any[];
  const skill = skills.find((s) => s.id === skillId);

  if (!skill) {
    console.error("✗ Skill not found!");
    return;
  }
  console.log(`✓ Found: ${skill.name} (${skill.category})`);

  // 3. Get details
  const details = await invoke("get_skill_details_command", { skillId });
  console.log(`  Triggers: ${details.triggers.taskTypes.join(", ")}`);
  console.log(`  Priority: ${details.compose.priority}`);

  // 4. Test suggestions
  const suggestions = await invoke("suggest_skills_command", {
    workspaceId: "test-workspace",
    request: "test query",
    databaseType: "postgresql",
  });
  console.log(
    `  Suggestion rank: ${suggestions.findIndex((s) => s.skillId === skillId) + 1}`,
  );

  console.log("\n=== Test Complete ===\n");
}

// Run
testSkill("my-custom-skill");
```

### Debugging Common Issues

#### Skill Not Loading

```bash
# Check file exists
ls -la ~/.config/<skillsConfigDirName>/skills/my-skill.md

# Check permissions
stat ~/.config/<skillsConfigDirName>/skills/my-skill.md

# Check file encoding (should be UTF-8)
file ~/.config/<skillsConfigDirName>/skills/my-skill.md
```

#### YAML Parse Errors

Common mistakes and fixes:

```yaml
# ✗ Missing quotes around special characters
description: What it does: analyze data

# ✓ Use quotes
description: "What it does: analyze data"

# ✗ Incorrect indentation
triggers:
task_types: [a, b]

# ✓ Proper indentation
triggers:
  task_types: [a, b]

# ✗ Tabs instead of spaces
compose:
	priority: 50  # Uses tab

# ✓ Use spaces only
compose:
  priority: 50  # Uses spaces
```

#### Skill Not Triggering

1. Check if enabled:

   ```typescript
   const settings = await getSkillSettings(workspaceId);
   console.log("Enabled:", settings.enabledSkills.includes("my-skill"));
   ```

2. Check priority (higher wins):

   ```typescript
   const details = await getSkillDetails("my-skill");
   console.log("Priority:", details.compose.priority);
   ```

3. Check for conflicts:
   ```typescript
   const details = await getSkillDetails("my-skill");
   console.log("Conflicts:", details.compose.conflictsWith);
   ```

### Continuous Integration

Add skill tests to your CI pipeline:

```yaml
# .github/workflows/test.yml
- name: Test Skills
  run: |
    cd src-tauri
    cargo test --lib skills:: --no-fail-fast
```

---

## Troubleshooting

### Skill Not Loading

**Symptoms**: Skill doesn't appear in the list

**Causes & Fixes**:

1. **Wrong file location**

   ```bash
   # Check the correct path
   ls ~/.config/<skillsConfigDirName>/skills/
   ```

2. **Invalid YAML syntax**

   ```bash
   # Validate YAML
   yamllint your-skill.md
   ```

3. **Missing required fields**
   - Ensure `id`, `version`, `name`, `description` are present
   - Ensure `feature.category` is set

4. **File not reloaded**
   ```typescript
   await reloadSkills();
   ```

### Skill Not Triggering

**Symptoms**: Skill exists but isn't selected

**Causes & Fixes**:

1. **Triggers don't match**
   - Add more trigger words to `task_types`
   - Ensure `entity_types` match your query

2. **Skill disabled**

   ```typescript
   await setSkillEnabled(workspaceId, "your-skill", true);
   ```

3. **Low priority**
   - Increase `compose.priority` value

4. **Conflicts with another skill**
   - Check `conflicts_with` settings

### Skill Output Issues

**Symptoms**: Skill runs but output is poor

**Causes & Fixes**:

1. **Missing context**
   - Add required context keys to `requires.context`

2. **Vague prompt**
   - Be more specific in your instructions
   - Add examples to the output format

3. **Wrong tools**
   - Verify tools in `requires.tools` exist
   - Check tool parameters

---

## Future Enhancements

- **Remote Skill Registry**: Download community skills from a central registry
- **Local Skill Editor**: UI for creating and editing local skills
- **Skill Versioning**: Version management and updates
- **Skill Analytics**: Track skill usage and effectiveness
- **Skill Marketplace**: Share and discover community skills
- **Skill Templates**: Starter templates for common use cases
- **Skill Testing UI**: Interactive skill testing interface

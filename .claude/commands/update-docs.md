# Introspect Code & Update All Documentation Surfaces

This command reads the actual source code to count routes, tools, and providers, then updates every documentation surface (README, docs/, docs-site/, GitHub About) with the correct values.

**It runs fully automatically with NO user prompts.**

## Instructions

Follow these steps strictly and sequentially. Do NOT ask the user for confirmation between steps — proceed automatically.

### Step 1: Introspect source of truth

Count the actual values from code. Use Grep/Read tools — do NOT guess or use cached values.

#### 1a. Count routes

Read `no_commit/api-routes.md` (canonical route list). Extract:
- **Total** from the `## Total:` line at the bottom
- **Base routes** count from the `## Base Routes (N)` header
- **Feature-gated** breakdowns from the `## Feature: name (N)` headers

Do NOT re-parse `routes.rs` — the canonical file is the source of truth. If `no_commit/api-routes.md` does not exist, warn the user and skip route counting.

#### 1b. Count tools

Read `crates/zenii-core/src/boot.rs`. Count:
- **Base tools**: `tool_registry.register(` calls NOT inside `#[cfg(feature` blocks
- **Feature-gated tools**: `tool_registry.register(` calls inside `#[cfg(feature` blocks
- **Total**: base + feature-gated

#### 1c. Count providers

Read `crates/zenii-core/src/ai/providers.json`. Count entries in the `providers` array.

#### 1d. Print summary

Print the introspected values:
```
Introspected values:
- Routes: {core} core + {channels} channels + {scheduler} scheduler + {workflows} workflows = {total} total
- Tools: {base} base + {feature_gated} feature-gated = {total} total
- Providers: {count} AI providers
```

### Step 2: Find stale values

Search for stale/incorrect counts across all doc surfaces:

```
grep -rn "routes" README.md docs/ docs-site/ --include="*.md" | grep -E "\d+ (API )?routes"
grep -rn "built-in" README.md docs/ --include="*.md" | grep -E "\d+ built"
grep -rn "tools" README.md docs/ --include="*.md" | grep -E "\d+ (base |built-in )?tools"
grep -rn "providers" README.md docs/ --include="*.md" | grep -E "\d+ .* providers"
```

Print which files have stale values and what needs updating.

### Step 3: Update README.md

Read `README.md` and replace ALL occurrences of stale counts with the introspected values. Common patterns to find and replace:
- `{old_route_count} API routes` → `{new} API routes`
- `{old_route_count} routes` → `{new} routes`
- `{old_tool_count} built-in` → `{new} built-in`
- `{old_base} base + {old_gated} feature-gated` → `{new_base} base + {new_gated} feature-gated`
- Tool breakdown counts (e.g., `83 core + 9 channels + ...`)
- Any tool listing sections (ensure all tools are listed)
- Startup diagrams or ASCII art with counts

Use the Edit tool for each replacement. Do NOT rewrite the entire file.

### Step 4: Update docs/architecture.md

Read `docs/architecture.md` and replace stale route/tool counts in:
- Mermaid diagrams (node labels with counts)
- Prose descriptions mentioning route or tool totals
- Any tables with counts

### Step 5: Sync docs/ → docs-site/docs/

Mirror these files if they exist:
```
cp docs/api-reference.md docs-site/docs/api-reference.md
cp docs/cli-reference.md docs-site/docs/cli-reference.md
```

Only copy files that exist in the source. If `docs-site/docs/` doesn't exist, skip this step entirely.

### Step 6: Update GitHub About

Run:
```
gh repo view --json description -q '.description'
```

If the description contains a stale route count, update it:
```
gh repo edit --description "<updated description with correct route count>"
```

Keep the existing description style — only change the number. If the description doesn't mention routes, skip this step.

### Step 7: Verify

Run these verification checks and print results:

#### 7a. Check for stale route counts
Search for the OLD route count values (the ones that were replaced). If any remain, list them as warnings.

#### 7b. Check for stale tool counts
Search for the OLD tool count values. If any remain, list them as warnings.

#### 7c. Verify doc mirrors
If docs-site/ exists, diff the mirrored files:
```
diff docs/api-reference.md docs-site/docs/api-reference.md 2>/dev/null
diff docs/cli-reference.md docs-site/docs/cli-reference.md 2>/dev/null
```

#### 7d. Verify GitHub About
```
gh repo view --json description -q '.description'
```

### Step 8: Summary

Print a summary:
```
Documentation update complete:
- Routes: {old} → {new} (updated in {n} files)
- Tools: {old} → {new} (updated in {n} files)
- Providers: {count} (no change needed / updated)
- Files modified: {list}
- Mirrors synced: {list or "none needed"}
- GitHub About: {updated / no change needed}
- Stale references remaining: {count, should be 0}
```

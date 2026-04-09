# LLM Wiki

Zenii includes a Karpathy-pattern LLM wiki: a persistent, structured knowledge base that an LLM
maintains from raw sources. Unlike RAG (which re-synthesizes from raw docs on every query), the wiki
**compiles knowledge at ingestion time** — the LLM reads a document once, writes structured pages,
and maintains cross-references. Future queries draw on pre-built, interlinked knowledge.

---

## Structure

```
wiki/
  SCHEMA.md      ← operating manual for the LLM agent
  index.md       ← catalog of all pages (LLM-maintained)
  log.md         ← append-only operation history
  sources/       ← drop raw input documents here
  pages/
    concepts/    ← ideas, techniques, frameworks
    entities/    ← people, orgs, projects, products
    topics/      ← subject areas
    comparisons/ ← side-by-side analyses
    queries/     ← saved answers to important questions
```

---

## Quick Start: Claude Code

Claude Code reads `CLAUDE.md` which points to `wiki/SCHEMA.md`, so it already knows how to operate
the wiki.

**1. Drop a source document**
```bash
cp ~/Downloads/paper.pdf wiki/sources/
# or save any markdown/text file there
```

**2. Ingest it**
```
ingest wiki/sources/paper.pdf
```

Claude Code will read the document, create/update wiki pages, update `index.md`, and append to
`log.md`.

**3. Ask questions**
```
what does the wiki say about transformer attention mechanisms?
```

**4. Run a health check**
```
lint the wiki
```

---

## Quick Start: Zenii Agent

Zenii's built-in agent already has `FileReadTool`, `FileWriteTool`, `FileListTool`, and
`FileSearchTool` — no new routes or configuration needed.

**Via curl (REST)**
```bash
# Start a session
SESSION=$(curl -s -X POST http://localhost:18981/sessions \
  -H "Authorization: Bearer $ZENII_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"wiki session"}' | jq -r '.id')

# Send an ingest command
curl -X POST http://localhost:18981/sessions/$SESSION/messages \
  -H "Authorization: Bearer $ZENII_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Read wiki/SCHEMA.md then ingest wiki/sources/my-article.md",
    "role": "user"
  }'
```

**Via WebSocket (streaming)**
```javascript
const ws = new WebSocket('ws://localhost:18981/ws');
ws.send(JSON.stringify({
  type: "chat",
  session_id: "...",
  content: "Read wiki/SCHEMA.md then ingest wiki/sources/my-article.md"
}));
```

---

## Ingesting via the Web UI

Open **Knowledge Wiki** in the sidebar, click **Ingest**, then drag-and-drop or paste a file. The
gateway writes it to `wiki/pages/topics/` and returns the slug.

### Page Title Resolution

When a document has no YAML frontmatter `title:` field, the title is derived in order:

1. **Frontmatter** — `title: "My Title"` in the `---` block (highest priority)
2. **First `# Heading`** — the first `# ` line in the document body
3. **Filename** — original filename without extension, preserving casing
   (e.g. `GitHub Stars Growth Tips.md` → `"GitHub Stars Growth Tips"`)
4. **Slug** — humanized slug as last resort (`my-doc-slug` → `"My Doc Slug"`)

To set the page type, tags, or other metadata, include a YAML frontmatter block:

```markdown
---
title: "My Page"
type: concept        # concept | entity | topic | comparison | query
tags: [rust, async]
updated: 2026-04-09
---

Content here.
```

## Supported Source Formats

Anything the LLM agent can read:
- **Markdown** (`.md`) — best format, no conversion needed
- **Plain text** (`.txt`)
- **Code files** (`.rs`, `.ts`, etc.) — useful for technical wikis
- **PDF** (`.pdf`) — Claude Code can read these natively
- **HTML** — paste as `.html` or convert to markdown first

Tip: [Obsidian Web Clipper](https://obsidian.md/clipper) converts web articles to clean markdown.

---

## Example Session

```
You: ingest wiki/sources/attention-is-all-you-need.md

Agent: Key takeaways from "Attention Is All You Need":
- Introduces the Transformer architecture (encoder-decoder with self-attention)
- Eliminates recurrence entirely — enables parallelization
- Multi-head attention allows attending to multiple positions simultaneously
- Positional encodings substitute for recurrence to track sequence order
- Achieves SOTA on WMT 2014 EN-DE and EN-FR translation

Creating pages:
- pages/concepts/transformer-architecture.md (new)
- pages/concepts/self-attention.md (new)
- pages/concepts/multi-head-attention.md (new)
- pages/entities/vaswani-et-al-2017.md (new)
- pages/topics/sequence-to-sequence.md (updated)

Updated index.md (5 new entries).
Appended to log.md.

You: what does the wiki say about positional encodings?

Agent: Based on wiki/pages/concepts/transformer-architecture.md:
Positional encodings are fixed sine/cosine vectors added to token embeddings to inject
sequence position information, since the Transformer has no recurrence or convolution.
[source: attention-is-all-you-need.md]

See also: [[self-attention]], [[multi-head-attention]]
```

---

## Tips

- **Commit after each ingest**: `git add wiki/ && git commit -m "wiki: ingest <source-name>"`
- **Obsidian graph view**: open the `wiki/` folder in Obsidian to visualize page relationships
- **Dataview plugin**: query frontmatter metadata across pages with SQL-like syntax
- **Keep sources small**: split large documents into focused chunks for better page granularity
- **Evolve the schema**: if `wiki/SCHEMA.md` conventions feel awkward, ask the agent to suggest
  improvements — the schema is designed to co-evolve with your domain

---

## LLM Compatibility

The wiki works with any LLM that has file read/write tools:

| Interface | Works? | Notes |
|-----------|--------|-------|
| Claude Code | Yes | Native — reads CLAUDE.md → SCHEMA.md automatically |
| Zenii agent | Yes | Uses existing FileReadTool/FileWriteTool, no config needed |
| Codex CLI | Yes | Point it at `wiki/SCHEMA.md` as the instruction file |
| Any agent with filesystem tools | Yes | Tell it to read `wiki/SCHEMA.md` first |

No embeddings, vector databases, or special infrastructure required.

## Summary

<!-- 2–5 bullet points describing what this PR does -->
-
-

## Change Type

- [ ] Bug fix (non-breaking, fixes an issue)
- [ ] New feature (non-breaking, adds functionality)
- [ ] Refactor (non-breaking, code quality improvement)
- [ ] Documentation update
- [ ] Security fix
- [ ] Chore / dependency update
- [ ] Breaking change (requires migration guide)

## Linked Issue

Closes #<!-- issue number -->

## Validation Evidence

<!-- Commands you ran and their output confirming the change works -->

```
# Rust
cargo fmt --check  ✅
cargo clippy -- -D warnings  ✅
cargo test --lib  ✅ (N tests passing)

# Frontend
bun run check  ✅
bun run test   ✅ (N tests passing)
```

## Security Impact

- [ ] This PR touches security-sensitive code (security/, agent/, credentials)
- [ ] New IPC commands are added (verify permissions in capabilities/)
- [ ] User-facing input is sanitised / validated
- [ ] No secrets, API keys, or credentials are included in this diff

## Compatibility & Migration

<!-- Describe any breaking changes and the migration path, or write "N/A" -->

## Rollback Plan

<!-- How would we revert this if it causes issues in production? -->
<!-- e.g. "Revert commit X; no DB migrations to undo" -->

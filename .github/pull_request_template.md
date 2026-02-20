## Summary

<!-- What changed and why? Use bullet points. -->
-
-

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update
- [ ] Refactor (no functional change)
- [ ] Test improvement
- [ ] Security fix

## Validation Evidence

<!-- Required: show proof this works. Paste command output or test results. -->

```
# Command run:

# Output:
```

## Security Impact

- [ ] No security impact
- [ ] Changes security policy (please describe below)
- [ ] Adds new permissions or capabilities
- [ ] Updates dependencies with known CVEs (list them below)

<!-- If any boxes above are checked (except the first), explain: -->

## Rollback Plan

<!-- How do we revert this if it causes a production issue? -->

## Checklist

- [ ] `cd src-tauri && cargo test --lib` passes (420+ tests)
- [ ] `bunx ultracite check` passes (no lint errors)
- [ ] No new `## TODO` or `## MOCK` markers without tracking in `todo.md`
- [ ] Documentation updated if this changes behavior
- [ ] Commit messages follow conventional commit format

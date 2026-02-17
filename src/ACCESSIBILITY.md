# Accessibility Standards

This document outlines the accessibility standards and guidelines for the application. All components and features must adhere to these standards to ensure the application is usable by everyone, including users with disabilities.

## Table of Contents

1. [Core Principles](#core-principles)
2. [Component Standards](#component-standards)
3. [Color Usage Guidelines](#color-usage-guidelines)
4. [Form Requirements](#form-requirements)
5. [ARIA Attributes](#aria-attributes)
6. [Testing Checklist](#testing-checklist)
7. [Common Patterns](#common-patterns)

---

## Core Principles

The application follows the **WCAG 2.1 AA** standard with a goal of reaching AAA where feasible. Our four core principles are:

1. **Perceivable** - Information must be presentable in ways users can perceive
2. **Operable** - Interface components must be operable by all users
3. **Understandable** - Information and operation must be understandable
4. **Robust** - Content must be robust enough to work with assistive technologies

### Target Standards

- **WCAG Level**: AA (2.1)
- **Keyboard Accessibility**: 100% of functionality
- **Screen Reader Support**: NVDA, JAWS, VoiceOver, TalkBack
- **Color Contrast**: 4.5:1 for text, 3:1 for large text
- **Focus Indicators**: Visible on all interactive elements

---

## Component Standards

### Buttons

**Standard Button**

```tsx
// ✅ Correct - Text button
<Button onClick={handleClick}>Save Changes</Button>
```

**Icon-Only Button** (Must have aria-label)

```tsx
// ✅ Correct - Icon button with aria-label
<Button
  size="icon"
  aria-label="Close dialog"
  onClick={handleClose}
>
  <X className="h-4 w-4" />
</Button>

// ❌ Incorrect - Icon button without aria-label
<Button size="icon" onClick={handleClose}>
  <X className="h-4 w-4" />
</Button>
```

**Button with Decorative Icon**

```tsx
// ✅ Correct - Icon marked as decorative
<Button onClick={handleClick}>
  <Search aria-hidden="true" className="h-4 w-4 mr-2" />
  <span>Search</span>
</Button>
```

**Requirements:**

- ✅ All buttons are keyboard accessible (`Tab`, `Enter`, `Space`)
- ✅ Icon-only buttons MUST have `aria-label`
- ✅ Decorative icons marked with `aria-hidden="true"`
- ✅ Visible focus indicator (3:1 contrast ratio)
- ✅ Disabled buttons not focusable and marked with `disabled` attribute

---

### Tabs

**Location:** `src/components/ui/tabs.tsx`

```tsx
// ✅ Correct - Proper tab ARIA implementation
<Tabs defaultValue="overview">
  <TabsList role="tablist" aria-label="Workspace main tabs">
    <TabsTrigger
      value="overview"
      role="tab"
      aria-selected={isSelected}
      tabIndex={isSelected ? 0 : -1}
    >
      Overview
    </TabsTrigger>
  </TabsList>
  <TabsContent value="overview" role="tabpanel" tabIndex={0}>
    {/* Content */}
  </TabsContent>
</Tabs>
```

**Requirements:**

- ✅ `role="tablist"` on container
- ✅ `role="tab"` on each tab trigger
- ✅ `role="tabpanel"` on each tab panel
- ✅ `aria-selected="true"` on active tab
- ✅ `aria-controls` pointing to associated panel
- ✅ `aria-labelledby` on panel pointing to tab
- ✅ Arrow key navigation implemented
- ✅ `tabIndex` managed correctly (active tab: 0, inactive: -1)

---

### Trees (Hierarchical Navigation)

**Location:** `src/components/SchemaTree.tsx`, `src/components/SchemaTreeNode.tsx`

```tsx
// ✅ Correct - Tree ARIA implementation
<div role="tree" aria-label="Database schema">
  <div
    role="treeitem"
    aria-expanded={isExpanded}
    aria-selected={isSelected}
    tabIndex={isSelected ? 0 : -1}
  >
    {/* Node content */}
  </div>
</div>
```

**Requirements:**

- ✅ `role="tree"` on container
- ✅ `role="treeitem"` on each item
- ✅ `aria-expanded="true/false"` on expandable items
- ✅ `aria-selected="true"` on selected item
- ✅ Arrow key navigation (Up/Down/Left/Right)
- ✅ `Enter` or `Space` to select item
- ✅ Focus indicator visible on each item

---

### Dialogs (Modals)

**Location:** `src/components/ui/dialog.tsx`

```tsx
// ✅ Correct - Dialog ARIA implementation
<Dialog open={isOpen} onOpenChange={setIsOpen}>
  <DialogContent
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId}
    aria-describedby={descriptionId}
  >
    <DialogTitle id={titleId}>Settings</DialogTitle>
    <DialogDescription id={descriptionId}>
      Configure your workspace preferences
    </DialogDescription>
    {/* Content */}
  </DialogContent>
</Dialog>
```

**Requirements:**

- ✅ `role="dialog"` on dialog element
- ✅ `aria-modal="true"` when open
- ✅ `aria-labelledby` pointing to title
- ✅ `aria-describedby` pointing to description (if present)
- ✅ Focus trap within dialog
- ✅ `Escape` key closes dialog
- ✅ Focus returns to trigger after close
- ✅ First focusable element receives focus on open

---

### Forms

```tsx
// ✅ Correct - Form with proper labels
<form onSubmit={handleSubmit}>
  <label htmlFor="workspace-name">
    Workspace Name
    <span className="text-destructive" aria-label="required">
      *
    </span>
  </label>
  <Input
    id="workspace-name"
    name="workspaceName"
    required
    aria-invalid={errors.workspaceName ? "true" : "false"}
    aria-describedby={
      errors.workspaceName
        ? "workspace-name-error"
        : "workspace-name-description"
    }
  />
  <p id="workspace-name-description" className="text-sm text-muted-foreground">
    Enter a name for your workspace
  </p>
  {errors.workspaceName && (
    <p
      id="workspace-name-error"
      className="text-sm text-destructive"
      role="alert"
    >
      {errors.workspaceName}
    </p>
  )}
</form>
```

**Requirements:**

- ✅ All inputs have associated labels (using `htmlFor` or `aria-label`)
- ✅ Required fields marked programmatically (`required` attribute)
- ✅ Validation errors associated with inputs (`aria-describedby`, `role="alert"`)
- ✅ Helpful descriptions associated with inputs
- ✅ Form can be submitted without mouse
- ✅ Error messages are announced to screen readers

---

### Loading States

**Location:** `src/components/ui/loading-state.tsx`

```tsx
// ✅ Correct - Accessible loading state
<LoadingState message="Loading table details..." size="lg" className="my-4" />
```

**Implementation:**

```tsx
// Component must include:
<div role="status" aria-live="polite" aria-busy="true">
  {/* Loading indicator and message */}
</div>
```

**Requirements:**

- ✅ `role="status"` to announce as status message
- ✅ `aria-live="polite"` to announce when user is idle
- ✅ `aria-busy="true"` while loading
- ✅ Clear, descriptive message
- ✅ Not used for critical errors (use `role="alert"` instead)

---

### Error States

**Location:** `src/components/ui/error-state.tsx`

```tsx
// ✅ Correct - Accessible error state
<ErrorState
  title="Failed to Generate Overview"
  message="Unable to connect to database. Please check your connection."
  onRetry={handleRetry}
  retryLabel="Try Again"
  className="my-4"
/>
```

**Implementation:**

```tsx
// Component must include:
<div role="alert" aria-live="assertive">
  {/* Error icon, title, message, and retry button */}
</div>
```

**Requirements:**

- ✅ `role="alert"` to immediately announce
- ✅ `aria-live="assertive"` for immediate announcement
- ✅ Clear error title and description
- ✅ Retry action available if applicable
- ✅ Error is actionable (provides next steps)

---

### Empty States

**Location:** `src/components/ui/empty-state.tsx`

```tsx
// ✅ Correct - Accessible empty state
<EmptyState
  icon={Database}
  title="No tables found"
  description="Create a table to get started with your database"
  action={{
    label: "Create Table",
    onClick: handleCreate,
  }}
/>
```

**Implementation:**

```tsx
// Component must include:
<div role="status" aria-live="polite">
  {/* Icon, title, description, and action button */}
</div>
```

**Requirements:**

- ✅ `role="status"` to announce state
- ✅ Clear, helpful title and description
- ✅ Action to resolve empty state
- ✅ Icon marked with `aria-hidden="true"` if decorative

---

### Search Inputs

```tsx
// ✅ Correct - Accessible search input
<div className="relative">
  <Search
    className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
    aria-hidden="true"
  />
  <Input
    type="search"
    placeholder="Search tables..."
    aria-label="Search tables and columns"
    className="pl-9"
  />
  {value && (
    <button
      aria-label="Clear search"
      onClick={handleClear}
      className="absolute right-3 top-1/2 -translate-y-1/2"
    >
      <X className="h-4 w-4" aria-hidden="true" />
    </button>
  )}
</div>
```

**Requirements:**

- ✅ `aria-label` describing search purpose
- ✅ `type="search"` for search inputs
- ✅ Decorative icons marked with `aria-hidden="true"`
- ✅ Clear button has `aria-label`
- ✅ Result count announced to screen readers

---

## Color Usage Guidelines

### WCAG AA Compliance

All colors must meet the following contrast ratios:

- **Normal text** (< 18pt or < 14pt bold): 4.5:1
- **Large text** (≥ 18pt or ≥ 14pt bold): 3:1
- **UI components**: 3:1 against adjacent colors
- **Focus indicators**: 3:1 against background

### Semantic Color Tokens

Use semantic color tokens from `src/styles/design-tokens.css` instead of arbitrary Tailwind colors:

```css
/* Status colors */
.status-success     /* Green for success states */
.status-warning     /* Amber/yellow for warnings */
.status-error       /* Red for errors */
.status-info        /* Blue for informational messages */

/* Data type colors */
.type-primary       /* Amber for primary keys */
.type-foreign       /* Purple for foreign keys */
.type-reference     /* Cyan for references */
.type-code          /* Blue for code elements */

/* Accent colors */
.accent-primary     /* Primary brand color */
.accent-secondary   /* Secondary brand color */
```

### Correct Usage

```tsx
// ✅ Correct - Using semantic tokens
<span className="type-primary">Primary Key</span>
<span className="status-error">Error occurred</span>
<Badge className="status-success">Connected</Badge>

// ❌ Incorrect - Low contrast colors
<span className="text-cyan-400">Username</span>
<span className="text-amber-500">Required</span>
<Badge className="bg-purple-500">Status</Badge>
```

### Dark Mode Considerations

All semantic tokens automatically adapt to dark mode with OKLCH color values. Ensure:

- ✅ Test color contrast in both light and dark modes
- ✅ Use semantic tokens, not hardcoded colors
- ✅ Verify custom CSS works in both themes

### Don't Rely on Color Alone

Always provide additional indicators beyond color:

```tsx
// ✅ Correct - Color + icon + text
<div className="flex items-center gap-2">
  <CheckCircle className="h-4 w-4 text-green-700" aria-hidden="true" />
  <span className="text-green-700">Connected</span>
</div>

// ❌ Incorrect - Color only
<span className="text-green-600">Connected</span>
```

---

## Form Requirements

### Labels

All form inputs must have labels:

**Explicit Label** (Preferred)

```tsx
<label htmlFor="email">Email Address</label>
<Input id="email" type="email" />
```

**ARIA Label** (When visible label not possible)

```tsx
<Input
  aria-label="Search tables and columns"
  type="search"
  placeholder="Search..."
/>
```

**aria-labelledby** (When label is separate element)

```tsx
<div>
  <span id="search-label">Search</span>
  <Input aria-labelledby="search-label" type="search" />
</div>
```

### Required Fields

```tsx
// ✅ Correct - Programmatic and visual indication
<label htmlFor="name">
  Name
  <span className="text-destructive" aria-label="required">
    *
  </span>
</label>
<Input id="name" required />
```

### Error Handling

```tsx
// ✅ Correct - Associated error message
<Input
  id="email"
  aria-invalid={hasError ? "true" : "false"}
  aria-describedby={hasError ? "email-error" : undefined}
/>;
{
  hasError && (
    <p id="email-error" className="text-destructive" role="alert">
      Please enter a valid email address
    </p>
  );
}
```

### Instructions

```tsx
// ✅ Correct - Helpful instructions
<label htmlFor="password">
  Password
</label>
<Input
  id="password"
  type="password"
  aria-describedby="password-instructions"
/>
<p id="password-instructions" className="text-sm text-muted-foreground">
  Must be at least 8 characters with letters and numbers
</p>
```

---

## ARIA Attributes

### When to Use ARIA

**Use ARIA when:**

- HTML semantics are insufficient (complex widgets like trees, tabs)
- Providing additional context (labels, descriptions)
- Managing dynamic content updates (live regions)
- Indicating states (expanded, selected, invalid)

**Don't use ARIA when:**

- Native HTML element works (use `<button>` not `<div role="button">`)
- Can use semantic HTML instead
- Would duplicate native semantics

### Common ARIA Attributes

| Attribute            | Usage                                       | Example                           |
| -------------------- | ------------------------------------------- | --------------------------------- |
| `aria-label`         | Hidden label for screen readers             | `aria-label="Close dialog"`       |
| `aria-labelledby`    | Reference to label element                  | `aria-labelledby="title-id"`      |
| `aria-describedby`   | Reference to description                    | `aria-describedby="help-text"`    |
| `aria-hidden="true"` | Hide decorative content from screen readers | Decorative icons                  |
| `aria-expanded`      | State of expandable element                 | Tree nodes, accordions            |
| `aria-selected`      | Selected state in tabs/lists                | Tab triggers, list items          |
| `aria-invalid`       | Form validation state                       | `aria-invalid="true"`             |
| `aria-live`          | Announce dynamic content                    | `aria-live="polite"`              |
| `role`               | Element type when HTML insufficient         | `role="dialog"`, `role="tablist"` |

### Live Regions

Announce dynamic content changes:

```tsx
// ✅ Correct - Live region for non-critical updates
<LiveRegion message="Changes saved" />

// Implementation
<div
  role="status"
  aria-live="polite"
  aria-atomic="true"
>
  {message}
</div>

// For critical updates (errors)
<div
  role="alert"
  aria-live="assertive"
  aria-atomic="true"
>
  {errorMessage}
</div>
```

---

## Testing Checklist

### Pre-Commit Checklist

Before committing any component, verify:

**Basic Accessibility**

- [ ] All interactive elements are keyboard accessible
- [ ] Focus indicators are visible
- [ ] Color contrast meets WCAG AA
- [ ] Images have alt text or are decorative
- [ ] Forms have proper labels

**ARIA Attributes**

- [ ] Icon-only buttons have aria-label
- [ ] Decorative icons have aria-hidden="true"
- [ ] Dialogs have proper ARIA roles
- [ ] Tabs have proper ARIA attributes
- [ ] Form errors have role="alert"

**Screen Reader**

- [ ] Test with NVDA (Windows) or VoiceOver (Mac)
- [ ] All actions are announced clearly
- [ ] Navigation is logical
- [ ] Dynamic content updates are announced

### Regression Testing

Run these tests regularly:

```bash
# Type check
bun run check

# Lint
bun run lint

# Automated accessibility tests (if implemented)
bun run test:a11y
```

### Browser Testing

Test accessibility in:

- [ ] Chrome/Edge (Windows, macOS, Linux)
- [ ] Firefox (Windows, macOS, Linux)
- [ ] Safari (macOS, iOS)

### Screen Reader Testing

Test with:

- [ ] NVDA (Windows) - Free, open source
- [ ] JAWS (Windows) - Commercial, common in enterprise
- [ ] VoiceOver (macOS, iOS) - Built-in
- [ ] TalkBack (Android) - Built-in

---

## Common Patterns

### Pattern 1: Toggle Switch

```tsx
<button
  role="switch"
  aria-checked={isEnabled}
  onClick={toggle}
  aria-label="Enable notifications"
>
  {isEnabled ? "On" : "Off"}
</button>
```

### Pattern 2: Expandable Content

```tsx
<button
  aria-expanded={isExpanded}
  aria-controls="content-id"
  onClick={toggle}
>
  Show Details
</button>
<div id="content-id" hidden={!isExpanded}>
  {/* Content */}
</div>
```

### Pattern 3: Dropdown Menu

```tsx
<button aria-haspopup="true" aria-expanded={isOpen} onClick={toggle}>
  Options
</button>;
{
  isOpen && (
    <ul role="menu" aria-label="Actions">
      <li role="menuitem">
        <button onClick={handleEdit}>Edit</button>
      </li>
      <li role="menuitem">
        <button onClick={handleDelete}>Delete</button>
      </li>
    </ul>
  );
}
```

### Pattern 4: Progress Indicator

```tsx
<div
  role="progressbar"
  aria-valuenow={progress}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-label="Loading progress"
>
  {progress}%
</div>
```

### Pattern 5: Modal Dialog

```tsx
<Dialog open={isOpen} onOpenChange={setIsOpen}>
  <DialogContent
    role="dialog"
    aria-modal="true"
    aria-labelledby="dialog-title"
    aria-describedby="dialog-description"
  >
    <h2 id="dialog-title">Confirm Deletion</h2>
    <p id="dialog-description">This action cannot be undone.</p>
    <div className="flex gap-2">
      <Button onClick={confirm}>Delete</Button>
      <Button variant="ghost" onClick={cancel}>
        Cancel
      </Button>
    </div>
  </DialogContent>
</Dialog>
```

---

## Resources

### Documentation

- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [ARIA Authoring Practices Guide (APG)](https://www.w3.org/WAI/ARIA/apg/)
- [WebAIM Accessibility Checklist](https://webaim.org/standards/wcag/checklist)

### Testing Tools

- [axe DevTools](https://www.deque.com/axe/devtools/) - Browser extension
- [WAVE](https://wave.webaim.org/) - Browser extension
- [Lighthouse](https://developers.google.com/web/tools/lighthouse) - Built into Chrome
- [playwright-axe](https://www.npmjs.com/package/playwright-axe) - Automated testing

### Screen Readers

- [NVDA (Windows)](https://www.nvaccess.org/) - Free, open source
- [VoiceOver (Mac/iOS)](https://www.apple.com/accessibility/voiceover/) - Built-in
- [JAWS (Windows)](https://www.freedomscientific.com/products/software/jaws/) - Commercial

---

## Getting Help

When you encounter accessibility issues:

1. **Check this document** - Review relevant standards and patterns
2. **Test with screen reader** - Verify how it's announced
3. **Use testing tools** - Run axe DevTools or WAVE
4. **Consult resources** - Check ARIA APG for widget patterns
5. **Ask for help** - Post in team chat with specific issue

### Report Accessibility Issues

When reporting, include:

- **Component/Feature** - What part of the app
- **Issue Description** - Clear problem statement
- **Steps to Reproduce** - How to encounter the issue
- **Expected Behavior** - What should happen
- **Actual Behavior** - What actually happens
- **Testing Environment** - Browser, OS, screen reader

---

## Continuous Improvement

Accessibility is not a one-time task but an ongoing process. We commit to:

- ✅ Testing accessibility in every PR
- ✅ Including accessibility in acceptance criteria
- ✅ Training team members on accessibility
- ✅ Regular accessibility audits
- ✅ Listening to feedback from disabled users
- ✅ Staying updated on WCAG guidelines

**Remember:** Accessibility benefits everyone, not just disabled users. Good accessibility practices improve usability for all users.

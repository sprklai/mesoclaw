# Keyboard Navigation Testing Guide

This guide provides comprehensive testing checklists and scenarios for verifying keyboard navigation accessibility in the aiboilerplate.

## Overview

Keyboard accessibility ensures that users can navigate and interact with the entire application using only a keyboard. This is essential for users with motor disabilities, screen reader users, and power users.

## General Testing Approach

### Basic Setup

1. **Unplug your mouse** or disable trackpad
2. Use only keyboard keys: `Tab`, `Shift+Tab`, `Enter`, `Space`, `Arrow keys`, `Escape`
3. Test on both Windows and macOS if possible (key differences: `Ctrl` vs `Cmd`)

### Visual Focus Indicators

- All interactive elements must show a visible focus indicator
- Focus indicator must have 3:1 contrast ratio against background
- Focus ring should be clearly visible around the focused element

### Logical Tab Order

- Tab order should follow the visual layout (left-to-right, top-to-bottom)
- Focus should not jump unexpectedly
- Skip links should be provided for main content navigation

---

## Component-Specific Testing

### 1. Workspace Tabs

**Location:** `src/routes/workspace.$id.tsx`

**Keys to Test:**

- `Tab` - Navigate to tab list
- `ArrowRight` / `ArrowLeft` - Move between tabs
- `Enter` or `Space` - Activate tab
- `Shift+Tab` - Navigate backward through tabs

**Expected Behavior:**

- ✅ Tabs have visible focus indicator
- ✅ Arrow keys cycle through tabs when tab list has focus
- ✅ Activated tab shows `aria-selected="true"`
- ✅ Tab panel appears and receives focus
- ✅ Focus stays within tab panel until `Tab` exits
- ✅ Tab panels have `role="tabpanel"` with proper ARIA relationships

**Test Scenarios:**

1. Navigate to workspace page
2. Press `Tab` until focus reaches tab list
3. Use `ArrowRight` to move through all 6 tabs (Overview, Schema, ERD, Chat, Insights, Metadata)
4. Press `Enter` on "ERD" tab
5. Verify ERD panel is displayed
6. Press `Tab` to move into ERD panel content
7. Use `Shift+Tab` to return to tab list
8. Verify tab focus is maintained

---

### 2. Schema Tree

**Location:** `src/components/SchemaTree.tsx`, `src/components/SchemaTreeNode.tsx`

**Keys to Test:**

- `Tab` - Navigate to tree container
- `ArrowUp` / `ArrowDown` - Move between tree items
- `ArrowRight` - Expand collapsed node / Move to first child
- `ArrowLeft` - Collapse expanded node / Move to parent
- `Enter` or `Space` - Select table/node
- `Home` - Move to first tree item
- `End` - Move to last tree item

**Expected Behavior:**

- ✅ Tree container has `role="tree"` and `aria-label`
- ✅ Tree items have `role="treeitem"`
- ✅ Expandable nodes show `aria-expanded="true/false"`
- ✅ Selected nodes show `aria-selected="true"`
- ✅ Arrow keys navigate between items
- ✅ `Enter` triggers selection action (shows table details)
- ✅ Focus indicator visible on each item

**Test Scenarios:**

1. Navigate to Schema tab
2. Press `Tab` to reach schema tree
3. Use `ArrowDown` to move through tables
4. Press `ArrowRight` on a table to expand it
5. Use `ArrowDown` to navigate to columns
6. Press `Enter` on a column to select it
7. Press `ArrowLeft` to collapse table
8. Verify table details panel updates on selection

---

### 3. Dialogs (Modals)

**Locations:** Multiple components using `src/components/ui/dialog.tsx`

**Keys to Test:**

- `Escape` - Close dialog
- `Tab` - Move focus within dialog
- `Shift+Tab` - Move focus backward within dialog
- `Enter` - Activate focused button

**Expected Behavior:**

- ✅ Dialog has `role="dialog"` and `aria-modal="true"`
- ✅ Focus moves into dialog when opened
- ✅ Focus is trapped within dialog (tab cycles inside)
- ✅ `Escape` closes dialog and returns focus to trigger
- ✅ Focus returns to trigger element after close
- ✅ First focusable element receives focus when opened
- ✅ Dialog has `aria-labelledby` pointing to title
- ✅ Dialog has `aria-describedby` pointing to description

**Test Scenarios:**

1. Open any dialog (e.g., settings, help dialog)
2. Verify focus moves into dialog
3. Press `Tab` to cycle through all interactive elements
4. Press `Shift+Tab` to go backwards
5. Press `Escape` to close dialog
6. Verify focus returns to button that opened dialog

---

### 4. Buttons and Actions

**Locations:** Throughout the application

**Keys to Test:**

- `Tab` - Navigate to button
- `Enter` or `Space` - Activate button
- `Shift+Tab` - Navigate backward

**Expected Behavior:**

- ✅ All buttons are focusable
- ✅ Icon-only buttons have `aria-label` describing action
- ✅ Decorative icons have `aria-hidden="true"`
- ✅ Disabled buttons have `disabled` attribute and are not focusable
- ✅ Button focus indicator clearly visible
- ✅ `Enter` and `Space` both activate buttons

**Test Scenarios:**

1. Tab to "Connect Database" button
2. Press `Enter` to activate
3. Tab to "Refresh" button (icon-only)
4. Verify screen reader announces "Refresh" (aria-label)
5. Tab to disabled button
6. Verify it's skipped in tab order

---

### 5. Forms and Inputs

**Locations:** Connection form, search inputs, settings

**Keys to Test:**

- `Tab` - Navigate between fields
- `Shift+Tab` - Navigate backward
- `Enter` - Submit form (when on submit button)
- `Space` - Toggle checkboxes
- `ArrowUp`/`ArrowDown` - Select from dropdown

**Expected Behavior:**

- ✅ All inputs have visible labels (using `htmlFor` or `aria-label`)
- ✅ Required fields are marked programmatically
- ✅ Error messages are associated with inputs (`aria-describedby`)
- ✅ Focus moves to next field on `Tab`
- ✅ Form can be submitted with keyboard only
- ✅ Validation errors are announced to screen readers

**Test Scenarios:**

1. Navigate to connection form
2. Press `Tab` to move through each field
3. Verify each field has visible focus
4. Enter data in each field
5. Tab to submit button
6. Press `Enter` to submit
7. Verify validation errors are announced if present

---

### 6. Search and Filtering

**Location:** `src/components/SchemaSearch.tsx`, model selectors

**Keys to Test:**

- `Tab` - Navigate to search input
- Type text - Filter results
- `ArrowDown` / `ArrowUp` - Navigate filtered results
- `Enter` - Select result
- `Escape` - Clear search or close dropdown

**Expected Behavior:**

- ✅ Search input has `aria-label` describing purpose
- ✅ Filtered results are announced to screen readers
- ✅ Result count is announced (e.g., "5 results found")
- ✅ Arrow keys navigate through results
- ✅ `Enter` selects highlighted result
- ✅ `Escape` clears input and resets results

**Test Scenarios:**

1. Navigate to schema search
2. Type table name to filter
3. Use `ArrowDown` to navigate results
4. Press `Enter` on a result
5. Verify table is selected in schema tree
6. Press `Escape` to clear search

---

### 7. ERD Graph (ReactFlow)

**Location:** `src/components/ERDPanel.tsx`

**Keys to Test:**

- `Tab` - Navigate to graph canvas
- `Arrow keys` - Pan the graph (if implemented)
- `+` / `-` - Zoom in/out (if implemented)
- `R` / `Space` - Reset view (if implemented)

**Expected Behavior:**

- ✅ Graph canvas is focusable
- ✅ Keyboard alternatives provided for drag/drop actions
- ✅ Selected nodes show visible focus indicator
- ✅ Keyboard can navigate between nodes
- ✅ `Escape` exits node selection mode

**Test Scenarios:**

1. Navigate to ERD tab
2. Press `Tab` to reach graph canvas
3. Test keyboard shortcuts for pan/zoom
4. Verify nodes can be selected with keyboard
5. Verify focus indicator is visible on selected nodes

---

### 8. Data Tables

**Location:** `src/components/TableDataView.tsx`

**Keys to Test:**

- `Tab` - Navigate through table cells
- `ArrowUp` / `ArrowDown` - Move between rows
- `ArrowLeft` / `ArrowRight` - Move between cells
- `Ctrl+Home` - Jump to first cell
- `Ctrl+End` - Jump to last cell

**Expected Behavior:**

- ✅ Table has proper semantic HTML (`<table>`, `<thead>`, etc.)
- ✅ Headers have `scope="col"` or `scope="row"`
- ✅ Cells can be navigated with arrow keys
- ✅ Focus indicator visible on selected cell
- ✅ Sortable columns indicate sort direction

**Test Scenarios:**

1. Navigate to table data view
2. Use `Tab` to enter table
3. Use arrow keys to navigate cells
4. Verify focus moves logically through table
5. Test column sorting with keyboard

---

## Screen Reader Testing

### NVDA (Windows)

1. Download NVDA from https://www.nvaccess.org/
2. Start NVDA with `Ctrl+Alt+N`
3. Use arrow keys to navigate
4. Test all components above
5. Verify ARIA labels are announced correctly

### VoiceOver (macOS)

1. Enable VoiceOver: `Cmd+F5`
2. Use `VO+Left/Right` arrows to navigate
3. Use `VO+Space` to activate
4. Test all components above
5. Verify labels and roles are announced

### Common Screen Reader Issues to Check

- ✅ Icons without aria-label are not announced (or announced as "unlabeled")
- ✅ Button actions are clearly described
- ✅ Form fields have associated labels
- ✅ Error messages are announced
- ✅ Dynamic content updates are announced (aria-live regions)
- ✅ Dialog titles and descriptions are announced
- ✅ Tab changes are announced
- ✅ Expand/collapse states are announced

---

## Automated Testing Tools

### axe DevTools

```bash
# Install browser extension
# Chrome: https://chrome.google.com/webstore/detail/axe-devtools-web-accessib/lhdoppojpmngadmnindnejefpokejbdd
# Firefox: https://addons.mozilla.org/en-US/firefox/addon/axe-devtools/
```

### WAVE Browser Extension

```bash
# Chrome: https://chrome.google.com/webstore/detail/wave-evaluation-tool/jbbplnpkjmmeebjpijfedlgcdilocofh
# Firefox: https://addons.mozilla.org/en-US/firefox/addon/wave-accessibility-tool/
```

### Playwright Accessibility Tests

```typescript
// Example automated test
test("keyboard navigation works", async ({ page }) => {
  await page.goto("/workspace/test-id");
  await page.keyboard.press("Tab");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  // Assert expected state
});
```

---

## Quick Checklist

Use this quick checklist for rapid testing:

### Must Have (Critical)

- [ ] All interactive elements are keyboard accessible
- [ ] Tab order is logical
- [ ] Focus indicators are visible on all elements
- [ ] `Escape` closes all modals/dialogs
- [ ] `Enter` and `Space` activate buttons
- [ ] Icon-only buttons have aria-labels
- [ ] Forms can be submitted without mouse
- [ ] No keyboard traps (can always move forward)

### Should Have (Important)

- [ ] Skip links for main content
- [ ] Arrow key navigation for complex widgets
- [ ] Focus returns to trigger after dialog close
- [ ] Error messages are announced
- [ ] Live regions announce dynamic content

### Nice to Have (Enhancement)

- [ ] Keyboard shortcuts for common actions
- [ ] Visual indication of keyboard mode
- [ ] Help documentation for keyboard users
- [ ] Custom focus styles matching brand

---

## Reporting Issues

When documenting keyboard navigation issues, include:

1. **Component Name** - Which component has the issue
2. **Key Pressed** - What key combination was used
3. **Expected Behavior** - What should happen
4. **Actual Behavior** - What actually happened
5. **Screen Reader** - If applicable, which screen reader and version
6. **Browser/OS** - Testing environment

Example:

```
Component: Schema Tree
Key Pressed: ArrowRight on expanded node
Expected: Focus moves to first child
Actual: Focus jumps to next sibling
Screen Reader: NVDA 2024.1
Browser/OS: Chrome 120 / Windows 11
```

---

## Additional Resources

- [WCAG 2.1 Guideline 2.1 - Keyboard Accessible](https://www.w3.org/WAI/WCAG21/Understanding/keyboard.html)
- [ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
- [WebAIM Keyboard Accessibility](https://webaim.org/techniques/keyboard/)
- [Inclusive Components - Keyboard Navigation](https://inclusive-components.design/keyboard-navigation/)

# Responsive Design System

A comprehensive, mobile-first responsive design system built with Tailwind CSS 4 utilities.

## Design Philosophy

- **Mobile-First**: Styles start with mobile and scale up
- **Progressive Enhancement**: Base experience works everywhere, enhanced on larger screens
- **Accessibility-First**: Touch targets, reduced motion, and high contrast support
- **Performance**: Minimal custom CSS, leveraging Tailwind's utility classes

## Breakpoints

Following Tailwind's default breakpoints:

| Breakpoint | Min Width | Target Devices                          |
| ---------- | --------- | --------------------------------------- |
| `sm`       | 640px     | Small tablets, large phones (landscape) |
| `md`       | 768px     | Tablets                                 |
| `lg`       | 1024px    | Small laptops, tablets (landscape)      |
| `xl`       | 1280px    | Desktops                                |
| `2xl`      | 1536px    | Large screens                           |

## Responsive Utilities

### Container Classes

```tsx
// Full-width responsive container
<div className="container-responsive">...</div>

// Narrow content (max 4xl)
<div className="container-narrow">...</div>

// Wide content (max 7xl)
<div className="container-wide">...</div>
```

### Grid Layouts

```tsx
// 2-column responsive grid
<div className="grid-responsive-2">
  <div>Item 1</div>
  <div>Item 2</div>
</div>

// 3-column responsive grid
<div className="grid-responsive-3">
  <div>Item 1</div>
  <div>Item 2</div>
  <div>Item 3</div>
</div>

// 4-column responsive grid
<div className="grid-responsive-4">
  <div>Item 1</div>
  <div>Item 2</div>
  <div>Item 3</div>
  <div>Item 4</div>
</div>
```

### Responsive Typography

```tsx
<h1 className="text-responsive-2xl">Responsive Heading</h1>
<p className="text-responsive-base">Responsive paragraph text</p>
<span className="text-responsive-sm">Small responsive text</span>
```

**Available sizes:**

- `text-responsive-xs` (xs → sm)
- `text-responsive-sm` (sm → base)
- `text-responsive-base` (base → lg → xl)
- `text-responsive-lg` (lg → xl → 2xl)
- `text-responsive-xl` (xl → 2xl → 3xl)
- `text-responsive-2xl` (2xl → 3xl → 4xl)

### Responsive Spacing

```tsx
// Padding
<div className="p-responsive">Responsive padding</div>
<div className="p-responsive-sm">Small responsive padding</div>
<div className="p-responsive-lg">Large responsive padding</div>
<div className="px-responsive">Horizontal responsive padding</div>
<div className="py-responsive">Vertical responsive padding</div>

// Gaps
<div className="grid gap-responsive">Responsive gaps</div>
<div className="grid gap-responsive-sm">Small responsive gaps</div>
<div className="grid gap-responsive-lg">Large responsive gaps</div>
```

### Responsive Layouts

```tsx
// Flex column on mobile, row on tablet+
<div className="flex-responsive-col">
  <div>Column 1</div>
  <div>Column 2</div>
</div>

// Flex column on mobile/tablet, row on desktop
<div className="flex-responsive-row">
  <div>Column 1</div>
  <div>Column 2</div>
</div>
```

### Responsive Widths

```tsx
// Full width on mobile, auto on desktop
<div className="w-responsive">...</div>

// Full width → half width
<div className="w-responsive-half">...</div>

// Full width → third width
<div className="w-responsive-third">...</div>

// Full width → two-thirds width
<div className="w-responsive-two-thirds">...</div>

// Full width → quarter width
<div className="w-responsive-quarter">...</div>
```

### Responsive Buttons

```tsx
<button className="btn-responsive">Default responsive button</button>
<button className="btn-responsive-sm">Small responsive button</button>
<button className="btn-responsive-lg">Large responsive button</button>
```

### Responsive Cards

```tsx
<div className="card-responsive">
  <h2>Card Title</h2>
  <p>Card content with responsive padding</p>
</div>
```

## Display Utilities

### Show/Hide by Breakpoint

```tsx
// Only visible on mobile
<div className="mobile-only">Mobile only content</div>

// Only visible on desktop
<div className="desktop-only">Desktop only content</div>

// Hide on mobile
<div className="hide-mobile">Hidden on mobile</div>

// Hide on tablet
<div className="hide-tablet">Hidden on tablet</div>

// Hide on desktop
<div className="hide-desktop">Hidden on desktop</div>
```

## Form Layouts

```tsx
// 2-column responsive form grid
<form className="form-grid-responsive">
  <input type="text" placeholder="Field 1" />
  <input type="text" placeholder="Field 2" />
</form>

// 3-column responsive form grid
<form className="form-grid-responsive-3">
  <input type="text" placeholder="Field 1" />
  <input type="text" placeholder="Field 2" />
  <input type="text" placeholder="Field 3" />
</form>

// Responsive input height
<input className="input-responsive" type="text" />
```

## Tables

```tsx
// Horizontal scroll wrapper for tables
<div className="table-responsive">
  <table>
    <thead>
      <tr>
        <th>Column 1</th>
        <th>Column 2</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>Data 1</td>
        <td>Data 2</td>
      </tr>
    </tbody>
  </table>
</div>
```

## Modals and Dialogs

```tsx
// Responsive modal container
<div className="modal-responsive modal-content-responsive">
  <h2>Dialog Title</h2>
  <p>Dialog content</p>
</div>
```

## Touch Targets

For better mobile usability:

```tsx
// Minimum 44x44px touch target (iOS/Android standard)
<button className="touch-target">Button</button>

// Slightly smaller (36x36px)
<button className="touch-target-sm">Small Button</button>
```

## Safe Areas

For mobile devices with notches/home indicators:

```tsx
// Apply safe area insets
<div className="safe-area-all">Content</div>
<div className="safe-area-top">Top padding only</div>
<div className="safe-area-bottom">Bottom padding only</div>
```

## Accessibility Features

### Reduced Motion

Respects user's motion preferences:

```css
/* Automatically applied - no class needed */
@media (prefers-reduced-motion: reduce) {
  /* Animations and transitions are reduced to minimum */
}
```

### High Contrast Mode

Enhances borders and outlines for high contrast mode:

```css
/* Automatically applied - no class needed */
@media (prefers-contrast: high) {
  /* Borders and outlines are enhanced */
}
```

### Touch-Optimized Interactions

Better behavior on touch devices:

```tsx
<div className="touch-optimized">Large touch targets on touch devices</div>
<div className="hover-effect">No hover transitions on touch devices</div>
```

## Print Styles

```tsx
// Hide element when printing
<div className="no-print">Won't print</div>

// Only show when printing
<div className="print-only">Only prints</div>

// Control page breaks
<div className="break-before">Break before this</div>
<div className="break-after">Break after this</div>
<div className="break-inside-avoid">Don't break inside</div>
```

## Scrollbars

Custom responsive scrollbars:

```tsx
<div className="scrollbar-responsive max-h-responsive">
  <p>Scrollable content with styled scrollbar</p>
</div>
```

## Icons and Badges

```tsx
// Responsive icon sizes
<Icon className="icon-responsive" />
<Icon className="icon-responsive-lg" />

// Responsive badges
<span className="badge-responsive">Badge</span>
```

## Tooltips

```tsx
// Responsive tooltip (wider on mobile)
<div className="tooltip-responsive">Tooltip content</div>
```

## Section Spacing

```tsx
// Consistent vertical spacing for sections
<section className="section-spacing">
  <h2>Section Title</h2>
  <p>Section content</p>
</section>
```

## Dark Mode Responsive Adjustments

Dark mode has specific responsive behaviors:

- Reduced shadow intensity
- Increased contrast on small screens
- Automatic color scheme switching

## Usage Examples

### Responsive Card Grid

```tsx
export function CardGrid() {
  return (
    <div className="container-responsive">
      <div className="grid-responsive-3">
        {items.map((item) => (
          <div key={item.id} className="card-responsive">
            <h3 className="text-responsive-lg">{item.title}</h3>
            <p className="text-responsive-sm text-muted-foreground">
              {item.description}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}
```

### Responsive Form

```tsx
export function ResponsiveForm() {
  return (
    <form className="container-narrow">
      <div className="form-grid-responsive">
        <div>
          <label>First Name</label>
          <input className="input-responsive" type="text" />
        </div>
        <div>
          <label>Last Name</label>
          <input className="input-responsive" type="text" />
        </div>
      </div>
      <button className="btn-responsive-lg mt-4">Submit</button>
    </form>
  );
}
```

### Responsive Header

```tsx
export function ResponsiveHeader() {
  return (
    <header className="container-responsive p-responsive">
      <div className="flex-responsive-col items-center justify-between gap-4">
        <h1 className="text-responsive-xl">Logo</h1>
        <nav className="flex-responsive-col gap-responsive">
          <a href="#" className="text-responsive-sm">
            Link 1
          </a>
          <a href="#" className="text-responsive-sm">
            Link 2
          </a>
        </nav>
      </div>
    </header>
  );
}
```

## Best Practices

1. **Start with mobile**: Write mobile styles first, add `sm:`, `md:`, `lg:` prefixes
2. **Use utility classes**: Prefer predefined responsive utilities over custom breakpoints
3. **Test touch targets**: Ensure buttons are at least 44x44px on mobile
4. **Check accessibility**: Test with screen readers and keyboard navigation
5. **Optimize images**: Use responsive images with `srcset` and `sizes`
6. **Consider landscape**: Test in both portrait and landscape orientations
7. **Test dark mode**: Verify responsive layouts work in both themes
8. **Print preview**: Check print styles for documentation pages

## Migration Guide

### Converting existing components:

**Before:**

```tsx
<div className="p-4 sm:p-6 lg:p-8">
  <h2 className="text-2xl sm:text-3xl lg:text-4xl">Title</h2>
</div>
```

**After:**

```tsx
<div className="p-responsive">
  <h2 className="text-responsive-2xl">Title</h2>
</div>
```

## Browser Support

- Modern browsers with CSS Grid and Flexbox support
- iOS Safari 12+
- Chrome/Edge 88+
- Firefox 85+
- Samsung Internet 14+

## Performance Notes

- All responsive utilities use Tailwind's JIT compiler
- No unused CSS in production builds
- Minimal runtime overhead
- Uses native CSS features (no JavaScript)

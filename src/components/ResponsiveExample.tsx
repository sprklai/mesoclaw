/**
 * ResponsiveExample - Demonstrates the responsive design system
 *
 * This component showcases various responsive utilities from the design system.
 * Use it as a reference for implementing responsive layouts in your components.
 */

import { Button } from "./ui/button";

export function ResponsiveExample() {
  return (
    <div className="container-responsive py-responsive">
      {/* Responsive typography */}
      <h1 className="text-responsive-2xl font-bold">Responsive Typography</h1>
      <p className="text-responsive-base text-muted-foreground">
        This text scales from base to lg to xl depending on screen size
      </p>

      {/* Responsive grid */}
      <div className="section-spacing">
        <h2 className="text-responsive-xl font-semibold mb-4">
          Responsive Grid
        </h2>
        <div className="grid-responsive-3">
          {[1, 2, 3, 4, 5, 6].map((item) => (
            <div key={item} className="card-responsive">
              <div className="p-responsive-sm">
                <h3 className="text-responsive-sm font-medium">Card {item}</h3>
              </div>
              <div className="p-responsive-sm">
                <p className="text-responsive-xs text-muted-foreground">
                  Responsive card with adaptive padding
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Responsive form */}
      <div className="section-spacing">
        <h2 className="text-responsive-xl font-semibold mb-4">
          Responsive Form
        </h2>
        <div className="card-responsive">
          <form className="form-grid-responsive">
            <div className="space-y-2">
              <label className="text-responsive-sm font-medium">
                First Name
              </label>
              <input
                type="text"
                className="input-responsive w-full rounded-md border border-input bg-background px-3 py-2"
                placeholder="John"
              />
            </div>
            <div className="space-y-2">
              <label className="text-responsive-sm font-medium">
                Last Name
              </label>
              <input
                type="text"
                className="input-responsive w-full rounded-md border border-input bg-background px-3 py-2"
                placeholder="Doe"
              />
            </div>
            <div className="space-y-2 sm:col-span-2">
              <label className="text-responsive-sm font-medium">Email</label>
              <input
                type="email"
                className="input-responsive w-full rounded-md border border-input bg-background px-3 py-2"
                placeholder="john@example.com"
              />
            </div>
          </form>
          <div className="mt-4 flex justify-end gap-2">
            <Button type="button" className="btn-responsive" variant="outline">
              Cancel
            </Button>
            <Button type="button" className="btn-responsive">
              Submit
            </Button>
          </div>
        </div>
      </div>

      {/* Responsive flex layout */}
      <div className="section-spacing">
        <h2 className="text-responsive-xl font-semibold mb-4">
          Responsive Flex Layout
        </h2>
        <div className="flex-responsive-col gap-responsive">
          <div className="card-responsive flex-1">
            <h3 className="text-responsive-base font-semibold mb-2">
              Column 1
            </h3>
            <p className="text-responsive-sm text-muted-foreground">
              This column stacks vertically on mobile and horizontally on
              tablet+
            </p>
          </div>
          <div className="card-responsive flex-1">
            <h3 className="text-responsive-base font-semibold mb-2">
              Column 2
            </h3>
            <p className="text-responsive-sm text-muted-foreground">
              Content adapts to screen size automatically
            </p>
          </div>
        </div>
      </div>

      {/* Show/hide by breakpoint */}
      <div className="section-spacing">
        <h2 className="text-responsive-xl font-semibold mb-4">
          Breakpoint Visibility
        </h2>
        <div className="card-responsive">
          <div className="mobile-only p-responsive bg-muted rounded-md mb-2">
            <p className="text-responsive-sm font-medium">
              Mobile only (hidden on desktop)
            </p>
          </div>
          <div className="desktop-only p-responsive bg-muted rounded-md">
            <p className="text-responsive-sm font-medium">
              Desktop only (hidden on mobile)
            </p>
          </div>
        </div>
      </div>

      {/* Responsive buttons */}
      <div className="section-spacing">
        <h2 className="text-responsive-xl font-semibold mb-4">
          Responsive Buttons
        </h2>
        <div className="flex flex-wrap gap-responsive">
          <button className="btn-responsive-sm">Small</button>
          <button className="btn-responsive">Default</button>
          <button className="btn-responsive-lg">Large</button>
        </div>
      </div>

      {/* Responsive badges and icons */}
      <div className="section-spacing">
        <h2 className="text-responsive-xl font-semibold mb-4">
          Responsive Components
        </h2>
        <div className="flex flex-wrap items-center gap-responsive">
          <span className="badge-responsive">Badge</span>
          <span className="badge-responsive bg-primary text-primary-foreground">
            Primary
          </span>
          <span className="badge-responsive bg-destructive text-destructive-foreground">
            Alert
          </span>
        </div>
      </div>

      {/* Touch targets */}
      <div className="section-spacing">
        <h2 className="text-responsive-xl font-semibold mb-4">Touch Targets</h2>
        <p className="text-responsive-sm text-muted-foreground mb-4">
          On mobile, these buttons have minimum 44x44px touch targets
        </p>
        <div className="flex flex-wrap gap-responsive">
          <button className="touch-target rounded-md bg-primary px-4 py-2 text-primary-foreground">
            Touch Optimized
          </button>
          <button className="touch-target-sm rounded-md bg-secondary px-3 py-1.5 text-secondary-foreground">
            Small Touch Target
          </button>
        </div>
      </div>
    </div>
  );
}

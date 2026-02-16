# Splash Screen Position Fix

## Problem

The splash screen and main window were appearing at different locations on the screen because:

1. **Both windows were the same size** (1280x720), making the splash screen unnecessarily large
2. **Window-state plugin was restoring ALL window positions**, including the splashscreen, causing conflicts
3. **Saved main window position** from previous session was different from the centered splash screen

## Solution

### 1. Made Splash Screen Smaller and More Professional

**File**: `src-tauri/tauri.conf.json`

**Changes**:

- Reduced splash screen size from **1280x720** to **600x400**
- Added `"resizable": false` - prevents resizing
- Added `"alwaysOnTop": true` - keeps splash on top during loading
- Added `"skipTaskbar": true` - doesn't show in taskbar

```json
{
  "title": "Loading...",
  "label": "splashscreen",
  "url": "splash.html",
  "width": 600,
  "height": 400,
  "center": true,
  "decorations": false,
  "resizable": false,
  "alwaysOnTop": true,
  "skipTaskbar": true
}
```

### 2. Fixed Window-State Plugin to Skip Splashscreen

**File**: `src-tauri/src/plugins/window_state.rs`

**Problem**: The plugin was restoring saved positions for ALL windows, including the splashscreen.

**Fix**: Skip splashscreen in both restore and save operations:

```rust
// During initialization - skip restoring splashscreen state
for (label, window) in windows {
    // Skip splashscreen - it should always be centered
    if label == "splashscreen" {
        continue;
    }
    // ... restore state for other windows
}

// During close - don't save splashscreen state
pub fn on_close_requested<R: Runtime>(window: &Window<R>) {
    // Don't save state for splashscreen
    if window.label() == "splashscreen" {
        return;
    }
    // ... save state for other windows
}
```

## Result

### Before:

- 🔴 Large splash screen (1280x720) taking up most of the screen
- 🔴 Splash screen and main window in different positions
- 🔴 Window-state plugin interfering with splash positioning
- 🔴 Splash screen showed in taskbar

### After:

- ✅ Compact splash screen (600x400) - professional loading screen size
- ✅ Splash screen always centered on screen
- ✅ Main window appears in the same location (either centered or restored position)
- ✅ Splash screen stays on top during loading
- ✅ Splash screen doesn't show in taskbar
- ✅ No positioning conflicts

## Visual Comparison

### Before:

```
Screen: [============================================]
Main:   [====== Main Window (1280x720) ======]
Splash:              [== Splash (1280x720) ==]
        ↑                    ↑
        Different positions!
```

### After:

```
Screen: [============================================]
Main:   [====== Main Window (1280x720) ======]
Splash:        [Splash (600x400)]
        ↑           ↑
        Both centered (splash appears first, then main replaces it)
```

## Testing

Run the app:

```bash
pnpm tauri dev
```

**Expected behavior:**

1. Compact splash screen appears **centered** on the screen
2. Splash screen stays on top while loading
3. Initialization completes (check console for timing logs)
4. Splash screen closes
5. Main window appears in the **same centered location** (or restored position if previously moved)
6. No more "two windows in different places" issue

## Additional Benefits

The smaller splash screen:

- ✅ Loads faster (less rendering)
- ✅ Looks more professional
- ✅ Doesn't cover other windows unnecessarily
- ✅ Provides better UX - users can see it's a loading screen, not the main app

## Files Changed

1. **src-tauri/tauri.conf.json**:
   - Changed splashscreen dimensions from 1280x720 to 600x400
   - Added resizable: false, alwaysOnTop: true, skipTaskbar: true

2. **src-tauri/src/plugins/window_state.rs**:
   - Skip splashscreen during window state restoration
   - Don't save splashscreen state on close

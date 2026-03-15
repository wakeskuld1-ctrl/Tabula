# Excel UI Implementation

This document details the implementation of the Excel-like UI shell, including the Toolbar, Formula Bar, and their integration with the Grid component.

## Components

### 1. ExcelShell (`frontend/src/components/layout/ExcelShell.tsx`)
The main layout container that structures the application into:
- Header (Title/Status)
- Toolbar (Ribbon)
- Formula Bar
- Main Grid Area
- Sheet Bar
- Status Bar

**Key Props:**
- `onSave`: Triggered by the Save button in the Toolbar.
- `onRefresh`: Triggered by the Refresh button in the Toolbar.
- `currentCell`: The label of the currently selected cell (e.g., "A1").
- `currentCellValue`: The value of the currently selected cell.
- `onFormulaChange`: Triggered when the user types in the Formula Bar.
- `onFormulaCommit`: Triggered when the user presses Enter or blurs the Formula Bar.
- `rightPanel`: Optional component to render in the right-side drawer (e.g., Time Machine).
- `onTimeMachine`: Triggered by the Time Machine button in the Toolbar.

### 2. Toolbar (`frontend/src/components/layout/Toolbar.tsx`)
Provides common actions. Currently implemented:
- **Save**: Persists the current session to disk.
- **Refresh**: Reloads the grid data from the backend.
- **Time Machine**: Toggles the version history drawer.
- **Undo/Redo**: Placeholders for future history management.
- **Styling**: Placeholders for Bold, Italic, Underline, Alignment.

### 2.1. TimeMachineDrawer (`frontend/src/components/TimeMachineDrawer.tsx`)
A right-side drawer component that allows users to:
- View the version history of the current Lance dataset.
- Roll back to a specific version (Time Travel).
- **Integration**:
  - Toggled via the Toolbar "Time Machine" button.
  - Rendered in the `rightPanel` prop of `ExcelShell`.
  - Prioritizes in-memory data for immediate feedback after rollback.

### 3. FormulaBar (`frontend/src/components/layout/FormulaBar.tsx`)
Allows editing the content of the selected cell.
- Displays the selected cell address (e.g., "A1").
- Input field for viewing and editing cell content.
- Supports `Enter` key to commit changes.

### 4. GlideGrid (`frontend/src/components/GlideGrid.tsx`)
The data grid component wrapping `@glideapps/glide-data-grid`.
- **Selection Tracking**: Added `onSelectionChange` to report the selected cell coordinates and value to the parent.
- **External Control**: Exposed `updateCell` and `refresh` methods via `ref` to allow external components (like the Formula Bar) to modify the grid.

## Data Flow

1.  **Selection**: 
    - User selects a cell in `GlideGrid`.
    - `GlideGrid` fires `onSelectionChange`.
    - `App.tsx` updates `selectedCellLabel` ("A1") and `selectedCellValue`.
    - `ExcelShell` receives these values and passes them to `FormulaBar`.

2.  **Editing via Formula Bar**:
    - User types in `FormulaBar`.
    - `FormulaBar` fires `onChange`, updating local state in `App.tsx` (for responsiveness).
    - User presses `Enter` or clicks away.
    - `FormulaBar` fires `onCommit`.
    - `App.tsx` calls `glideGridRef.current.updateCell()`.
    - `GlideGrid` calls the backend API `/api/update_cell` and updates its internal display.

3.  **Toolbar Actions**:
    - **Refresh**: Calls `glideGridRef.current.refresh()`, which clears the cache and refetches data.
    - **Save**: Calls `/api/save_session` endpoint.

## Future Enhancements
- **Styling Support**: Connect Toolbar styling buttons to grid cell metadata.
- **Sheet Management**: Implement `SheetBar` actions (Add, Rename, Delete sheets).
- **Undo/Redo**: Implement a history stack in the backend or frontend.

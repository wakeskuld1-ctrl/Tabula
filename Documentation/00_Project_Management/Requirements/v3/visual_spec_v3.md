# Visual Design Specification (Visual) - Iteration 3

## 1. Color Palette (Existing System)
*   **Background**: `#f8f9fa` (Toolbar), `#ffffff` (Grid)
*   **Text**: `#333333`
*   **Border**: `#e2e8f0`
*   **Accent**: `#2563eb` (Blue - Selection/Active State)

## 2. Toolbar Icons (New Additions)

We will use standard Unicode emojis or SVG icons consistent with the current design.

| Action | Icon | Tooltip | Class |
| :--- | :--- | :--- | :--- |
| **Merge** | 🔗 | Merge Cells | `.toolbar-btn .toolbar-btn-wide` |
| **Freeze** | ❄️ | Freeze Panes | `.toolbar-btn` |
| **Filter** | 🌪️ | Toggle Filter | `.toolbar-btn` |
| **Formula**| fx | Insert Function | `.toolbar-btn` |

## 3. Layout (Toolbar Group)

The "Layout/Data" group (where Merge currently is) will be expanded:

`[ Align Left | Center | Right ] [ Merge ] [ Freeze ] [ Filter ] [ Formula ]`

### CSS Reference
```css
.toolbar-group {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
    border-right: 1px solid #e2e8f0;
}

.toolbar-btn {
    padding: 6px 8px;
    border-radius: 4px;
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    font-size: 16px;
    transition: all 0.2s;
}

.toolbar-btn:hover {
    background: #e2e8f0;
}
```

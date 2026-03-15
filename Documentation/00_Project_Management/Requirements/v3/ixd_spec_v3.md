# Interaction Design Specification (IxD) - Iteration 3

## 1. User Flow (Toolbar Actions)

```mermaid
graph TD
    A[User Selects Cell/Range] --> B{Action?}
    B -->|Click Freeze| C[Freeze Rows/Cols up to Selection]
    B -->|Click Filter| D[Toggle Filter Headers]
    B -->|Click Merge| E[Merge Selected Cells]
    B -->|Click Formula| F[Insert Formula]
    
    C --> G[Grid Visual Update (Freeze Lines)]
    D --> H[Grid Header Update (Funnel Icons)]
    E --> I[Grid Visual Update (Merged Cell)]
    F --> J[Formula Bar Focus (=)]
```

## 2. Key Interactions

### 2.1 Freeze Panes (冻结行列)
*   **Trigger**: Toolbar Button "❄️ Freeze" (placed next to Merge).
*   **Pre-condition**: A cell is selected.
*   **Action**:
    *   If no freeze is active: Freeze columns left of selection and rows above selection.
    *   If freeze is active: Unfreeze all.
*   **Visual Feedback**: Thick gray lines appear separating frozen area from scrollable area.

### 2.2 Filter (筛选)
*   **Trigger**: Toolbar Button "🌪️ Filter" (placed next to Merge).
*   **Action**: Toggle filter mode.
*   **Visual Feedback**:
    *   **On**: Column headers show a small funnel icon (▼). Clicking it opens a menu (Sort/Filter).
    *   **Off**: Funnel icons disappear.

### 2.3 Merge Cells (合并单元格)
*   **Trigger**: Toolbar Button "🔗 Merge".
*   **Pre-condition**: A range (>1 cell) is selected.
*   **Action**: Merge selected cells into one. Content of top-left cell is preserved.

### 2.4 Formula (公式)
*   **Trigger**: Toolbar Button "fx".
*   **Action**: Focus Formula Bar and type `=`.

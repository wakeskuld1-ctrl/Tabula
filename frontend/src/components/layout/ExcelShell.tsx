import React, { ReactNode } from 'react';
import { Toolbar } from './Toolbar';
import { FormulaBar } from './FormulaBar';

interface ExcelShellProps {
    children: ReactNode;
    header?: ReactNode;
    // Props for child components could be passed down here or managed via context/state
    onSave?: () => void;
    onRefresh?: () => void;
    onUndo?: () => void;
    onRedo?: () => void;
    canUndo?: boolean;
    canRedo?: boolean;
    currentCell?: string;
    currentCellValue?: string;
    onFormulaChange?: (val: string) => void;
    onFormulaCommit?: () => void;
    onFormulaRefresh?: () => void;
    canFormulaRefresh?: boolean;
    rightPanel?: ReactNode;
    onTimeMachine?: () => void;
    onStyleChange?: (style: any) => void;
    onMerge?: () => void;
    onFreeze?: () => void;
    onFilter?: (anchor?: DOMRect | null) => void;
    onInsertFormula?: () => void;
    activeFilters?: any[];
    onRemoveFilter?: (colId: string) => void;
    onPivot?: () => void;
    leftPanel?: ReactNode;
    bottomPanel?: ReactNode;
}

export const ExcelShell: React.FC<ExcelShellProps> = ({ 
    children, 
    header,
    onSave,
    onRefresh,
    onUndo,
    onRedo,
    canUndo,
    canRedo,
    currentCell,
    currentCellValue,
    onFormulaChange,
    onFormulaCommit,
    onFormulaRefresh,
    canFormulaRefresh,
    rightPanel,
    onTimeMachine,
    onStyleChange,
    onMerge,
    onFreeze,
    onFilter,
    onInsertFormula,
    activeFilters,
    onRemoveFilter,
    onPivot,
    leftPanel,
    bottomPanel
}) => {
    return (
        <div className="app-shell">
            {header}
            <Toolbar 
                onSave={onSave} 
                onRefresh={onRefresh} 
                onUndo={onUndo}
                onRedo={onRedo}
                canUndo={canUndo}
                canRedo={canRedo}
                onTimeMachine={onTimeMachine}  
                onStyleChange={onStyleChange}
                onMerge={onMerge}
                onFreeze={onFreeze}
                onFilter={onFilter}
                onInsertFormula={onInsertFormula}
                activeFilters={activeFilters}
                onRemoveFilter={onRemoveFilter}
                onPivot={onPivot}
            />
            <FormulaBar 
                selectedCell={currentCell} 
                value={currentCellValue} 
                onChange={onFormulaChange}
                onCommit={onFormulaCommit}
                onRefresh={onFormulaRefresh}
                canRefresh={canFormulaRefresh}
            />
            <div className="app-main">
                {leftPanel}
                <div className="app-grid">
                    {children}
                </div>
                {rightPanel}
            </div>
            <div className="app-footer">
                {bottomPanel || "Ready"}
            </div>
        </div>
    );
};

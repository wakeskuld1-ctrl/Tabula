import React from 'react';

interface ToolbarProps {
    onSave?: () => void;
    onUndo?: () => void;
    onRedo?: () => void;
    canUndo?: boolean;
    canRedo?: boolean;
    onRefresh?: () => void;
    onTimeMachine?: () => void;
    onStyleChange?: (style: any) => void;
    onMerge?: () => void;
    onFreeze?: () => void;
    onFilter?: (anchor?: DOMRect | null) => void;
    onInsertFormula?: () => void;
    activeFilters?: { colId: string, colName: string, desc: string }[];
    onRemoveFilter?: (colId: string) => void;
    onPivot?: () => void;
}

interface ToolbarIconProps {
    name: 'save' | 'refresh' | 'undo' | 'redo' | 'history' | 'alignLeft' | 'alignCenter' | 'alignRight' | 'merge' | 'freeze' | 'filter' | 'pivot';
}

const ToolbarIcon: React.FC<ToolbarIconProps> = ({ name }) => {
    const iconProps = {
        width: 14,
        height: 14,
        viewBox: '0 0 24 24',
        fill: 'none',
        stroke: 'currentColor',
        strokeWidth: 2,
        strokeLinecap: 'round' as const,
        strokeLinejoin: 'round' as const,
        'aria-hidden': true,
        focusable: false
    };

    switch (name) {
        case 'save':
            return <svg {...iconProps}><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" /><path d="M17 21v-8H7v8" /><path d="M7 3v5h8" /></svg>;
        case 'refresh':
            return <svg {...iconProps}><path d="M21 2v6h-6" /><path d="M3 12a9 9 0 0 1 15.5-6.36L21 8" /><path d="M3 22v-6h6" /><path d="M21 12a9 9 0 0 1-15.5 6.36L3 16" /></svg>;
        case 'undo':
            return <svg {...iconProps}><path d="M3 7v6h6" /><path d="M3 13a9 9 0 1 0 3-7" /></svg>;
        case 'redo':
            return <svg {...iconProps}><path d="M21 7v6h-6" /><path d="M21 13a9 9 0 1 1-3-7" /></svg>;
        case 'history':
            return <svg {...iconProps}><circle cx="12" cy="12" r="10" /><path d="m12 6 0 6 4 2" /></svg>;
        case 'alignLeft':
            return <svg {...iconProps}><path d="M3 6h14" /><path d="M3 12h10" /><path d="M3 18h14" /></svg>;
        case 'alignCenter':
            return <svg {...iconProps}><path d="M5 6h14" /><path d="M7 12h10" /><path d="M5 18h14" /></svg>;
        case 'alignRight':
            return <svg {...iconProps}><path d="M7 6h14" /><path d="M11 12h10" /><path d="M7 18h14" /></svg>;
        case 'merge':
            return <svg {...iconProps}><path d="M8 3v6l-5 3 5 3v6" /><path d="M16 3v6l5 3-5 3v6" /><path d="M8 12h8" /></svg>;
        case 'freeze':
            return <svg {...iconProps}><path d="M12 2v20" /><path d="m4.93 4.93 14.14 14.14" /><path d="M2 12h20" /><path d="m4.93 19.07 14.14-14.14" /></svg>;
        case 'filter':
            return <svg {...iconProps}><path d="M22 3H2l8 9.46V19l4 2v-8.54L22 3z" /></svg>;
        case 'pivot':
            return <svg {...iconProps}><rect x="3" y="3" width="7" height="7" /><rect x="14" y="3" width="7" height="7" /><rect x="14" y="14" width="7" height="7" /><path d="M10 7h4" /><path d="M7 10v4" /><path d="M10 14h4" /></svg>;
        default:
            return null;
    }
};

export const Toolbar: React.FC<ToolbarProps> = ({ 
    onSave, onUndo, onRedo, canUndo, canRedo, onRefresh, onTimeMachine, onStyleChange, onMerge,
    onFreeze, onFilter, onInsertFormula, activeFilters, onRemoveFilter, onPivot
}) => {
    const filterButtonRef = React.useRef<HTMLButtonElement | null>(null);

    const handleMerge = (e: React.MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        onMerge?.();
    };

    const handleFilterClick = (e: React.MouseEvent<HTMLButtonElement>) => {
        e.preventDefault();
        e.stopPropagation();
        const rect = filterButtonRef.current?.getBoundingClientRect() || null;
        onFilter?.(rect);
    };

    return (
        <div className="toolbar">
            <div className="toolbar-group">
                <button type="button" className="toolbar-btn" onClick={onSave} title="Save" aria-label="Save">
                    <ToolbarIcon name="save" />
                </button>
                <button type="button" className="toolbar-btn" onClick={onRefresh} title="Refresh" aria-label="Refresh">
                    <ToolbarIcon name="refresh" />
                </button>
                <button 
                    type="button"
                    className="toolbar-btn" 
                    onClick={onUndo} 
                    title="Undo" 
                    aria-label="Undo"
                    disabled={canUndo === false} 
                >
                    <ToolbarIcon name="undo" />
                </button>
                <button 
                    type="button"
                    className="toolbar-btn" 
                    onClick={onRedo} 
                    title="Redo" 
                    aria-label="Redo"
                    disabled={canRedo === false} 
                >
                    <ToolbarIcon name="redo" />
                </button>
                <button type="button" className="toolbar-btn" onClick={onTimeMachine} title="Time Machine (History)" aria-label="Time Machine">
                    <ToolbarIcon name="history" />
                </button>
            </div>
            
            <div className="toolbar-divider" />
            
            <div className="toolbar-group">
                <button type="button" className="toolbar-btn" onClick={() => onStyleChange?.({ bold: true })} title="Bold"><b>B</b></button>
                <button type="button" className="toolbar-btn" onClick={() => onStyleChange?.({ italic: true })} title="Italic"><i>I</i></button>
                <button type="button" className="toolbar-btn" onClick={() => onStyleChange?.({ underline: true })} title="Underline"><u>U</u></button>
                <input className="toolbar-color" type="color" onChange={(e) => onStyleChange?.({ color: e.target.value })} title="Text Color" />
                <input className="toolbar-color" type="color" onChange={(e) => onStyleChange?.({ bg_color: e.target.value })} title="Background Color" defaultValue="#ffffff" />
            </div>

            <div className="toolbar-divider" />

            <div className="toolbar-group">
                <button type="button" className="toolbar-btn" onClick={() => onStyleChange?.({ align: 'left' })} title="Align Left" aria-label="Align Left">
                    <ToolbarIcon name="alignLeft" />
                </button>
                <button type="button" className="toolbar-btn" onClick={() => onStyleChange?.({ align: 'center' })} title="Align Center" aria-label="Align Center">
                    <ToolbarIcon name="alignCenter" />
                </button>
                <button type="button" className="toolbar-btn" onClick={() => onStyleChange?.({ align: 'right' })} title="Align Right" aria-label="Align Right">
                    <ToolbarIcon name="alignRight" />
                </button>
                <button type="button" className="toolbar-btn toolbar-btn-wide" onClick={handleMerge} title="Merge Cells">
                    <ToolbarIcon name="merge" /> <span>Merge</span>
                </button>
                <button type="button" className="toolbar-btn" onClick={onFreeze} title="Freeze Panes">
                    <ToolbarIcon name="freeze" /> <span>Freeze</span>
                </button>
                <button type="button" className="toolbar-btn" ref={filterButtonRef} onClick={handleFilterClick} title="Toggle Filter">
                    <ToolbarIcon name="filter" /> <span>Filter</span>
                </button>
                <button type="button" className="toolbar-btn" onClick={onPivot} title="Insert Pivot Table">
                    <ToolbarIcon name="pivot" /> <span>Pivot</span>
                </button>
                {activeFilters && activeFilters.length > 0 && (
                    <div className="toolbar-filter-wrap">
                        <span className="toolbar-filter-label">筛选结果:</span>
                        {activeFilters.map(f => (
                            <div key={f.colId} className="toolbar-filter-chip" title={`${f.colName}: ${f.desc}`}>
                                <span>{f.desc}</span>
                                <button 
                                    type="button"
                                    className="toolbar-filter-remove"
                                    aria-label={`移除筛选 ${f.desc}`}
                                    onClick={(e) => {
                                        e.stopPropagation();
                                        onRemoveFilter?.(f.colId);
                                    }}
                                >
                                    ✕
                                </button>
                            </div>
                        ))}
                    </div>
                )}
                <button type="button" className="toolbar-btn" onClick={onInsertFormula} title="Insert Function">fx</button>
            </div>
        </div>
    );
};

import React from 'react';

interface SheetBarProps {
    sheets?: string[];
    activeSheet?: string;
    onSheetChange?: (sheet: string) => void;
    onAddSheet?: () => void;
    onDeleteSheet?: (sheet: string) => void;
}

export const SheetBar: React.FC<SheetBarProps> = ({ 
    sheets = ['Sheet1'], 
    activeSheet = 'Sheet1', 
    onSheetChange,
    onAddSheet,
    onDeleteSheet
}) => {
    // ### 变更记录
    // - 2026-03-11 21:45: 原因=底部标签页仅鼠标可用，不满足键盘可达性; 目的=补齐左右切换与 Home/End 快捷键。
    const handleTabKeyDown = (e: React.KeyboardEvent<HTMLElement>, index: number) => {
        if (!sheets.length) return;
        const moveTo = (targetIndex: number) => {
            const safeIndex = Math.max(0, Math.min(targetIndex, sheets.length - 1));
            const next = sheets[safeIndex];
            if (next) onSheetChange?.(next);
        };
        if (e.key === 'ArrowRight') {
            e.preventDefault();
            moveTo(index + 1);
        } else if (e.key === 'ArrowLeft') {
            e.preventDefault();
            moveTo(index - 1);
        } else if (e.key === 'Home') {
            e.preventDefault();
            moveTo(0);
        } else if (e.key === 'End') {
            e.preventDefault();
            moveTo(sheets.length - 1);
        }
    };

    return (
        <div className="sheet-bar">
            <div className="sheet-tabs" role="tablist" aria-label="数据表标签">
                {sheets.map((sheet, index) => (
                    <div
                        key={sheet}
                        className={`sheet-tab ${sheet === activeSheet ? 'active' : ''}`}
                        role="tab"
                        aria-selected={sheet === activeSheet}
                        tabIndex={sheet === activeSheet ? 0 : -1}
                        data-testid={`sheet-tab-${sheet}`}
                        onClick={() => onSheetChange?.(sheet)}
                        onKeyDown={(e) => handleTabKeyDown(e, index)}
                    >
                        {sheet}
                        {sheet !== 'Table' && (
                            <button
                                type="button"
                                className="sheet-close"
                                aria-label={`删除表 ${sheet}`}
                                onClick={(e) => {
                                    e.stopPropagation();
                                    onDeleteSheet?.(sheet);
                                }}
                            >
                                ×
                            </button>
                        )}
                    </div>
                ))}
            </div>
            <button 
                onClick={onAddSheet}
                className="sheet-add"
                title="新建沙盘"
            >
                +
            </button>
        </div>
    );
};

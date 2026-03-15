import React from 'react';
import ReactDOM from "react-dom";
import { useFormulaLogic } from '../../hooks/useFormulaLogic';

interface FormulaBarProps {
    selectedCell?: string; // e.g., "A1"
    value?: string;
    onChange?: (newValue: string) => void;
    onCommit?: () => void;
    onRefresh?: () => void;
    canRefresh?: boolean;
}

export const FormulaBar: React.FC<FormulaBarProps> = ({
    selectedCell = '',
    value = '',
    onChange,
    onCommit,
    onRefresh,
    canRefresh = false
}) => {
    const {
        text,
        suggestions,
        selectedIndex,
        showFxPopup,
        setShowFxPopup,
        inputRef,
        coords,
        handleKeyDown,
        handleChange,
        toggleFxPopup,
        applySuggestion,
        allFunctions
    } = useFormulaLogic({
        initialValue: value,
        onChange: onChange,
        onCommit: onCommit
    });

    return (
        <div className="formula-bar">
            <div className="formula-cell">
                {selectedCell}
            </div>
            
            {/* FX Button */}
            <button 
                type="button"
                className={`formula-fx ${showFxPopup ? 'active' : ''}`}
                onMouseDown={toggleFxPopup}
                title="Insert Function"
                aria-label="插入函数"
            >
                fx
            </button>

            <button
                type="button"
                className={`formula-refresh ${canRefresh ? '' : 'disabled'}`}
                onMouseDown={(e) => {
                    e.preventDefault();
                    if (canRefresh) onRefresh?.();
                }}
                title="刷新公式"
                aria-label="刷新公式"
                disabled={!canRefresh}
            >
                刷新
            </button>

            <input
                ref={inputRef}
                type="text"
                value={text}
                onChange={handleChange}
                onKeyDown={handleKeyDown}
                onBlur={() => {
                    // Optional: commit on blur? Or just leave it?
                    // Excel usually commits on Enter or clicking away (which is complex).
                    // For now, let's keep it simple. User must press Enter to commit.
                    // Or we can commit on blur if we want.
                    // onCommit?.(); 
                }}
                className="formula-input"
                placeholder=""
            />

            {/* Suggestions Dropdown (Auto) - Portal */}
            {suggestions.length > 0 && coords && !showFxPopup && ReactDOM.createPortal(
                <div className="fx-popup" style={{
                    position: "absolute",
                    top: coords.top,
                    left: coords.left
                }}>
                    <div className="fx-popup-title">
                        Suggested Formulas
                    </div>
                    {suggestions.map((s, i) => (
                        <div
                            key={s}
                            className={`fx-popup-item ${i === selectedIndex ? 'active' : ''}`}
                            onMouseDown={(e) => {
                                e.preventDefault();
                                applySuggestion(s);
                            }}
                        >
                            {s}
                        </div>
                    ))}
                </div>,
                document.body
            )}

            {/* FX Popup (Manual) - Portal */}
            {showFxPopup && coords && ReactDOM.createPortal(
                <div className="fx-modal" style={{
                    position: "absolute",
                    top: coords.top,
                    left: coords.left
                }}>
                    <div className="fx-modal-title">
                        <span>Insert Function</span>
                        <button
                            type="button"
                            className="fx-close"
                            onClick={() => setShowFxPopup(false)}
                            aria-label="关闭函数面板"
                        >
                            ×
                        </button>
                    </div>
                    <div className="fx-modal-list">
                        {allFunctions.map(f => (
                            <div 
                                key={f}
                                className="fx-popup-item"
                                onMouseDown={(e) => {
                                    e.preventDefault();
                                    applySuggestion(f);
                                }}
                            >
                                {f}
                            </div>
                        ))}
                    </div>
                </div>,
                document.body
            )}
        </div>
    );
};

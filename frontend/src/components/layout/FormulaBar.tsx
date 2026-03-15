// ### Change Log
// - 2026-03-15: Reason=Always-on tips need memoized selection; Purpose=avoid redundant recompute
// - 2026-03-15: Reason=Add collapsible tips state; Purpose=track expanded rows
import React, { useMemo, useState } from "react";
import ReactDOM from "react-dom";
import { useFormulaLogic } from "../../hooks/useFormulaLogic";
// ### Change Log
// - 2026-03-15: Reason=Always-on tips need data source + selector; Purpose=centralize filtering logic
import formulaHelpData from "../../data/formula_help.json";
// ### Change Log
// - 2026-03-15: Reason=Conditional popup needs trigger + summary helpers; Purpose=keep UI logic thin
import {
    FormulaHelpItem,
    selectFormulaHelpItems,
    shouldShowFormulaHelp,
    formatFormulaTipSummary,
} from "../../utils/formulaHelp";
// ### Change Log
// - 2026-03-15: Reason=Reuse bilingual labels; Purpose=keep tips text consistent
import { APP_LABELS } from "../../utils/appLabels";

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
    // ### Change Log
    // - 2026-03-15: Reason=JSON data needs stable typing; Purpose=ensure TypeScript constraints apply
    const formulaHelpItems = formulaHelpData as FormulaHelpItem[];
    // ### Change Log
    // - 2026-03-15: Reason=Always-on tips should show full list by default; Purpose=match “always visible” requirement
    const FORMULA_TIPS_DEFAULT_LIMIT = Math.max(1, formulaHelpItems.length);
    // ### Change Log
    // - 2026-03-15: Reason=Tips should filter with input; Purpose=match formula entry flow
    const tipsItems = useMemo(() => {
        return selectFormulaHelpItems(formulaHelpItems, text, FORMULA_TIPS_DEFAULT_LIMIT);
    }, [formulaHelpItems, text]);
    // ### Change Log
    // - 2026-03-15: Reason=Tips should gate by focus; Purpose=avoid showing on selection
    const [isFocused, setIsFocused] = useState(false);
    // ### Change Log
    // - 2026-03-15: Reason=Tips should only show on '=' or fx while focused; Purpose=match popup requirement
    const shouldShowTips = useMemo(() => {
        return shouldShowFormulaHelp({ text, isFxToggled: showFxPopup, isFocused });
    }, [text, showFxPopup, isFocused]);
    // ### Change Log
    // - 2026-03-15: Reason=Collapsed by default; Purpose=store expanded state by formula name
    const [expandedTips, setExpandedTips] = useState<Record<string, boolean>>({});
    // ### Change Log
    // - 2026-03-15: Reason=Toggle requires stable callback; Purpose=avoid re-creating per render
    const toggleTip = (name: string) => {
        setExpandedTips((prev) => ({
            ...prev,
            [name]: !prev[name],
        }));
    };

    /*
    ### Change Log
    * - 2026-03-15: Reason=User requested always-on formula tips; Purpose=show tips below formula bar
    * - 2026-03-15: Reason=Need bilingual content; Purpose=reuse formula help labels
    * - 2026-03-15: Reason=JSX comment tail caused parse error; Purpose=keep change log outside JSX
    */
    return (
        <div className="formula-bar">
            <div className="formula-bar-row">
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
                onFocus={() => {
                    // ### Change Log
                    // - 2026-03-15: Reason=Only show tips while editing; Purpose=mark focus entry
                    setIsFocused(true);
                }}
                onBlur={() => {
                    // ### Change Log
                    // - 2026-03-15: Reason=Hide tips when leaving input; Purpose=avoid selection-triggered tips
                    setIsFocused(false);
                    // Optional: commit on blur? Or just leave it?
                    // Excel usually commits on Enter or clicking away (which is complex).
                    // For now, let's keep it simple. User must press Enter to commit.
                    // Or we can commit on blur if we want.
                    // onCommit?.(); 
                }}
                className="formula-input"
                placeholder=""
            />

            </div>
            {/* Formula tips panel (conditional + collapsible) */}
            {shouldShowTips ? (
                <div className="formula-tips">
                    <div className="formula-tips-header">
                        <span>{APP_LABELS.formulaHelp.title}</span>
                        <span className="formula-tips-sub">
                            {APP_LABELS.formulaHelp.searchPlaceholder}
                        </span>
                    </div>
                    <div className="formula-tips-list">
                        {tipsItems.length === 0 ? (
                            <div className="formula-tips-empty">
                                {APP_LABELS.formulaHelp.empty}
                            </div>
                        ) : (
                            tipsItems.map((item) => {
                                // ### Change Log
                                // - 2026-03-15: Reason=Collapsed by default; Purpose=derive expanded state per row
                                const isExpanded = Boolean(expandedTips[item.name]);
                                // ### Change Log
                                // - 2026-03-15: Reason=Note may be placeholder; Purpose=avoid showing "—"
                                const note = (item.note || "").trim();
                                const shouldShowNote = note !== "" && note !== "—";

                                return (
                                    <div
                                        key={item.name}
                                        className={`formula-tip-item ${isExpanded ? "is-expanded" : "is-collapsed"}`}
                                        role="button"
                                        tabIndex={0}
                                        aria-expanded={isExpanded}
                                        onClick={() => toggleTip(item.name)}
                                        onKeyDown={(event) => {
                                            if (event.key === "Enter" || event.key === " ") {
                                                event.preventDefault();
                                                toggleTip(item.name);
                                            }
                                        }}
                                    >
                                        <div className="formula-tip-summary">
                                            {formatFormulaTipSummary(item)}
                                        </div>
                                        {isExpanded ? (
                                            <div className="formula-tip-details">
                                                <div className="formula-tip-name">{item.name}</div>
                                                <div className="formula-tip-syntax">{item.syntax}</div>
                                                <div className="formula-tip-example">{item.example}</div>
                                                <div className="formula-tip-purpose">{item.purpose}</div>
                                                {item.paramNotes ? (
                                                    <div className="formula-tip-notes">{item.paramNotes}</div>
                                                ) : null}
                                                {shouldShowNote ? (
                                                    <div className="formula-tip-note">{item.note}</div>
                                                ) : null}
                                            </div>
                                        ) : null}
                                    </div>
                                );
                            })
                        )}
                    </div>
                </div>
            ) : null}

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

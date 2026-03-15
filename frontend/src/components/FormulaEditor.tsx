import React from "react";
import ReactDOM from "react-dom";
import { GridCell, TextCell } from "@glideapps/glide-data-grid";
import { useFormulaLogic } from "../hooks/useFormulaLogic";

interface FormulaEditorProps {
    value: TextCell;
    onChange: (newValue: GridCell) => void;
    onFinished: () => void;
}

export const FormulaEditor: React.FC<FormulaEditorProps> = ({ value, onChange, onFinished }) => {
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
        initialValue: value.data?.toString() || "",
        onChange: (newVal) => {
            onChange({
                ...value,
                data: newVal,
                displayData: newVal,
            });
        },
        onCommit: onFinished
    });

    return (
        <div className="gdg-custom-editor" style={{ width: "100%", height: "100%", position: "relative", display: "flex", alignItems: "center" }}>
            <input
                ref={inputRef}
                className="gdg-input"
                style={{
                    flex: 1,
                    height: "100%",
                    border: "none",
                    outline: "none",
                    padding: "0 8px",
                    fontSize: "13px",
                    fontFamily: "var(--gdg-font-family)",
                }}
                value={text}
                onChange={handleChange}
                onKeyDown={handleKeyDown}
                autoFocus
            />
            {/* FX Button */}
            <div 
                style={{
                    padding: "0 8px",
                    cursor: "pointer",
                    fontWeight: "bold",
                    color: "#666",
                    fontStyle: "italic",
                    fontFamily: "serif",
                    borderLeft: "1px solid #eee",
                    height: "100%",
                    display: "flex",
                    alignItems: "center",
                    backgroundColor: showFxPopup ? "#e6f7ff" : "transparent"
                }}
                onMouseDown={toggleFxPopup}
                title="Insert Function"
            >
                fx
            </div>

            {/* Suggestions Dropdown (Auto) */}
            {suggestions.length > 0 && coords && !showFxPopup && ReactDOM.createPortal(
                <div style={{
                    position: "absolute",
                    top: coords.top,
                    left: coords.left,
                    minWidth: "200px",
                    backgroundColor: "white",
                    border: "1px solid #ccc",
                    boxShadow: "0 2px 8px rgba(0,0,0,0.2)",
                    zIndex: 99999,
                    maxHeight: "200px",
                    overflowY: "auto",
                    borderRadius: "4px"
                }}>
                    <div style={{ padding: "4px 8px", background: "#f0f0f0", fontSize: "11px", fontWeight: "bold", borderBottom: "1px solid #eee" }}>
                        Suggested Formulas
                    </div>
                    {suggestions.map((s, i) => (
                        <div
                            key={s}
                            style={{
                                padding: "6px 10px",
                                cursor: "pointer",
                                backgroundColor: i === selectedIndex ? "#e6f7ff" : "white",
                                fontSize: "12px",
                                color: "black"
                            }}
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

            {/* FX Popup (Manual) */}
            {showFxPopup && coords && ReactDOM.createPortal(
                <div style={{
                    position: "absolute",
                    top: coords.top,
                    left: coords.left,
                    width: "300px",
                    backgroundColor: "white",
                    border: "1px solid #ccc",
                    boxShadow: "0 4px 12px rgba(0,0,0,0.3)",
                    zIndex: 99999,
                    maxHeight: "300px",
                    borderRadius: "4px",
                    display: "flex",
                    flexDirection: "column"
                }}>
                    <div style={{ 
                        padding: "8px", 
                        background: "#f0f0f0", 
                        fontSize: "12px", 
                        fontWeight: "bold", 
                        borderBottom: "1px solid #eee",
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center"
                    }}>
                        <span>Insert Function</span>
                        <span 
                            style={{ cursor: "pointer", fontSize: "14px" }} 
                            onClick={() => setShowFxPopup(false)}
                        >
                            ×
                        </span>
                    </div>
                    <div style={{ overflowY: "auto", flex: 1 }}>
                        {allFunctions.map((s) => (
                            <div
                                key={s}
                                style={{
                                    padding: "6px 10px",
                                    cursor: "pointer",
                                    fontSize: "12px",
                                    color: "black",
                                    borderBottom: "1px solid #f9f9f9"
                                }}
                                onMouseDown={(e) => {
                                    e.preventDefault();
                                    applySuggestion(s);
                                }}
                                onMouseEnter={(e) => e.currentTarget.style.backgroundColor = "#e6f7ff"}
                                onMouseLeave={(e) => e.currentTarget.style.backgroundColor = "white"}
                            >
                                {s}
                            </div>
                        ))}
                    </div>
                </div>,
                document.body
            )}
        </div>
    );
};

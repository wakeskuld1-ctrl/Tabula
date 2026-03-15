import { useState, useEffect, useRef, KeyboardEvent, ChangeEvent, MouseEvent } from 'react';
import { FormulaEngine } from '../utils/FormulaEngine';

interface UseFormulaLogicProps {
    initialValue: string;
    onChange?: (value: string) => void;
    onCommit?: () => void;
}

export const useFormulaLogic = ({ initialValue, onChange, onCommit }: UseFormulaLogicProps) => {
    const [text, setText] = useState<string>(initialValue);
    const [suggestions, setSuggestions] = useState<string[]>([]);
    const [selectedIndex, setSelectedIndex] = useState(0);
    const [showFxPopup, setShowFxPopup] = useState(false);
    const inputRef = useRef<HTMLInputElement>(null);
    const [coords, setCoords] = useState<{ top: number, left: number } | null>(null);

    // Sync local text if initialValue changes
    useEffect(() => {
        setText(initialValue);
    }, [initialValue]);

    // Coords logic
    useEffect(() => {
        if (inputRef.current) {
            const rect = inputRef.current.getBoundingClientRect();
            setCoords({ top: rect.bottom + window.scrollY, left: rect.left + window.scrollX });
        }
    }, [text, showFxPopup]);

    // Auto-suggestion logic
    useEffect(() => {
        if (text.startsWith("=")) {
            const match = text.match(/=([A-Z0-9\.]*)$/i);
            if (match) {
                const prefix = match[1].toUpperCase();
                const allFuncs = FormulaEngine.getInstance().getSupportedFunctions();
                const filtered = allFuncs.filter(f => f.startsWith(prefix)).slice(0, 10);
                console.log(`[useFormulaLogic] Suggesting for prefix '${prefix}':`, filtered);
                setSuggestions(filtered);
                setSelectedIndex(0);
            } else {
                setSuggestions([]);
            }
        } else {
            setSuggestions([]);
        }
    }, [text]);

    const applySuggestion = (funcName: string) => {
        let newText = text;
        if (text.startsWith("=")) {
            newText = text.replace(/=([A-Z0-9\.]*)$/i, `=${funcName}(`);
        } else {
            newText = `=${funcName}(`;
        }
        
        setText(newText);
        if (onChange) onChange(newText);
        setSuggestions([]);
        setShowFxPopup(false);
        inputRef.current?.focus();
    };

    const handleKeyDown = (e: KeyboardEvent) => {
        if (suggestions.length > 0) {
            if (e.key === "ArrowDown") {
                e.preventDefault();
                e.stopPropagation();
                setSelectedIndex(i => (i + 1) % suggestions.length);
            } else if (e.key === "ArrowUp") {
                e.preventDefault();
                e.stopPropagation();
                setSelectedIndex(i => (i - 1 + suggestions.length) % suggestions.length);
            } else if (e.key === "Enter" || e.key === "Tab") {
                e.preventDefault();
                e.stopPropagation();
                applySuggestion(suggestions[selectedIndex]);
            } else if (e.key === "Escape") {
                e.preventDefault();
                e.stopPropagation();
                setSuggestions([]);
            }
        } else {
            if (e.key === "Enter") {
                // Let it bubble or handle commit?
                // If we bubble, GDG handles it. If we call onCommit, we handle it.
                // Original code called onFinished().
                if (onCommit) onCommit();
            }
        }
    };

    const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
        const newVal = e.target.value;
        setText(newVal);
        if (onChange) onChange(newVal);
    };

    const toggleFxPopup = (e: MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setShowFxPopup(!showFxPopup);
    };

    return {
        text,
        setText,
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
        allFunctions: FormulaEngine.getInstance().getSupportedFunctions()
    };
};

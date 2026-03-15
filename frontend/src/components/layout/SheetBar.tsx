import React from 'react';
// ### Change Log
// - 2026-03-15: Reason=Place add button inline with tabs; Purpose=share list model with tests
import { buildSheetTabItems } from '../../utils/sheetTabsModel';

// ### Change Log
// - 2026-03-14: Reason=Switch tabs data source to sessions; Purpose=render sessionId-based tabs
// - 2026-03-14: Reason=Fix garbled labels; Purpose=keep aria/text readable

interface SessionTabItem {
    sessionId: string;
    displayName: string;
    isDefault: boolean;
}

interface SheetBarProps {
    sessions?: SessionTabItem[];
    activeSessionId?: string;
    onSessionChange?: (sessionId: string) => void;
    onAddSession?: () => void;
}

const TABLIST_LABEL = "会话标签列表";
const DEFAULT_TAG_TEXT = "只读";
const DEFAULT_TAG_LABEL = "只读会话";
const ADD_TITLE = "新增沙盘";

export const SheetBar: React.FC<SheetBarProps> = ({
    sessions = [],
    activeSessionId = '',
    onSessionChange,
    onAddSession
}) => {
    // ### Change Log
    // - 2026-03-15: Reason=Add button should be part of tab flow; Purpose=render inline
    const items = buildSheetTabItems(sessions);
    // ### Change Log
    // - 2026-03-14: Reason=Keyboard navigation should follow session order; Purpose=accessible tab switching
    const handleTabKeyDown = (e: React.KeyboardEvent<HTMLElement>, index: number) => {
        if (!sessions.length) return;
        const moveTo = (targetIndex: number) => {
            const safeIndex = Math.max(0, Math.min(targetIndex, sessions.length - 1));
            const next = sessions[safeIndex];
            if (next) onSessionChange?.(next.sessionId);
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
            moveTo(sessions.length - 1);
        }
    };

    return (
        <div className="sheet-bar">
            <div className="sheet-tabs" role="tablist" aria-label={TABLIST_LABEL}>
                {items.map((item) => {
                    if (item.type === 'add') {
                        return (
                            <button
                                key="sheet-add"
                                onClick={onAddSession}
                                className="sheet-add"
                                title={ADD_TITLE}
                                aria-label={ADD_TITLE}
                            >
                                +
                            </button>
                        );
                    }
                    // ### Change Log
                    // - 2026-03-15: Reason=Keyboard navigation uses session index; Purpose=keep arrows aligned
                    const sessionIndex = sessions.findIndex(session => session.sessionId === item.sessionId);
                    return (
                        <div
                            key={item.sessionId}
                            className={`sheet-tab ${item.sessionId === activeSessionId ? 'active' : ''}`}
                            role="tab"
                            aria-selected={item.sessionId === activeSessionId}
                            tabIndex={item.sessionId === activeSessionId ? 0 : -1}
                            data-testid={`sheet-tab-${item.sessionId}`}
                            onClick={() => onSessionChange?.(item.sessionId)}
                            onKeyDown={(e) => handleTabKeyDown(e, sessionIndex)}
                        >
                            {item.displayName}
                            {/* ### Change Log
                                - 2026-03-14: Reason=Default session is read-only; Purpose=show a visible tag */}
                            {item.isDefault && (
                                <span className="sheet-default-tag" aria-label={DEFAULT_TAG_LABEL}>
                                    {DEFAULT_TAG_TEXT}
                                </span>
                            )}
                        </div>
                    );
                })}
            </div>
        </div>
    );
};

import React from 'react';

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
                {sessions.map((session, index) => (
                    <div
                        key={session.sessionId}
                        className={`sheet-tab ${session.sessionId === activeSessionId ? 'active' : ''}`}
                        role="tab"
                        aria-selected={session.sessionId === activeSessionId}
                        tabIndex={session.sessionId === activeSessionId ? 0 : -1}
                        data-testid={`sheet-tab-${session.sessionId}`}
                        onClick={() => onSessionChange?.(session.sessionId)}
                        onKeyDown={(e) => handleTabKeyDown(e, index)}
                    >
                        {session.displayName}
                        {/* ### Change Log
                            - 2026-03-14: Reason=Default session is read-only; Purpose=show a visible tag */}
                        {session.isDefault && (
                            <span className="sheet-default-tag" aria-label={DEFAULT_TAG_LABEL}>
                                {DEFAULT_TAG_TEXT}
                            </span>
                        )}
                    </div>
                ))}
            </div>
            <button
                onClick={onAddSession}
                className="sheet-add"
                title={ADD_TITLE}
                aria-label={ADD_TITLE}
            >
                +
            </button>
        </div>
    );
};

import React, { useEffect, useState } from 'react';
// ### Change Log
// - 2026-03-15: Reason=API route alignment; Purpose=use shared GridAPI helper
import { fetchVersions } from '../utils/GridAPI';

interface Version {
    version: number;
    timestamp: number;
    metadata: Record<string, string>;
}

interface TimeMachineDrawerProps {
    tableName: string;
    onClose: () => void;
    onCheckout: (version: number) => void;
}

export const TimeMachineDrawer: React.FC<TimeMachineDrawerProps> = ({ tableName, onClose, onCheckout }) => {
    const [versions, setVersions] = useState<Version[]>([]);
    const [loading, setLoading] = useState(false);

    useEffect(() => {
        if (!tableName) return;
        let cancelled = false;
        const loadVersions = async () => {
            setLoading(true);
            try {
                // ### Change Log
                // - 2026-03-15: Reason=Route alignment; Purpose=delegate versions fetch to GridAPI
                const data = await fetchVersions(tableName);
                if (data.status === 'ok') {
                    if (!cancelled) {
                        setVersions((data.versions || []).slice().reverse()); // Show newest first
                    }
                } else {
                    console.error("Failed to fetch versions:", data.message || data.error || "unknown error");
                }
            } catch (err) {
                // ### Change Log
                // - 2026-03-15: Reason=Keep error visibility; Purpose=preserve debugging info
                console.error("Failed to fetch versions:", err);
            } finally {
                if (!cancelled) setLoading(false);
            }
        };
        loadVersions();
        return () => {
            cancelled = true;
        };
    }, [tableName]);

    const formatTime = (ts: number) => {
        return new Date(ts * 1000).toLocaleString();
    };

    return (
        <div className="time-machine-drawer">
            <div className="tm-header">
                <span className="tm-title">
                    <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        aria-hidden="true"
                        focusable="false"
                        style={{ marginRight: 6 }}
                    >
                        <path d="M6 2h12" />
                        <path d="M6 22h12" />
                        <path d="M17 2v5l-5 5-5-5V2" />
                        <path d="M7 22v-5l5-5 5 5v5" />
                    </svg>
                    时光机 (Time Machine)
                </span>
                <button onClick={onClose} className="tm-close-btn" title="Close" aria-label="关闭时光机面板">✕</button>
            </div>
            
            <div className="tm-list">
                {loading && <div className="empty-state-tm">Loading quantum states...</div>}
                {!loading && versions.length === 0 && (
                    <div className="empty-state-tm">
                        暂无版本记录 (No versions found).
                        <br />Make some edits to create history.
                    </div>
                )}
                
                {versions.map((v) => (
                    <button
                        type="button"
                        key={v.version} 
                        className="tm-item"
                        onClick={() => {
                            if (window.confirm(`确定要回滚到版本 ${v.version} 吗? (Are you sure to rollback to v${v.version}?)`)) {
                                onCheckout(v.version);
                            }
                        }}
                    >
                        <div className="tm-version">
                            <span>Version {v.version}</span>
                        </div>
                        <div className="tm-time">{formatTime(v.timestamp)}</div>
                        {Object.keys(v.metadata).length > 0 && (
                            <div className="tm-meta">
                                {JSON.stringify(v.metadata)}
                            </div>
                        )}
                    </button>
                ))}
            </div>
        </div>
    );
};

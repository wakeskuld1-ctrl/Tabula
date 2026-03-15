import React, { useEffect, useState } from 'react';

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

const parseJsonSafely = async (res: Response): Promise<{ ok: true; data: any } | { ok: false; reason: string; rawPreview: string }> => {
    const raw = await res.text();
    if (!raw || raw.trim().length === 0) {
        return { ok: false, reason: `empty response (status ${res.status})`, rawPreview: "" };
    }
    try {
        return { ok: true, data: JSON.parse(raw) };
    } catch {
        return { ok: false, reason: `invalid json (status ${res.status})`, rawPreview: raw.slice(0, 200) };
    }
};

export const TimeMachineDrawer: React.FC<TimeMachineDrawerProps> = ({ tableName, onClose, onCheckout }) => {
    const [versions, setVersions] = useState<Version[]>([]);
    const [loading, setLoading] = useState(false);

    useEffect(() => {
        if (!tableName) return;
        let cancelled = false;
        const loadVersions = async () => {
            setLoading(true);
            try {
                const res = await fetch(`/api/versions?table_name=${encodeURIComponent(tableName)}`);
                const parsed = await parseJsonSafely(res);
                if (!res.ok) {
                    const preview = parsed.ok ? JSON.stringify(parsed.data).slice(0, 200) : parsed.rawPreview;
                    console.error(`Failed to fetch versions: HTTP ${res.status}`, preview);
                    return;
                }
                if (!parsed.ok) {
                    console.error(`Failed to fetch versions: ${parsed.reason}`);
                    return;
                }
                const data = parsed.data;
                if (data.status === 'ok') {
                    if (!cancelled) {
                        setVersions((data.versions || []).slice().reverse()); // Show newest first
                    }
                } else {
                    console.error("Failed to fetch versions:", data.message || data.error || "unknown error");
                }
            } catch (err) {
                console.error(err);
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

import type { Notice } from "../../features/paste/paster";

/** "Copied", or why a paste failed, over the bottom of the list. Always in the tree, so screen
 * readers announce each new message. */
export function NoticeBar({ notice }: { notice: Notice | null }) {
  return (
    <div className="notice-region" role="status" aria-live="polite">
      {notice && (
        <div className={`notice ${notice.tone}`}>
          <p className="notice-message">{notice.message}</p>
          {notice.detail && <p className="notice-detail">{notice.detail}</p>}
          {notice.action && (
            <button type="button" className="notice-action" onClick={notice.action.run}>
              {notice.action.label}
            </button>
          )}
        </div>
      )}
    </div>
  );
}

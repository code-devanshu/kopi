import { ImageOff } from "lucide-react";
import { useEffect, useState } from "react";
import { KindIcon } from "../../components/KindIcon";
import { imageUrl } from "../../lib/assets";
import { dateTime, hex, KIND_LABELS, size } from "../../lib/format";
import { commands, unwrap, type ClipDetail } from "../../lib/ipc";
import { highlight } from "./highlight";

type Loaded = { id: number; version: number; detail: ClipDetail | null; error: string | null };

/**
 * The selected clip in full. `version` changes when any clip changes, so a clip that was
 * copied again or deleted is fetched again. `onCopyText` copies the text read from an image.
 */
export function Preview({
  id,
  version,
  query,
  onCopyText,
}: {
  id: number;
  version: number;
  query: string;
  onCopyText: (id: number) => void;
}) {
  const [loaded, setLoaded] = useState<Loaded | null>(null);

  useEffect(() => {
    let live = true;
    unwrap(commands.getClip(id)).then(
      (detail) => live && setLoaded({ id, version, detail, error: null }),
      (e) => live && setLoaded({ id, version, detail: null, error: String(e) }),
    );
    return () => {
      live = false;
    };
  }, [id, version]);

  if (!loaded) return <aside className="preview empty" aria-label="Preview" />;
  // Until the selected clip arrives, the last one stays up, so arrowing through doesn't flash.
  const stale = loaded.id !== id || loaded.version !== version;
  if (!stale && loaded.error) {
    return (
      <aside className="preview empty" aria-label="Preview">
        <p className="muted">Couldn’t load this clip.</p>
      </aside>
    );
  }
  const detail = loaded.detail;
  if (!detail) return <aside className="preview empty" aria-label="Preview" />;

  return (
    <aside className="preview" aria-label="Preview" aria-busy={stale}>
      <div className="preview-body" key={detail.id}>
        <Body detail={detail} query={query} onCopyText={onCopyText} />
      </div>
      <dl className="preview-meta">
        <dt>Kind</dt>
        <dd>
          <KindIcon kind={detail.kind} size={12} /> {kindLabel(detail)}
        </dd>
        {detail.sourceAppName && (
          <>
            <dt>From</dt>
            <dd>{detail.sourceAppName}</dd>
          </>
        )}
        {detail.copiedOn && (
          <>
            <dt>Copied on</dt>
            <dd>{detail.copiedOn}</dd>
          </>
        )}
        <dt>Copied</dt>
        <dd>{dateTime(detail.createdAt)}</dd>
        {detail.lastUsedAt !== detail.createdAt && (
          <>
            <dt>Last used</dt>
            <dd>{dateTime(detail.lastUsedAt)}</dd>
          </>
        )}
        {detail.useCount > 1 && (
          <>
            <dt>Used</dt>
            <dd>{detail.useCount} times</dd>
          </>
        )}
        <dt>Size</dt>
        <dd>{size(detail.sizeBytes)}</dd>
      </dl>
    </aside>
  );
}

function Body({
  detail,
  query,
  onCopyText,
}: {
  detail: ClipDetail;
  query: string;
  onCopyText: (id: number) => void;
}) {
  const m = detail.metadata;
  switch (detail.kind) {
    case "image":
      return (
        <div className="preview-image-stack">
          <PreviewImage key={detail.contentHash} hash={detail.contentHash} />
          <ImageText detail={detail} query={query} onCopy={() => onCopyText(detail.id)} />
        </div>
      );
    case "color": {
      const color = hex(m?.rgba);
      return (
        <div className="preview-color">
          <span className="preview-swatch" style={{ background: color ?? "transparent" }} />
          <span className="mono">{color ?? detail.text}</span>
        </div>
      );
    }
    case "file":
      if (m?.names?.length) {
        return (
          <ul className="preview-files">
            {m.names.map((name, i) => (
              <li key={i}>
                <KindIcon kind="file" size={13} /> {name}
              </li>
            ))}
          </ul>
        );
      }
      break;
  }
  if (!detail.text) return <p className="muted">No text</p>;
  return (
    <pre className={detail.kind === "code" ? "preview-text mono" : "preview-text"}>
      {highlight(detail.text, query).map((span, i) =>
        span.matched ? <mark key={i}>{span.text}</mark> : span.text,
      )}
    </pre>
  );
}

/** The text Kopi read from an image, while Find text in images is on. */
function ImageText({ detail, query, onCopy }: { detail: ClipDetail; query: string; onCopy: () => void }) {
  if (detail.readingText) return <p className="image-text-status muted">Looking for text…</p>;
  if (!detail.text) return null;
  return (
    <section className="image-text" aria-labelledby="image-text-title">
      <div className="image-text-head">
        <h3 id="image-text-title" className="preview-heading">
          Text in image
        </h3>
        <button type="button" className="link-button" onClick={onCopy}>
          Copy text
        </button>
      </div>
      <pre className="preview-text image-text-body">
        {highlight(detail.text, query).map((span, i) =>
          span.matched ? <mark key={i}>{span.text}</mark> : span.text,
        )}
      </pre>
    </section>
  );
}

function PreviewImage({ hash }: { hash: string }) {
  const [failed, setFailed] = useState(false);
  if (failed) {
    return (
      <div className="preview-missing">
        <ImageOff size={20} strokeWidth={1.5} aria-hidden="true" />
        <p className="muted">Can’t show this image</p>
      </div>
    );
  }
  return (
    <div className="preview-image">
      <img src={imageUrl(hash)} alt="" draggable={false} onError={() => setFailed(true)} />
    </div>
  );
}

function kindLabel(detail: ClipDetail): string {
  const m = detail.metadata;
  if (detail.kind === "image" && m?.w && m?.h) return `Image, ${m.w} × ${m.h}`;
  if (detail.kind === "link" && m?.host) return `Link to ${m.host}`;
  if (detail.kind === "code" && m?.lang) return `Code, ${m.lang}`;
  if (detail.kind === "file" && m?.count && m.count > 1) return `${m.count} files`;
  return KIND_LABELS[detail.kind];
}

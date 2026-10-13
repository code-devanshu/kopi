//! `kopi://` serves images to the panel, so they don't go over IPC as base64. The UI never
//! reads files itself.
//!
//! - `thumb/<content hash>`: an image clip's 320 px PNG thumbnail. Not found until it's made.
//! - `image/<content hash>`: the image itself, PNG or TIFF, for the preview.
//!
//! URLs are `kopi://localhost/<path>` on macOS and `http://kopi.localhost/<path>` on Windows.
//! Both name content by hash, so a found response never changes and can be cached for good.

use std::sync::Arc;

use kopi_core::storage::{StorageResult, Store};
use kopi_platform::uti;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

pub const SCHEME: &str = "kopi";

/// Image formats a webview can show, best first.
const IMAGE_FORMATS: &[(&str, &str)] = &[(uti::PNG, "image/png"), (uti::TIFF, "image/tiff")];

/// Answers a request off the main thread, since an image can be large.
pub fn handle<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let store = Arc::clone(&ctx.app_handle().state::<Arc<Store>>());
    tauri::async_runtime::spawn_blocking(move || {
        responder.respond(respond(&store, request.uri().path()));
    });
}

fn respond(store: &Store, path: &str) -> Response<Vec<u8>> {
    let found = match path.trim_start_matches('/').split_once('/') {
        Some(("thumb", hash)) => thumbnail(store, hash),
        Some(("image", hash)) => image(store, hash),
        _ => Ok(None),
    };
    match found {
        Ok(Some((content_type, body))) => Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
            .body(body),
        // A bad hash is an error from the store. Either way there's nothing to show.
        Ok(None) | Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CACHE_CONTROL, "no-store")
            .body(Vec::new()),
    }
    .expect("static headers are valid")
}

fn thumbnail(store: &Store, hash: &str) -> StorageResult<Option<(&'static str, Vec<u8>)>> {
    let Some(path) = store.thumbnail_path(hash)? else {
        return Ok(None);
    };
    Ok(Some(("image/png", std::fs::read(path)?)))
}

fn image(store: &Store, hash: &str) -> StorageResult<Option<(&'static str, Vec<u8>)>> {
    let Some(id) = store.clip_id_by_hash(hash)? else {
        return Ok(None);
    };
    let mut formats = store.clip_formats(id)?;
    for (format, content_type) in IMAGE_FORMATS {
        if let Some(i) = formats.iter().position(|f| f.format == *format) {
            return Ok(Some((content_type, formats.swap_remove(i).data)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use kopi_core::classifier::content_hash;
    use kopi_core::storage::{ClipKind, NewClip, NewFormat, Platform};

    use super::*;

    fn clip(kind: ClipKind, payload: &[u8], format: &str) -> NewClip {
        NewClip {
            kind,
            plain_text: None,
            content_hash: content_hash(kind, payload),
            source_app_name: None,
            source_app_id: None,
            metadata: None,
            platform: Platform::MacOs,
            formats: vec![NewFormat {
                item_index: 0,
                format: format.into(),
                data: payload.to_vec(),
            }],
        }
    }

    fn png() -> Vec<u8> {
        let mut png = Vec::new();
        image::RgbaImage::new(4, 2)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        png
    }

    #[test]
    fn serves_images_and_thumbnails_by_hash() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let store = Store::open_with_listener(
            dir.path(),
            Box::new(move |_| {
                let _ = tx.send(());
            }),
        )
        .unwrap();
        let image_data = png();
        let image = clip(ClipKind::Image, &image_data, uti::PNG);
        let image_hash = image.content_hash.clone();
        store.upsert_clip(image).unwrap();
        rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let text = clip(ClipKind::Text, b"hello", uti::HTML);
        let text_hash = text.content_hash.clone();
        store.upsert_clip(text).unwrap();

        let ok = respond(&store, &format!("/image/{image_hash}"));
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(ok.headers()[header::CONTENT_TYPE], "image/png");
        assert_eq!(ok.body(), &image_data);

        let thumb = respond(&store, &format!("/thumb/{image_hash}"));
        assert_eq!(thumb.status(), StatusCode::OK);
        assert_eq!(thumb.headers()[header::CONTENT_TYPE], "image/png");
        assert!(thumb.body().starts_with(b"\x89PNG"));

        for path in [
            format!("/image/{text_hash}"),
            format!("/thumb/{text_hash}"),
            format!("/image/{}", "0".repeat(64)),
            "/image/../../etc/passwd".to_owned(),
            "/thumb/".to_owned(),
            "/other/x".to_owned(),
            "/".to_owned(),
        ] {
            let missing = respond(&store, &path);
            assert_eq!(missing.status(), StatusCode::NOT_FOUND, "{path}");
            assert_eq!(missing.headers()[header::CACHE_CONTROL], "no-store");
            assert!(missing.body().is_empty());
        }
    }
}

//! `xian://` protocol handler.
//!
//! Maps `xian://...` URLs to local files under a configured web root directory.
//! The root directory is set by the embedder via the C ABI (`xian_web_engine_set_web_root_dir`).
//!
//! Security:
//! - Only supports GET.
//! - Rejects any path that attempts to escape the web root (including `..` and path separator
//!   injection via percent-decoding).

use std::fs::File;
use std::future::{Future, ready};
use std::io::BufReader;
use std::path::PathBuf;
use std::pin::Pin;

use headers::{ContentType, HeaderMapExt};
use http::Method;
use servo::protocol_handler::{
    DoneChannel, FetchContext, NetworkError, ProtocolHandler, RelativePos, Request,
    ResourceFetchTiming, Response, ResponseBody, FILE_CHUNK_SIZE,
};
use tokio::sync::mpsc::unbounded_channel;

#[derive(Default)]
pub(crate) struct XianProtocolHandler;

impl XianProtocolHandler {
    fn invalid(message: &'static str) -> Response {
        Response::network_error(NetworkError::ResourceLoadError(message.to_owned()))
    }

    fn host_looks_like_file(host: &str) -> bool {
        // Heuristic: treat `xian://index.html` as a file path alias for `xian:///index.html`.
        // This keeps `xian://index.html` usable while not breaking relative URLs that resolve to
        // `xian://index.html/<something>`.
        let Some((_, ext)) = host.rsplit_once('.') else {
            return false;
        };
        matches!(
            ext.to_ascii_lowercase().as_str(),
            // HTML
            "html" | "htm" |
            // JS / JSON
            "js" | "mjs" | "json" |
            // CSS
            "css" |
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" |
            // Fonts
            "woff" | "woff2" | "ttf" | "otf"
        )
    }

    fn sanitize_to_relative_path(url: &servo::ServoUrl) -> Result<PathBuf, Response> {
        // We only support the `xian://...` form (two slashes, non-empty host).
        // `xian:///...` (empty host) is rejected on purpose.
        let host = url.host_str().unwrap_or("");
        if host.is_empty() {
            return Err(Self::invalid("Invalid xian:// path"));
        }

        let mut rel = PathBuf::new();

        let path = url.path();
        if !path.is_empty() && !path.starts_with('/') {
            // Reject non-hierarchical forms like `xian:foo` (no //).
            return Err(Self::invalid("Invalid xian:// path"));
        }

        let path_is_empty = path.is_empty() || path == "/";
        let treat_host_as_file = path_is_empty;
        let treat_host_as_dir = !path_is_empty && !Self::host_looks_like_file(host);

        if treat_host_as_file || treat_host_as_dir {
            Self::push_sanitized_segment(&mut rel, host)?;
        }
        // If host looks like a file and we have further path segments (e.g.
        // `xian://index.html/assets/a.css`), ignore the host so relative URLs work.

        let Some(segments) = url.path_segments() else {
            // For non-hierarchical URLs, we only allow empty paths (handled above).
            if !path.is_empty() {
                return Err(Self::invalid("Invalid xian:// path"));
            }
            if rel.as_os_str().is_empty() {
                rel.push("index.html");
            }
            return Ok(rel);
        };

        for seg in segments {
            if seg.is_empty() {
                continue;
            }
            Self::push_sanitized_segment(&mut rel, seg)?;
        }

        if rel.as_os_str().is_empty() {
            rel.push("index.html");
        }

        Ok(rel)
    }

    fn push_sanitized_segment(out: &mut PathBuf, seg: &str) -> Result<(), Response> {
        // Reject traversal and separator injection (including percent-decoded `%2f`/`%5c`).
        if seg == "." || seg == ".." || seg.contains('/') || seg.contains('\\') || seg.contains(':') {
            return Err(Self::invalid("Invalid xian:// path"));
        }

        out.push(seg);
        Ok(())
    }

    fn response_for_file(
        request: &mut Request,
        done_chan: &mut DoneChannel,
        context: &FetchContext,
        file_path: PathBuf,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        if !file_path.exists() || file_path.is_dir() {
            return Box::pin(ready(Self::invalid("Not found")));
        }

        let response = if let Ok(file) = File::open(file_path.clone()) {
            let mut response = Response::new(
                request.current_url(),
                ResourceFetchTiming::new(request.timing_type()),
            );
            let reader = BufReader::with_capacity(FILE_CHUNK_SIZE, file);

            // Set Content-Type header.
            let mime = mime_guess::from_path(&file_path).first_or_octet_stream();
            response.headers.typed_insert(ContentType::from(mime));

            // Setup channel to receive cross-thread messages about the file fetch operation.
            let (mut done_sender, done_receiver) = unbounded_channel();
            *done_chan = Some((done_sender.clone(), done_receiver));

            *response.body.lock() = ResponseBody::Receiving(vec![]);

            context.filemanager.lock().fetch_file_in_chunks(
                &mut done_sender,
                reader,
                response.body.clone(),
                context.cancellation_listener.clone(),
                RelativePos::full_range(),
            );

            response
        } else {
            Response::network_error(NetworkError::ResourceLoadError(
                "Opening file failed".to_owned(),
            ))
        };

        Box::pin(ready(response))
    }

    fn resolve_to_file_path(url: &servo::ServoUrl) -> Result<PathBuf, Response> {
        let Some(root) = crate::engine::web_root_dir() else {
            return Err(Self::invalid("Web root dir is not set"));
        };

        let root_canon = match root.canonicalize() {
            Ok(p) => p,
            Err(_) => return Err(Self::invalid("Web root dir is invalid")),
        };

        let rel = Self::sanitize_to_relative_path(url)?;
        let candidate = root_canon.join(rel);

        let candidate_canon = match candidate.canonicalize() {
            Ok(p) => p,
            Err(_) => return Err(Self::invalid("Not found")),
        };

        // Path traversal / symlink escape protection.
        if !candidate_canon.starts_with(&root_canon) {
            return Err(Self::invalid("Invalid xian:// path"));
        }

        Ok(candidate_canon)
    }
}

impl ProtocolHandler for XianProtocolHandler {
    fn load(
        &self,
        request: &mut Request,
        done_chan: &mut DoneChannel,
        context: &FetchContext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        if request.method != Method::GET {
            return Box::pin(ready(Response::network_error(NetworkError::InvalidMethod)));
        }

        let url = request.current_url();
        let file_path = match Self::resolve_to_file_path(&url) {
            Ok(p) => p,
            Err(resp) => return Box::pin(ready(resp)),
        };

        Self::response_for_file(request, done_chan, context, file_path)
    }
}

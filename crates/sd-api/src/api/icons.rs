//! Application icon lookup.
//!
//! Audio streams report an icon *name* (`"firefox"`), not image bytes — a
//! plugin has no filesystem and shipping pixels across the component boundary
//! on every poll would be wasteful. Resolving that name against the system
//! icon theme is the host's job, and this is where it happens.
//!
//! The client asks for `/api/icon/firefox` and gets a PNG or SVG back, or a
//! 404 it can fall back from. Nothing here is plugin-specific; any widget that
//! knows an icon name can use it.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use axum::{
    extract::Path as AxumPath,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::collections::HashMap;

/// Theme directories searched, in order of preference.
///
/// `hicolor` is the freedesktop fallback theme every application is supposed
/// to install into, which makes it the right place to look for third-party app
/// icons regardless of which theme the desktop is actually using.
fn icon_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(&home).join(".local/share/icons"));
        roots.push(PathBuf::from(&home).join(".icons"));
    }
    roots.push(PathBuf::from("/usr/share/icons"));
    roots.push(PathBuf::from("/usr/local/share/icons"));
    // Flat directory, no theme structure — still where plenty of apps land.
    roots.push(PathBuf::from("/usr/share/pixmaps"));

    roots
}

/// Preferred pixel sizes, largest first.
///
/// A widget may render the icon at any size, and downscaling a large icon
/// looks far better than upscaling a 16px one.
const SIZES: &[&str] = &[
    "scalable", "512x512", "256x256", "192x192", "128x128", "96x96", "64x64", "48x48", "32x32",
    "24x24", "22x22", "16x16",
];

/// Reject anything that is not a plain icon name.
///
/// The name reaches us from an audio stream's proplist, which is attacker-
/// influenced: any application on the machine can set it to whatever it likes.
/// Without this, `../../etc/passwd` would be a path traversal straight out of
/// the icon directories.
fn sanitize(name: &str) -> Option<String> {
    if name.is_empty() || name.len() > 128 {
        return None;
    }
    let ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'));
    // A leading dot would allow `.` and `..`; interior dots are legitimate,
    // since reverse-DNS icon names like `com.obsproject.Studio` are common.
    if !ok || name.starts_with('.') {
        return None;
    }
    Some(name.to_string())
}

/// Resolved paths, keyed by icon name.
///
/// Icon themes do not change while the process runs, and a widget polling
/// several streams would otherwise walk the same directories every refresh.
/// Negative results are cached too — a missing icon is the common case for
/// obscure applications, and re-walking the tree to rediscover that is the
/// expensive path.
fn cache() -> &'static Mutex<HashMap<String, Option<PathBuf>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<PathBuf>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn first_existing(dir: &Path, name: &str) -> Option<PathBuf> {
    for ext in ["svg", "png", "xpm"] {
        let candidate = dir.join(format!("{name}.{ext}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Find the best icon file for `name`, or `None`.
fn resolve(name: &str) -> Option<PathBuf> {
    if let Some(hit) = cache().lock().ok().and_then(|c| c.get(name).cloned()) {
        return hit;
    }

    let found = search(name);

    if let Ok(mut c) = cache().lock() {
        c.insert(name.to_string(), found.clone());
    }
    found
}

fn search(name: &str) -> Option<PathBuf> {
    for root in icon_roots() {
        if !root.is_dir() {
            continue;
        }

        // Flat directory such as /usr/share/pixmaps.
        if let Some(hit) = first_existing(&root, name) {
            return Some(hit);
        }

        // Theme layout: <theme>/<size>/apps/<name>.<ext>. `hicolor` first,
        // then any other theme present, so a themed icon still wins over
        // nothing at all.
        let mut themes: Vec<PathBuf> = vec![root.join("hicolor")];
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.file_name().is_some_and(|n| n != "hicolor") {
                    themes.push(path);
                }
            }
        }

        for theme in themes {
            if !theme.is_dir() {
                continue;
            }
            for size in SIZES {
                for category in ["apps", "devices", "categories", "status"] {
                    let dir = theme.join(size).join(category);
                    if let Some(hit) = first_existing(&dir, name) {
                        return Some(hit);
                    }
                }
            }
        }
    }
    None
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("svg") => "image/svg+xml",
        Some("xpm") => "image/x-xpixmap",
        _ => "image/png",
    }
}

/// `GET /api/icon/:name`
///
/// Returns the icon, or 404 when the name is unknown or unsafe. A 404 is a
/// normal outcome the client renders a fallback for, not an error worth
/// logging loudly.
pub(crate) async fn get_icon(AxumPath(name): AxumPath<String>) -> Response {
    let Some(name) = sanitize(&name) else {
        return (StatusCode::BAD_REQUEST, "invalid icon name").into_response();
    };

    let Some(path) = resolve(&name) else {
        return (StatusCode::NOT_FOUND, "no such icon").into_response();
    };

    match std::fs::read(&path) {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type(&path)),
                // Icons are effectively immutable for the life of the process.
                (header::CACHE_CONTROL, "public, max-age=86400"),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "icon unreadable").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_icon_names() {
        assert_eq!(sanitize("firefox").as_deref(), Some("firefox"));
        assert_eq!(sanitize("web-browser").as_deref(), Some("web-browser"));
        assert_eq!(sanitize("audio_card").as_deref(), Some("audio_card"));
        // Reverse-DNS names are common for Flatpak-style apps.
        assert_eq!(
            sanitize("com.obsproject.Studio").as_deref(),
            Some("com.obsproject.Studio")
        );
    }

    /// The name comes from an audio stream's proplist, which any local
    /// application can set. Traversal must be impossible.
    #[test]
    fn rejects_path_traversal() {
        assert!(sanitize("../../etc/passwd").is_none());
        assert!(sanitize("..").is_none());
        assert!(sanitize(".").is_none());
        assert!(sanitize(".hidden").is_none());
        assert!(sanitize("foo/bar").is_none());
        assert!(sanitize("foo\\bar").is_none());
        assert!(sanitize("/absolute").is_none());
    }

    #[test]
    fn rejects_empty_and_oversized_names() {
        assert!(sanitize("").is_none());
        assert!(sanitize(&"a".repeat(129)).is_none());
    }

    #[test]
    fn rejects_shell_and_null_bytes() {
        assert!(sanitize("foo;rm -rf /").is_none());
        assert!(sanitize("foo\0bar").is_none());
        assert!(sanitize("foo bar").is_none());
    }

    #[test]
    fn maps_extensions_to_content_types() {
        assert_eq!(content_type(Path::new("a.svg")), "image/svg+xml");
        assert_eq!(content_type(Path::new("a.png")), "image/png");
        assert_eq!(content_type(Path::new("a.xpm")), "image/x-xpixmap");
    }

    /// Prefers large icons, so a widget can downscale rather than upscale.
    #[test]
    fn size_preference_runs_largest_first() {
        let numeric: Vec<usize> = SIZES
            .iter()
            .filter_map(|s| s.split('x').next()?.parse().ok())
            .collect();
        let mut sorted = numeric.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(numeric, sorted);
        assert_eq!(SIZES[0], "scalable");
    }
}

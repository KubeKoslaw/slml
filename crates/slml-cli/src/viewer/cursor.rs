//! Mouse cursors on Linux and the BSDs.
//!
//! minifb's Wayland backend loads the cursor theme named by `XCURSOR_THEME`,
//! or a theme literally called `default`, at the size in `XCURSOR_SIZE`
//! (16 when unset). Many desktops set neither variable (KDE Plasma keeps its
//! theme in `kcminputrc`) and most systems have no theme called `default`.
//! Every cursor lookup then fails, the pointer surface stays empty and the
//! cursor is invisible over the window. [`prepare`] points both variables at
//! the theme the desktop really uses before the window opens.
//!
//! The same backend draws each cursor image with its top-left corner at the
//! pointer, ignoring the image's hotspot (the tip of the arrow, the middle of
//! the I-beam). [`Hotspots`] reads the hotspots from the theme so the viewer
//! can act on the spot the user sees instead of the spot the system reports.

use minifb::CursorStyle;

/// How far the visible hot point of each cursor image is from where the
/// window system says the pointer is, in window pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Hotspots {
    arrow: (f32, f32),
    ibeam: (f32, f32),
    hand: (f32, f32),
    crosshair: (f32, f32),
}

impl Hotspots {
    pub fn offset(&self, style: CursorStyle) -> (f32, f32) {
        match style {
            CursorStyle::Ibeam => self.ibeam,
            CursorStyle::ClosedHand | CursorStyle::OpenHand => self.hand,
            CursorStyle::Crosshair => self.crosshair,
            _ => self.arrow,
        }
    }
}

/// Sets up the cursor theme for the window that is about to open and returns
/// the offsets to correct for. Must run before any other thread starts.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn prepare() -> Hotspots {
    use std::env;

    let paths = theme::search_paths();
    let set = |key: &str| env::var(key).ok().filter(|v| !v.is_empty());
    let configured = theme::desktop_settings();
    let name = set("XCURSOR_THEME")
        .into_iter()
        .chain(configured.theme)
        .chain(["default"].map(String::from))
        .chain(theme::COMMON.map(String::from))
        .find(|t| theme::find(t, "arrow", &paths).is_some());
    let size = set("XCURSOR_SIZE")
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|s| *s > 0)
        .or(configured.size)
        .unwrap_or(24);
    // SAFETY: the viewer calls this on the main thread before it opens the
    // window or starts any thread, so nothing reads the environment
    // concurrently.
    unsafe {
        if let Some(name) = &name
            && set("XCURSOR_THEME").as_ref() != Some(name)
        {
            env::set_var("XCURSOR_THEME", name);
        }
        if set("XCURSOR_SIZE").is_none() {
            env::set_var("XCURSOR_SIZE", size.to_string());
        }
    }

    // Only the Wayland backend ignores hotspots; X11 applies them itself.
    let wayland = set("WAYLAND_DISPLAY").is_some() || env::var_os("WAYLAND_SOCKET").is_some();
    let Some(name) = name.filter(|_| wayland) else {
        return Hotspots::default();
    };
    let hot = |cursor: &str| {
        theme::find(&name, cursor, &paths)
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|data| hotspot(&data, size))
            .unwrap_or((0.0, 0.0))
    };
    Hotspots {
        arrow: hot("arrow"),
        ibeam: hot("xterm"),
        hand: hot("hand2"),
        crosshair: hot("crosshair"),
    }
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
pub fn prepare() -> Hotspots {
    Hotspots::default()
}

/// The hotspot of the image an Xcursor file shows at `size`: the image whose
/// nominal size is nearest, the first one on ties (what `wayland-cursor`
/// picks).
fn hotspot(data: &[u8], size: u32) -> Option<(f32, f32)> {
    const IMAGE: u32 = 0xfffd_0002;
    let word = |at: usize| -> Option<u32> {
        let bytes = data.get(at..at + 4)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?))
    };
    if data.get(..4)? != b"Xcur" {
        return None;
    }
    let header = word(4)? as usize;
    let entries = word(12)? as usize;
    let mut best: Option<(u32, (f32, f32))> = None;
    for i in 0..entries.min(4096) {
        let entry = header + 12 * i;
        if word(entry)? != IMAGE {
            continue;
        }
        let nominal = word(entry + 4)?;
        let chunk = word(entry + 8)? as usize;
        // Chunk: header size, type, nominal size, version, width, height,
        // xhot, yhot, delay, pixels.
        let hot = (word(chunk + 24)? as f32, word(chunk + 28)? as f32);
        let distance = nominal.abs_diff(size);
        if best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, hot));
        }
    }
    best.map(|(_, hot)| hot)
}

/// Finding Xcursor themes the way libXcursor and `wayland-cursor` do.
#[cfg(all(unix, not(target_os = "macos")))]
mod theme {
    use std::env;
    use std::path::{Path, PathBuf};

    /// Widespread themes to fall back on when the desktop names none.
    pub const COMMON: [&str; 6] = [
        "breeze_cursors",
        "Adwaita",
        "Breeze_Light",
        "DMZ-White",
        "Vanilla-DMZ",
        "whiteglass",
    ];

    pub struct Settings {
        pub theme: Option<String>,
        pub size: Option<u32>,
    }

    /// The cursor theme and size chosen in KDE Plasma or GTK settings.
    pub fn desktop_settings() -> Settings {
        let config = env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
        let mut settings = Settings {
            theme: None,
            size: None,
        };
        let Some(config) = config else {
            return settings;
        };
        let files = [
            ("kcminputrc", "cursorTheme", "cursorSize"),
            (
                "gtk-4.0/settings.ini",
                "gtk-cursor-theme-name",
                "gtk-cursor-theme-size",
            ),
            (
                "gtk-3.0/settings.ini",
                "gtk-cursor-theme-name",
                "gtk-cursor-theme-size",
            ),
        ];
        for (file, theme_key, size_key) in files {
            let Ok(text) = std::fs::read_to_string(config.join(file)) else {
                continue;
            };
            if settings.theme.is_none() {
                settings.theme = value(&text, theme_key).map(str::to_string);
            }
            if settings.size.is_none() {
                settings.size = value(&text, size_key).and_then(|s| s.parse().ok());
            }
        }
        settings
    }

    /// `key=value` from an INI file, ignoring sections.
    fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
        text.lines().find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key)
                .then(|| v.trim().trim_matches('"'))
                .filter(|v| !v.is_empty())
        })
    }

    pub fn search_paths() -> Vec<PathBuf> {
        let var = |key: &str| env::var(key).ok().filter(|v| !v.is_empty());
        let home = var("HOME").map(PathBuf::from);
        let expand = |entry: &str| match (entry.strip_prefix("~/"), &home) {
            (Some(rest), Some(home)) => Some(home.join(rest)),
            (Some(_), None) => None,
            (None, _) => Some(PathBuf::from(entry)),
        };
        if let Some(list) = var("XCURSOR_PATH") {
            return list
                .split(':')
                .filter(|e| !e.is_empty())
                .filter_map(expand)
                .collect();
        }
        let mut paths = Vec::new();
        match (var("XDG_DATA_HOME"), &home) {
            (Some(data), _) => paths.extend(expand(&data)),
            (None, Some(home)) => paths.push(home.join(".local/share/icons")),
            (None, None) => {}
        }
        if let Some(home) = &home {
            paths.push(home.join(".icons"));
        }
        let dirs = var("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
        paths.extend(
            dirs.split(':')
                .filter(|e| !e.is_empty())
                .filter_map(expand)
                .map(|d| d.join("icons")),
        );
        paths.push(PathBuf::from("/usr/share/pixmaps"));
        if let Some(home) = &home {
            paths.push(home.join(".cursors"));
        }
        paths.push(PathBuf::from("/usr/share/cursors/xorg-x11"));
        paths
    }

    /// The file for `cursor` in `theme` or a theme it inherits from.
    pub fn find(theme: &str, cursor: &str, paths: &[PathBuf]) -> Option<PathBuf> {
        let mut visited = Vec::new();
        find_in(theme, cursor, paths, &mut visited)
    }

    fn find_in(
        theme: &str,
        cursor: &str,
        paths: &[PathBuf],
        visited: &mut Vec<String>,
    ) -> Option<PathBuf> {
        if visited.iter().any(|t| t == theme) || visited.len() > 16 {
            return None;
        }
        visited.push(theme.to_string());
        let dirs: Vec<PathBuf> = paths
            .iter()
            .map(|p| p.join(theme))
            .filter(|d| d.is_dir())
            .collect();
        if let Some(file) = dirs
            .iter()
            .map(|d| d.join("cursors").join(cursor))
            .find(|f| f.is_file())
        {
            return Some(file);
        }
        dirs.iter()
            .flat_map(|d| inherits(&d.join("index.theme"), theme))
            .find_map(|parent| find_in(&parent, cursor, paths, visited))
    }

    /// Themes named by `Inherits=`; themes without it inherit `default`.
    fn inherits(index: &Path, theme: &str) -> Vec<String> {
        let listed = std::fs::read_to_string(index)
            .ok()
            .and_then(|text| value(&text, "Inherits").map(str::to_string));
        match listed {
            Some(list) => list
                .split([',', ';'])
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(String::from)
                .collect(),
            None if theme != "default" => vec!["default".into()],
            None => Vec::new(),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn themes_are_found_through_inherits() {
            let dir = std::env::temp_dir().join(format!("slml-cursors-{}", std::process::id()));
            let base = dir.join("base/cursors");
            std::fs::create_dir_all(&base).unwrap();
            std::fs::write(base.join("arrow"), b"Xcur").unwrap();
            std::fs::create_dir_all(dir.join("child")).unwrap();
            std::fs::write(
                dir.join("child/index.theme"),
                "[Icon Theme]\nName=Child\nInherits=missing, base\n",
            )
            .unwrap();
            let paths = vec![dir.clone()];
            assert_eq!(
                find("child", "arrow", &paths),
                Some(base.join("arrow")),
                "inherited theme"
            );
            assert_eq!(find("child", "xterm", &paths), None);
            assert_eq!(find("nowhere", "arrow", &paths), None);
            std::fs::remove_dir_all(dir).unwrap();
        }

        #[test]
        fn ini_values_ignore_spaces_and_quotes() {
            let text = "[Mouse]\ncursorTheme = breeze_cursors\ngtk-cursor-theme-name=\"Adwaita\"\nempty=\n";
            assert_eq!(value(text, "cursorTheme"), Some("breeze_cursors"));
            assert_eq!(value(text, "gtk-cursor-theme-name"), Some("Adwaita"));
            assert_eq!(value(text, "empty"), None);
            assert_eq!(value(text, "missing"), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An Xcursor file with one image chunk per (nominal size, hotspot).
    fn xcursor(images: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut out = Vec::new();
        let words = |out: &mut Vec<u8>, ws: &[u32]| {
            for w in ws {
                out.extend_from_slice(&w.to_le_bytes());
            }
        };
        out.extend_from_slice(b"Xcur");
        words(&mut out, &[16, 0x1_0000, images.len() as u32]);
        let first = 16 + 12 * images.len() as u32;
        for (i, (size, _, _)) in images.iter().enumerate() {
            words(&mut out, &[0xfffd_0002, *size, first + 36 * i as u32]);
        }
        for (size, x, y) in images {
            words(&mut out, &[36, 0xfffd_0002, *size, 1, 1, 1, *x, *y, 0]);
        }
        out
    }

    #[test]
    fn hotspot_comes_from_the_nearest_size() {
        let file = xcursor(&[(24, 11, 12), (32, 16, 15), (48, 22, 23)]);
        assert_eq!(hotspot(&file, 24), Some((11.0, 12.0)));
        assert_eq!(hotspot(&file, 30), Some((16.0, 15.0)));
        assert_eq!(hotspot(&file, 96), Some((22.0, 23.0)));
        assert_eq!(hotspot(&file, 28), Some((11.0, 12.0)), "first on ties");
        assert_eq!(hotspot(b"nope", 24), None);
        assert_eq!(hotspot(&file[..40], 24), None, "truncated file");
    }
}

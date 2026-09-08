use std::time::{Duration, Instant};

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use super::types::NotificationType;
use crate::config::NotificationPosition;

/// How long an info toast stays on screen.
const INFO_DURATION: Duration = Duration::from_millis(4000);
/// Errors stay longer so there is time to read them.
const ERROR_DURATION: Duration = Duration::from_millis(6000);
/// Maximum toasts kept in memory (oldest is dropped first).
pub const MAX_TOASTS: usize = 4;
/// Maximum toasts painted at once (newest first).
pub const MAX_VISIBLE_TOASTS: usize = 3;
/// Toast width bounds in cells (before clamping to the screen).
pub const MIN_TOAST_WIDTH: u16 = 20;
pub const MAX_TOAST_WIDTH: u16 = 54;
/// Maximum wrapped message lines per toast.
pub const MAX_TOAST_LINES: usize = 4;

/// A single floating notification.
#[derive(Clone, Debug)]
pub struct Toast {
    pub message: String,
    pub kind: NotificationType,
    pub created: Instant,
    pub duration: Duration,
}

impl Toast {
    pub fn new(message: String, kind: NotificationType) -> Self {
        let duration = match kind {
            NotificationType::Info => INFO_DURATION,
            NotificationType::Error => ERROR_DURATION,
        };
        Self {
            message,
            kind,
            created: Instant::now(),
            duration,
        }
    }

    /// Fraction of lifetime elapsed, 0.0 (just born) ..= 1.0 (expired).
    pub fn elapsed_frac(&self) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        (self.created.elapsed().as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Fraction of lifetime remaining, for the draining progress bar.
    pub fn remaining_frac(&self) -> f32 {
        1.0 - self.elapsed_frac()
    }

    pub fn is_expired(&self) -> bool {
        self.elapsed_frac() >= 1.0
    }
}

/// Title shown in the toast border, e.g. " ● Info ".
pub fn toast_title(kind: NotificationType) -> String {
    match kind {
        NotificationType::Info => " ● Info ".to_string(),
        NotificationType::Error => " ● Error ".to_string(),
    }
}

/// Screen margins (horizontal, vertical) for the toast stack.
/// Narrow terminals get tighter margins so toasts still fit.
pub(crate) fn toast_margins(area_width: u16) -> (u16, u16) {
    (if area_width < 48 { 1 } else { 2 }, 1)
}

/// Clamp a content-derived toast width to the screen.
/// Never panics, even on degenerate terminal sizes.
pub(crate) fn clamp_toast_width(area_width: u16, natural: u16) -> u16 {
    let (margin_x, _) = toast_margins(area_width);
    // `8 <= MAX_TOAST_WIDTH`, so a direct clamp cannot panic here.
    let avail = area_width
        .saturating_sub(margin_x * 2)
        .clamp(8, MAX_TOAST_WIDTH);
    natural.min(avail).max(avail.min(MIN_TOAST_WIDTH))
}

/// Longest prefix of `s` fitting `width` cells.
pub(crate) fn truncate_cells(s: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for c in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
        if used + cw > width {
            break;
        }
        out.push(c);
        used += cw;
    }
    out
}

/// Greedy word wrap by display width (CJK-aware via `unicode-width`).
/// Hard-splits words longer than `width`, expands tabs, preserves newlines,
/// and trims leading whitespace per line (same as `Wrap { trim: true }`
/// used by the other popups). Always returns at least one line.
pub fn wrap_cells(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for logical in text.split('\n') {
        let logical = logical.replace('\t', "    ");
        let logical = logical.trim_start_matches(' ');
        if logical.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut line_w = 0;
        for word in logical.split(' ') {
            let word_w = UnicodeWidthStr::width(word);
            if word_w == 0 {
                // Consecutive spaces: keep one if the line is non-empty.
                if !line.is_empty() {
                    line.push(' ');
                    line_w += 1;
                }
                continue;
            }
            if word_w > width {
                // Overlong word: hard-split it across lines.
                if !line.is_empty() {
                    out.push(std::mem::take(&mut line));
                }
                let mut chunk = String::new();
                let mut chunk_w = 0;
                for c in word.chars() {
                    let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
                    if chunk_w + cw > width {
                        out.push(std::mem::take(&mut chunk));
                        chunk_w = 0;
                    }
                    chunk.push(c);
                    chunk_w += cw;
                }
                line = chunk;
                line_w = chunk_w;
                continue;
            }
            let sep = if line.is_empty() { 0 } else { 1 };
            if line_w + sep + word_w > width {
                out.push(std::mem::take(&mut line));
                line_w = 0;
            } else if !line.is_empty() {
                line.push(' ');
                line_w += 1;
            }
            line.push_str(word);
            line_w += word_w;
        }
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Minimum toast width that still fits the top-border close button
/// next to the title without overlapping it.
pub const MIN_CLOSE_BUTTON_WIDTH: u16 = 12;

/// A visible toast with everything the renderer and hit-testing need.
#[derive(Clone, Debug)]
pub(crate) struct ToastLayout {
    /// Index into the source slice (chronological order).
    pub source_idx: usize,
    pub rect: Rect,
    /// Inner content width in cells (rect minus borders and padding).
    pub inner: usize,
    /// Wrapped message lines (already capped).
    pub body: Vec<String>,
}

/// Filter expired toasts (newest first), compute the uniform width, wrap
/// bodies and lay out rectangles. Single source of truth shared by painting
/// and close-button hit-testing, so clicks always match what is on screen.
pub(crate) fn layout_visible_toasts(
    notifications: &[Toast],
    area: Rect,
    pos: NotificationPosition,
) -> Vec<ToastLayout> {
    let live: Vec<(usize, &Toast)> = notifications
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, toast)| !toast.is_expired())
        .take(MAX_VISIBLE_TOASTS)
        .collect();
    if live.is_empty() || area.width < 12 || area.height < 6 {
        return Vec::new();
    }

    // Uniform width from the widest content (title or longest raw line).
    let natural = live
        .iter()
        .map(|(_, toast)| {
            let title_w = UnicodeWidthStr::width(toast_title(toast.kind).as_str());
            let msg_w = toast
                .message
                .split('\n')
                .map(UnicodeWidthStr::width)
                .max()
                .unwrap_or(0);
            // Borders (2) + horizontal padding (2).
            title_w.max(msg_w).saturating_add(4) as u16
        })
        .max()
        .unwrap_or(MIN_TOAST_WIDTH);
    let width = clamp_toast_width(area.width, natural);
    let inner = (width as usize).saturating_sub(4).max(1);

    let mut bodies: Vec<Vec<String>> = Vec::with_capacity(live.len());
    let mut heights: Vec<u16> = Vec::with_capacity(live.len());
    for (_, toast) in &live {
        let mut lines = wrap_cells(&toast.message, inner);
        if lines.len() > MAX_TOAST_LINES {
            lines.truncate(MAX_TOAST_LINES);
            if let Some(last) = lines.last_mut() {
                let truncated = truncate_cells(last, inner.saturating_sub(1));
                *last = format!("{truncated}…");
            }
        }
        heights.push(lines.len().saturating_add(3) as u16);
        bodies.push(lines);
    }

    layout_toasts(area, pos, width, &heights)
        .into_iter()
        .zip(bodies)
        .enumerate()
        .filter_map(|(i, (rect, body))| {
            // Skip degenerate rects the painter would skip too.
            if rect.width < 3 || rect.height < 4 {
                return None;
            }
            Some(ToastLayout {
                source_idx: live[i].0,
                rect,
                inner,
                body,
            })
        })
        .collect()
}

/// Close-button cell of a laid-out toast: 1x1 on the top border, just left
/// of the rounded corner. `None` when the toast is too narrow for title +
/// button to coexist.
pub(crate) fn toast_close_cell(layout: &ToastLayout) -> Option<(u16, u16)> {
    if layout.rect.width < MIN_CLOSE_BUTTON_WIDTH {
        return None;
    }
    Some((
        layout
            .rect
            .x
            .saturating_add(layout.rect.width)
            .saturating_sub(2),
        layout.rect.y,
    ))
}

/// Index (into `notifications`) of the toast whose close button sits at
/// (`col`, `row`), if any.
pub(crate) fn toast_close_hit(
    notifications: &[Toast],
    area: Rect,
    pos: NotificationPosition,
    col: u16,
    row: u16,
) -> Option<usize> {
    layout_visible_toasts(notifications, area, pos)
        .iter()
        .find_map(|layout| {
            if toast_close_cell(layout) == Some((col, row)) {
                Some(layout.source_idx)
            } else {
                None
            }
        })
}

/// Compute overlay rectangles for stacked toasts.
///
/// `heights` are the outer heights (including borders) in paint order
/// (newest first). The newest toast hugs the corner edge; older ones stack
/// behind it with a 1-row gap. Toasts that would overflow the opposite
/// margin are dropped. Width is clamped to fit `area`.
pub fn layout_toasts(
    area: Rect,
    pos: NotificationPosition,
    width: u16,
    heights: &[u16],
) -> Vec<Rect> {
    if area.width == 0 || area.height == 0 || heights.is_empty() {
        return Vec::new();
    }
    let (margin_x, margin_y) = toast_margins(area.width);
    let width = width
        .min(area.width.saturating_sub(margin_x * 2))
        .max(1)
        .min(area.width)
        .max(1);
    if width == 0 {
        return Vec::new();
    }
    let x = match pos {
        NotificationPosition::TopLeft | NotificationPosition::BottomLeft => {
            area.x.saturating_add(margin_x)
        }
        NotificationPosition::TopRight | NotificationPosition::BottomRight => area
            .x
            .saturating_add(area.width)
            .saturating_sub(margin_x)
            .saturating_sub(width),
    };
    // Guard against overflow when the screen is narrower than margins.
    let x = x.min(area.x.saturating_add(area.width).saturating_sub(width));

    let mut rects = Vec::new();
    match pos {
        NotificationPosition::TopLeft | NotificationPosition::TopRight => {
            let mut y = area.y.saturating_add(margin_y);
            let bottom = area.y.saturating_add(area.height).saturating_sub(margin_y);
            for &h in heights {
                let h = h.max(1).min(area.height);
                if y.saturating_add(h) > bottom {
                    break;
                }
                rects.push(Rect::new(x, y, width, h));
                y = y.saturating_add(h).saturating_add(1);
            }
        }
        NotificationPosition::BottomLeft | NotificationPosition::BottomRight => {
            let mut bottom = area.y.saturating_add(area.height).saturating_sub(margin_y);
            let top = area.y.saturating_add(margin_y);
            for &h in heights {
                let h = h.max(1).min(area.height);
                if bottom < top.saturating_add(h) {
                    break;
                }
                let y = bottom.saturating_sub(h);
                rects.push(Rect::new(x, y, width, h));
                bottom = y.saturating_sub(1);
            }
        }
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn toast_with_age(kind: NotificationType, age: Duration, duration: Duration) -> Toast {
        Toast {
            message: "msg".to_string(),
            kind,
            created: Instant::now() - age,
            duration,
        }
    }

    #[test]
    fn progress_goes_from_full_to_empty() {
        let fresh = toast_with_age(
            NotificationType::Info,
            Duration::ZERO,
            Duration::from_secs(4),
        );
        assert!((fresh.remaining_frac() - 1.0).abs() < 0.05);
        assert!(!fresh.is_expired());

        let half = toast_with_age(
            NotificationType::Info,
            Duration::from_secs(2),
            Duration::from_secs(4),
        );
        assert!((half.remaining_frac() - 0.5).abs() < 0.1);

        let old = toast_with_age(
            NotificationType::Info,
            Duration::from_secs(10),
            Duration::from_secs(4),
        );
        assert_eq!(old.remaining_frac(), 0.0);
        assert!(old.is_expired());
    }

    #[test]
    fn error_toasts_live_longer_than_info() {
        let info = Toast::new("i".to_string(), NotificationType::Info);
        let error = Toast::new("e".to_string(), NotificationType::Error);
        assert!(error.duration > info.duration);
    }

    #[test]
    fn wrap_basic_and_newlines() {
        assert_eq!(wrap_cells("hello world", 20), vec!["hello world"]);
        assert_eq!(wrap_cells("hello world", 5), vec!["hello", "world"]);
        assert_eq!(wrap_cells("a\nb", 20), vec!["a", "b"]);
        assert_eq!(wrap_cells("", 20), vec![""]);
    }

    #[test]
    fn wrap_splits_overlong_words_and_counts_wide_chars() {
        assert_eq!(wrap_cells("abcdefgh", 3), vec!["abc", "def", "gh"]);
        // CJK chars are 2 cells wide: "日本語" is 6 cells.
        assert_eq!(wrap_cells("日本語x", 4), vec!["日本", "語x"]);
        // Tabs expand and leading whitespace is trimmed like other popups.
        assert_eq!(wrap_cells("\thi", 6), vec!["hi"]);
        assert_eq!(wrap_cells("  padded", 20), vec!["padded"]);
    }

    #[test]
    fn layout_bottom_right_hugs_corner() {
        let area = Rect::new(0, 0, 80, 24);
        let rects = layout_toasts(area, NotificationPosition::BottomRight, 20, &[4, 4]);
        assert_eq!(rects.len(), 2);
        // Newest toast sits at the very bottom-right (2-col, 1-row margin).
        assert_eq!(rects[0], Rect::new(58, 19, 20, 4));
        // Older one stacks above with a 1-row gap.
        assert_eq!(rects[1], Rect::new(58, 14, 20, 4));
    }

    #[test]
    fn layout_top_left_stacks_downward() {
        let area = Rect::new(0, 0, 80, 24);
        let rects = layout_toasts(area, NotificationPosition::TopLeft, 20, &[4, 4]);
        assert_eq!(rects, vec![Rect::new(2, 1, 20, 4), Rect::new(2, 6, 20, 4)]);
    }

    #[test]
    fn layout_drops_toasts_that_overflow() {
        let area = Rect::new(0, 0, 80, 12);
        // Usable rows are 1..=10. Two 4-row toasts + gap need 9 rows; a
        // third would need 14, so it is dropped.
        let rects = layout_toasts(area, NotificationPosition::BottomRight, 20, &[4, 4, 4]);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0], Rect::new(58, 7, 20, 4));
        assert_eq!(rects[1], Rect::new(58, 2, 20, 4));
    }

    #[test]
    fn toast_width_clamps_without_panicking() {
        // Wide screen: content width bounded by min/max.
        assert_eq!(clamp_toast_width(80, 5), MIN_TOAST_WIDTH);
        assert_eq!(clamp_toast_width(80, 30), 30);
        assert_eq!(clamp_toast_width(80, 500), MAX_TOAST_WIDTH);
        // Narrow screen: shrinks below the minimum, never below 8.
        assert_eq!(clamp_toast_width(20, 30), 18);
        assert_eq!(clamp_toast_width(10, 30), 8);
        assert_eq!(clamp_toast_width(0, 30), 8);
    }

    #[test]
    fn truncate_cells_respects_display_width() {
        assert_eq!(truncate_cells("hello", 3), "hel");
        assert_eq!(truncate_cells("日本語", 4), "日本");
        assert_eq!(truncate_cells("hi", 10), "hi");
    }

    #[test]
    fn close_hit_maps_cells_to_newest_first_indices() {
        use crate::config::NotificationPosition::BottomRight;
        let area = Rect::new(0, 0, 80, 24);
        let toasts = vec![
            Toast::new("first".to_string(), NotificationType::Info),
            Toast::new("second".to_string(), NotificationType::Info),
        ];
        // "Hi"-sized toasts: newest hugs the corner, older stacks above.
        assert_eq!(toast_close_hit(&toasts, area, BottomRight, 76, 19), Some(1));
        assert_eq!(toast_close_hit(&toasts, area, BottomRight, 76, 14), Some(0));
        // Title text and empty space are not the button.
        assert_eq!(toast_close_hit(&toasts, area, BottomRight, 60, 19), None);
        assert_eq!(toast_close_hit(&toasts, area, BottomRight, 0, 0), None);
        // No button on toasts too narrow for title + button.
        let narrow = Rect::new(0, 0, 0, 0);
        assert_eq!(toast_close_hit(&toasts, narrow, BottomRight, 0, 0), None);
    }

    #[test]
    fn layout_clamps_to_tiny_screens() {
        let area = Rect::new(0, 0, 20, 8);
        let rects = layout_toasts(area, NotificationPosition::TopRight, 54, &[4]);
        assert_eq!(rects.len(), 1);
        let r = rects[0];
        assert!(r.x + r.width <= area.width);
        assert!(r.y + r.height <= area.height);
        assert!(r.width >= 8);
    }
}

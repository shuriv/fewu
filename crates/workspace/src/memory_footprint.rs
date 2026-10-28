use std::time::Duration;

use gpui::{App, Context, Render, Task, Window};
use settings::Settings;
use ui::{Color, Label, LabelSize, prelude::*};

use crate::{
    HideStatusItem, StatusItemView, item::ItemHandle, workspace_settings::StatusBarSettings,
};

/// How often the process resident set size is re-read.
const POLL_INTERVAL: Duration = Duration::from_secs(5);
/// Resident set size, in mebibytes, at which the indicator turns amber.
const WARN_MEGABYTES: u64 = 350;
/// Resident set size, in mebibytes, at which the indicator turns red.
const CRITICAL_MEGABYTES: u64 = 500;

/// A status-bar item showing the current process resident set size.
///
/// fewu targets 8 GB machines, so keeping the real memory footprint visible
/// while working is useful. The value is read straight from the OS rather than
/// through a process-inspection crate.
pub struct MemoryFootprintItem {
    resident_megabytes: Option<u64>,
    _poll: Task<()>,
}

impl MemoryFootprintItem {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let resident_megabytes = current_resident_megabytes();
        let poll = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(POLL_INTERVAL).await;
            let latest = current_resident_megabytes();
            let updated = this.update(cx, |item, cx| {
                if item.resident_megabytes != latest {
                    item.resident_megabytes = latest;
                    cx.notify();
                }
            });
            if updated.is_err() {
                break;
            }
        });

        Self {
            resident_megabytes,
            _poll: poll,
        }
    }
}

impl Render for MemoryFootprintItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let item = div();
        if !StatusBarSettings::get_global(cx).memory_footprint_button {
            return item;
        }

        let Some(resident_megabytes) = self.resident_megabytes else {
            return item;
        };

        let color = if resident_megabytes >= CRITICAL_MEGABYTES {
            Color::Error
        } else if resident_megabytes >= WARN_MEGABYTES {
            Color::Warning
        } else {
            Color::Muted
        };

        item.child(
            Label::new(format!("RAM: {resident_megabytes} MB"))
                .size(LabelSize::Small)
                .color(color),
        )
    }
}

impl StatusItemView for MemoryFootprintItem {
    fn set_active_pane_item(
        &mut self,
        _active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    fn hide_setting(&self, _: &App) -> Option<HideStatusItem> {
        Some(HideStatusItem::new(|settings| {
            settings
                .status_bar
                .get_or_insert_default()
                .memory_footprint_button = Some(false);
        }))
    }
}

/// The current process resident set size in mebibytes, if the platform exposes
/// it. Only Linux is supported for now; elsewhere the indicator stays hidden.
#[cfg(target_os = "linux")]
fn current_resident_megabytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let kilobytes = status.lines().find_map(|line| {
        line.strip_prefix("VmRSS:")?
            .split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()
    })?;
    Some(kilobytes / 1024)
}

#[cfg(not(target_os = "linux"))]
fn current_resident_megabytes() -> Option<u64> {
    None
}

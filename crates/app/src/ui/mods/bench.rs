//! `--selftest`: the load order scrolled from top to bottom in ten seconds, as `ModsChecks.qml`
//! did. Each frame moves the list and draws the whole Mods view again: harder than the wheel,
//! whose scroll gpui-fast draws without the view.

use super::Mods;
use gpui::{Context, Window, point, px};
use std::time::{Duration, Instant};

const LENGTH: Duration = Duration::from_secs(10);
/// A frame slower than this counts as slow, as the Qt check counted it.
const SLOW: Duration = Duration::from_millis(20);

pub struct Bench {
    started: Instant,
    last: Instant,
    frames: usize,
    slow: usize,
    worst: Duration,
    cpu: Duration,
}

/// The time this process has run on the processors, user and system.
fn cpu_time() -> Duration {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let fields: Vec<&str> = stat
        .rsplit(')')
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    let ticks = |at: usize| {
        fields
            .get(at)
            .and_then(|t| t.parse::<u64>().ok())
            .unwrap_or(0)
    };
    // Fields 14 and 15 of `proc(5)`, counted after the name; ticks of 1/100 s on Linux.
    Duration::from_millis((ticks(11) + ticks(12)) * 10)
}

impl Mods {
    pub fn start_bench(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        self.bench = Some(Bench {
            started: now,
            last: now,
            frames: 0,
            slow: 0,
            worst: Duration::ZERO,
            cpu: cpu_time(),
        });
        cx.notify();
    }

    pub(super) fn bench_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(bench) = &mut self.bench else {
            return;
        };
        let now = Instant::now();
        let took = now - bench.last;
        if bench.frames > 0 {
            bench.slow += usize::from(took > SLOW);
            bench.worst = bench.worst.max(took);
        }
        (bench.frames, bench.last) = (bench.frames + 1, now);
        let part = ((now - bench.started).as_secs_f32() / LENGTH.as_secs_f32()).min(1.);
        let most = self.active.list.max_offset_for_scrollbar().y;
        self.active
            .list
            .set_offset_from_scrollbar(point(px(0.), -(most * part)));
        if part < 1. {
            window.request_animation_frame();
            return;
        }
        let cpu = cpu_time() - bench.cpu;
        let built = super::scrollbar::ROWS_BUILT.load(std::sync::atomic::Ordering::Relaxed);
        println!(
            "scroll frames={} slow={} worst_ms={:.1} cpu_ms={} rows_built={built}",
            bench.frames,
            bench.slow,
            bench.worst.as_secs_f64() * 1000.,
            cpu.as_millis()
        );
        self.bench = None;
        cx.quit();
    }
}

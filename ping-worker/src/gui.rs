use once_cell::sync::Lazy;
use raylib::prelude::*;
use std::{
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant},
};

pub trait GetColor {
    fn get_color(&self) -> Color;
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Slash16State {
    Skipped,
    Scheduled,
    Pending,
    Completed,
}

impl Slash16State {
    const SCHEDULED_COLOR: Color = Color::new(0x30, 0x30, 0x30, 0xFF);
    const COMPLETED_COLOR: Color = Color::new(0x50, 0xC0, 0x50, 0xFF);
    const SKIPPED_COLOR: Color = Color::new(0x60, 0x60, 0x60, 0xFF);
    const PENDING_COLOR: Color = Color::new(0xC0, 0xC0, 0x50, 0xFF);
}

impl GetColor for Slash16State {
    fn get_color(&self) -> Color {
        match self {
            Self::Scheduled => Self::SCHEDULED_COLOR,
            Self::Completed => Self::COMPLETED_COLOR,
            Self::Skipped => Self::SKIPPED_COLOR,
            Self::Pending => Self::PENDING_COLOR,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Slash32State {
    Scheduled,
    Pending,
    Success,
    Timeout,
    Error,
}

impl Slash32State {
    const SCHEDULED_COLOR: Color = Color::new(0x30, 0x30, 0x30, 0xFF);
    const PENDING_COLOR: Color = Color::new(0xC0, 0xC0, 0x50, 0xFF);
    const SUCCESS_COLOR: Color = Color::new(0x50, 0xC0, 0x50, 0xFF);
    const TIMEOUT_COLOR: Color = Color::new(0x60, 0x60, 0x60, 0xFF);
    const ERROR_COLOR: Color = Color::new(0xC0, 0x50, 0x50, 0xFF);
}

impl GetColor for Slash32State {
    fn get_color(&self) -> Color {
        match self {
            Self::Scheduled => Self::SCHEDULED_COLOR,
            Self::Pending => Self::PENDING_COLOR,
            Self::Success => Self::SUCCESS_COLOR,
            Self::Timeout => Self::TIMEOUT_COLOR,
            Self::Error => Self::ERROR_COLOR,
        }
    }
}

pub static SLASH_16_STATES: Lazy<Arc<Mutex<[[Slash16State; 256]; 256]>>> =
    Lazy::new(|| Arc::new(Mutex::new([[Slash16State::Scheduled; 256]; 256])));

pub static SLASH_32_STATES: Lazy<Arc<Mutex<[[Slash32State; 256]; 256]>>> =
    Lazy::new(|| Arc::new(Mutex::new([[Slash32State::Scheduled; 256]; 256])));

pub static PENDING_SLASH_16: AtomicU16 = AtomicU16::new(0);

static GLOBAL_START_TIME: Lazy<Instant> = Lazy::new(Instant::now);
pub static CURRENT_START_TIME: Lazy<Arc<RwLock<Instant>>> =
    Lazy::new(|| Arc::new(RwLock::new(Instant::now())));

const SLASH_8_BLOCK_SIZE: f32 = 40.0;
const SLASH_8_BLOCK_SPACING: f32 = 2.0;

const SLASH_16_BLOCK_SIZE: f32 = SLASH_8_BLOCK_SIZE / 16.0;

const TOTAL_SIZE: f32 = 16.0 * SLASH_8_BLOCK_SIZE + 15.0 * SLASH_8_BLOCK_SPACING;
const TEXT_SIZE: i32 = 12;

pub fn gui_main() {
    let (mut rl, thread) = raylib::init()
        .size(1500, 835)
        .title("Ping The Internet")
        .build();

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::new(0x18, 0x18, 0x18, 0xFF));

        d.draw_text(&format!("FPS: {}", d.get_fps()), 5, 5, 12, Color::LIGHTBLUE);

        let slash_16_states = { *SLASH_16_STATES.lock().unwrap() };
        let slash_32_states = { *SLASH_32_STATES.lock().unwrap() };

        let start_location = Vector2::new(75.0, 50.0);
        render_slash_0(&mut d, start_location, &slash_16_states, &slash_32_states);

        let start_location = Vector2::new(800.0, 50.0);
        render_slash_16(&mut d, start_location, &slash_32_states);
    }
}

fn render_slash_0(
    d: &mut RaylibDrawHandle,
    start_location: Vector2,
    slash_16_states: &[[Slash16State; 256]; 256],
    slash_32_states: &[[Slash32State; 256]; 256],
) {
    render_grid(d, start_location, slash_16_states);

    /* Legend */

    let all_states = slash_16_states.iter().flat_map(|s| *s).collect::<Vec<_>>();

    let scheduled = all_states
        .iter()
        .filter(|s| **s == Slash16State::Scheduled)
        .count();
    let pending = all_states
        .iter()
        .filter(|s| **s == Slash16State::Pending)
        .count();
    let completed = all_states
        .iter()
        .filter(|s| **s == Slash16State::Completed)
        .count();
    let skipped = all_states
        .iter()
        .filter(|s| **s == Slash16State::Skipped)
        .count();

    d.set_text_line_spacing(16);
    d.draw_text(
            &format!(
                "Scheduled: {} ({:.1}%)\nPending: {} ({:.1}%)\nCompleted: {} ({:.1}%)\nSkipped: {} ({:.1}%)",
                scheduled, scheduled as f32 / 65536.0 * 100.0,
                pending, pending as f32 / 65536.0 * 100.0,
                completed, completed as f32 / 65536.0 * 100.0,
                skipped, skipped as f32 / 65536.0 * 100.0,
            ),
            start_location.x as i32 + 16,
            (start_location.y + TOTAL_SIZE) as i32 + 20,
            TEXT_SIZE,
            Color::WHITE,
        );

    d.draw_rectangle(
        start_location.x as i32,
        (start_location.y + TOTAL_SIZE) as i32 + 20,
        12,
        12,
        Slash16State::SCHEDULED_COLOR,
    );

    d.draw_rectangle(
        start_location.x as i32,
        (start_location.y + TOTAL_SIZE) as i32 + 20 + 16,
        12,
        12,
        Slash16State::PENDING_COLOR,
    );
    d.draw_rectangle(
        start_location.x as i32,
        (start_location.y + TOTAL_SIZE) as i32 + 20 + 2 * 16,
        12,
        12,
        Slash16State::COMPLETED_COLOR,
    );
    d.draw_rectangle(
        start_location.x as i32,
        (start_location.y + TOTAL_SIZE) as i32 + 20 + 3 * 16,
        12,
        12,
        Slash16State::SKIPPED_COLOR,
    );

    /* Stats */

    let currently_pinging = PENDING_SLASH_16.load(Ordering::Acquire);

    let a = (currently_pinging / 256) as u8;
    let b = (currently_pinging % 256) as u8;

    let completed = slash_32_states
        .iter()
        .flat_map(|s| *s)
        .filter(|s| *s != Slash32State::Scheduled && *s != Slash32State::Pending)
        .count();

    let ratio = completed as f32 / 65536.0;
    let ms_elapsed = CURRENT_START_TIME.read().unwrap().elapsed().as_millis() as u64;
    let total_time_estimated_ms = (ms_elapsed as f32 / ratio) as u64;
    let estimated_time_remaining = Duration::from_millis(total_time_estimated_ms - ms_elapsed);

    d.draw_text(
        &format!(
            "Currently Pinging: {0}.{1}.x.x ({0:0>2X}.{1:0>2X}.xx.xx)",
            a, b,
        ),
        start_location.x as i32 + TOTAL_SIZE as i32 / 2,
        (start_location.y + TOTAL_SIZE) as i32 + 20,
        TEXT_SIZE,
        Color::WHITE,
    );
    d.draw_text(
        &format!(
            "Time Elapsed (Total): {}s\nTime Elapsed (Current /16): {}s\nEstimated Time Remaining (Current /16): {}s",
            GLOBAL_START_TIME.elapsed().as_secs(),
            CURRENT_START_TIME.read().unwrap().elapsed().as_secs(),
            estimated_time_remaining.as_secs(),
        ),
        start_location.x as i32 + TOTAL_SIZE as i32 / 2,
        (start_location.y + TOTAL_SIZE) as i32 + 20 + 16,
        TEXT_SIZE,
        Color::WHITE,
    );
}

fn render_slash_16(
    d: &mut RaylibDrawHandle,
    start_location: Vector2,
    states: &[[Slash32State; 256]; 256],
) {
    render_grid(d, start_location, states);

    let all_states = states.iter().flat_map(|s| *s).collect::<Vec<_>>();

    let scheduled = all_states
        .iter()
        .filter(|s| **s == Slash32State::Scheduled)
        .count();
    let pending = all_states
        .iter()
        .filter(|s| **s == Slash32State::Pending)
        .count();
    let success = all_states
        .iter()
        .filter(|s| **s == Slash32State::Success)
        .count();
    let timeout = all_states
        .iter()
        .filter(|s| **s == Slash32State::Timeout)
        .count();
    let error = all_states
        .iter()
        .filter(|s| **s == Slash32State::Error)
        .count();

    d.set_text_line_spacing(16);
    d.draw_text(
        &format!(
            "Scheduled: {} ({:.1}%)\nPending: {} ({:.1}%)\nSuccess: {} ({:.1}%)\nTimeout: {} ({:.1}%)\nError: {} ({:.1}%)",
            scheduled, scheduled as f32 / 65536.0 * 100.0,
            pending, pending as f32 / 65536.0 * 100.0,
            success, success as f32 / 65536.0 * 100.0,
            timeout, timeout as f32 / 65536.0 * 100.0,
            error, error as f32 / 65536.0 * 100.0,
        ),
        start_location.x as i32 + 16,
        (start_location.y + TOTAL_SIZE) as i32 + 20,
        TEXT_SIZE,
        Color::WHITE,
    );

    d.draw_rectangle(
        start_location.x as i32,
        (start_location.y + TOTAL_SIZE) as i32 + 20,
        12,
        12,
        Slash32State::SCHEDULED_COLOR,
    );
    d.draw_rectangle(
        start_location.x as i32,
        (start_location.y + TOTAL_SIZE) as i32 + 20 + 16,
        12,
        12,
        Slash32State::PENDING_COLOR,
    );
    d.draw_rectangle(
        start_location.x as i32,
        (start_location.y + TOTAL_SIZE) as i32 + 20 + 2 * 16,
        12,
        12,
        Slash32State::SUCCESS_COLOR,
    );
    d.draw_rectangle(
        start_location.x as i32,
        (start_location.y + TOTAL_SIZE) as i32 + 20 + 3 * 16,
        12,
        12,
        Slash32State::TIMEOUT_COLOR,
    );
    d.draw_rectangle(
        start_location.x as i32,
        (start_location.y + TOTAL_SIZE) as i32 + 20 + 4 * 16,
        12,
        12,
        Slash32State::ERROR_COLOR,
    );
}

fn render_grid(
    d: &mut RaylibDrawHandle,
    start_location: Vector2,
    states: &[[impl GetColor; 256]; 256],
) {
    for x in 0..16 {
        for y in 0..16 {
            render_block(
                d,
                Vector2::new(
                    start_location.x + x as f32 * (SLASH_8_BLOCK_SIZE + SLASH_8_BLOCK_SPACING),
                    start_location.y + y as f32 * (SLASH_8_BLOCK_SIZE + SLASH_8_BLOCK_SPACING),
                ),
                &states[y * 16 + x],
            )
        }
    }

    for x in 0..16 {
        let label = format!("{:0>2X}", x);

        let width = d.measure_text(&label, 12);

        d.draw_text(
            &label,
            (start_location.x
                + x as f32 * (SLASH_8_BLOCK_SIZE + SLASH_8_BLOCK_SPACING)
                + SLASH_8_BLOCK_SIZE / 2.0
                - width as f32 / 2.0) as i32,
            start_location.y as i32 - TEXT_SIZE - 20,
            TEXT_SIZE,
            Color::LIGHTGRAY,
        );
    }

    for y in 0..16 {
        let label = format!("{:0>2X}", y * 16);

        let width = d.measure_text(&label, 12);

        d.draw_text(
            &label,
            start_location.x as i32 - width - 20,
            (start_location.y
                + y as f32 * (SLASH_8_BLOCK_SIZE + SLASH_8_BLOCK_SPACING)
                + SLASH_8_BLOCK_SIZE / 2.0
                - 6.0) as i32,
            TEXT_SIZE,
            Color::LIGHTGRAY,
        );
    }
}

fn render_block(d: &mut RaylibDrawHandle, start_location: Vector2, states: &[impl GetColor; 256]) {
    for x in 0..16 {
        for y in 0..16 {
            let color = states[y * 16 + x].get_color();

            d.draw_rectangle_v(
                Vector2::new(
                    start_location.x + x as f32 * SLASH_16_BLOCK_SIZE,
                    start_location.y + y as f32 * SLASH_16_BLOCK_SIZE,
                ),
                Vector2::new(SLASH_16_BLOCK_SIZE - 0.5, SLASH_16_BLOCK_SIZE - 0.5),
                color,
            )
        }
    }
}

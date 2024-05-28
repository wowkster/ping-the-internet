use std::{
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use once_cell::sync::Lazy;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use raylib::prelude::*;

#[derive(Debug, Clone, Copy)]
enum BlockState {
    NotPinged,
    Success,
    Timeout,
    Error,
}

impl BlockState {
    const NOT_PINGED_COLOR: Color = Color::new(0x30, 0x30, 0x30, 0xFF);
    const SUCCESS_COLOR: Color = Color::new(0x50, 0xC0, 0x50, 0xFF);
    const TIMEOUT_COLOR: Color = Color::new(0x60, 0x60, 0x60, 0xFF);
    const ERROR_COLOR: Color = Color::new(0xC0, 0x50, 0x50, 0xFF);

    pub fn get_color(&self) -> Color {
        match self {
            BlockState::NotPinged => Self::NOT_PINGED_COLOR,
            BlockState::Success => Self::SUCCESS_COLOR,
            BlockState::Timeout => Self::TIMEOUT_COLOR,
            BlockState::Error => Self::ERROR_COLOR,
        }
    }
}

impl Distribution<BlockState> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BlockState {
        match rng.gen_range(0..100) {
            0..=9 => BlockState::Success,
            10..=98 => BlockState::Timeout,
            _ => BlockState::Error,
        }
    }
}

const SLASH_8_BLOCK_SIZE: f32 = 36.0;
const SLASH_8_BLOCK_SPACING: f32 = 2.0;

const SLASH_16_BLOCK_SIZE: f32 = SLASH_8_BLOCK_SIZE / 16.0;

const TOTAL_SIZE: f32 = 16.0 * SLASH_8_BLOCK_SIZE + 15.0 * SLASH_8_BLOCK_SPACING;
const TEXT_SIZE: i32 = 12;

static STATES: Lazy<Arc<Mutex<[[BlockState; 256]; 256]>>> =
    Lazy::new(|| Arc::new(Mutex::new([[BlockState::NotPinged; 256]; 256])));

static SLASH_16: AtomicU16 = AtomicU16::new(0);
static SLASH_32: AtomicU16 = AtomicU16::new(0);

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1400, 750)
        .title("Ping The Internet")
        .build();
    // rl.set_target_fps(80);

    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_micros(200000));

        let mut states = STATES.lock().unwrap();

        let index = SLASH_16.fetch_add(1, Ordering::Acquire);

        if index < u16::MAX {
            let x = (index / 256) as usize;
            let y = (index % 256) as usize;

            states[x][y] = rand::random();
        } else {
            *states = [[BlockState::NotPinged; 256]; 256]
        }
    });

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::new(0x20, 0x20, 0x20, 0xFF));

        d.draw_text(&format!("FPS: {}", d.get_fps()), 5, 5, 12, Color::LIGHTBLUE);

        let states = STATES.lock().unwrap();

        let start_location = Vector2::new(75.0, 90.0);

        render_slash_0(
            &mut d,
            start_location,
            &states,
            SLASH_16.load(Ordering::Acquire),
        );

        let start_location = Vector2::new(750.0, 90.0);

        render_slash_0(
            &mut d,
            start_location,
            &states,
            SLASH_32.load(Ordering::Acquire),
        );
    }
}

fn render_slash_0(
    d: &mut RaylibDrawHandle,
    start_location: Vector2,
    states: &[[BlockState; 256]; 256],
    currently_pinging: u16,
) {
    for x in 0..16 {
        for y in 0..16 {
            render_slash_8(
                d,
                Vector2::new(
                    start_location.x + x as f32 * (SLASH_8_BLOCK_SIZE + SLASH_8_BLOCK_SPACING),
                    start_location.y + y as f32 * (SLASH_8_BLOCK_SIZE + SLASH_8_BLOCK_SPACING),
                ),
                states[y * 16 + x],
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

    let a = (currently_pinging / 256) as u8;
    let b = (currently_pinging % 256) as u8;

    d.draw_text(
        &format!(
            "Currently Pinging: {0}.{1}.x.x ({0:0>2X}.{1:0>2X}.xx.xx)",
            a, b
        ),
        start_location.x as i32,
        (start_location.y + TOTAL_SIZE) as i32 + 20,
        TEXT_SIZE,
        Color::WHITE,
    );
}

fn render_slash_8(d: &mut RaylibDrawHandle, start_location: Vector2, states: [BlockState; 256]) {
    for x in 0..16 {
        for y in 0..16 {
            let state = states[y * 16 + x];

            d.draw_rectangle_v(
                Vector2::new(
                    start_location.x + x as f32 * SLASH_16_BLOCK_SIZE,
                    start_location.y + y as f32 * SLASH_16_BLOCK_SIZE,
                ),
                Vector2::new(SLASH_16_BLOCK_SIZE - 0.5, SLASH_16_BLOCK_SIZE - 0.5),
                state.get_color(),
            )
        }
    }
}

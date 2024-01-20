// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use image::{imageops, RgbaImage};
use rand::{rngs::SmallRng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use wassily::prelude::*;

const W: f32 = 1024.0;

// Shared state for the tauri app.
struct State {
    base_image: Mutex<RgbaImage>,
}

// Data to send to the js side for rendering the image.
#[derive(Serialize)]
struct Picture {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

#[derive(Deserialize)]
enum Style {
    Dots,
    VLines,
    HLines,
    Cross,
    Stipple,
    Grid,
    Multi,
}

fn main() {
    tauri::Builder::default()
        .manage(State {
            base_image: Mutex::new(RgbaImage::new(0, 0)),
        })
        .invoke_handler(tauri::generate_handler![get_image, gen_image, save_image])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Open the image and store it in the global state.
// Scale it to the canvas size before sending it to the js side.
#[tauri::command]
fn get_image(path: &str, state: tauri::State<State>) -> Result<Picture, String> {
    let img = image::open(path)
        .map_err(|err| format!("The file at {} could not be opened: {}", path, err))?;
    let mut state_base_image = state.base_image.lock().expect("Could not lock state mutex");
    *state_base_image = img.to_rgba8();
    let scale = W / img.width() as f32;
    let nwidth = (img.width() as f32 * scale) as u32;
    let nhight = (img.height() as f32 * scale) as u32;
    let new_img = imageops::resize(&img, nwidth, nhight, imageops::FilterType::Lanczos3);
    Ok(Picture {
        width: nwidth,
        height: nhight,
        data: new_img.into_vec(),
    })
}

pub fn halton_seq(width: f32, height: f32, n: u32, seed: u64) -> Vec<Point> {
    let mut rng = SmallRng::seed_from_u64(seed);
    let k: u32 = rng.gen();
    let xs = (k..n + k).map(|i| halton(i, 2));
    let ys = (k..n + k).map(|i| halton(i, 3));
    xs.zip(ys)
        .map(|p| {
            Point::from_xy(
                (p.0 * (width as f32)).clamp(0.0, width as f32 - 1.0),
                (p.1 * (height as f32)).clamp(0.0, width as f32 - 1.0),
            )
        })
        .collect()
}

fn dots(cell: u32, x: u32, y: u32, t: f32, canvas: &mut Canvas) {
    Shape::new()
        .circle(
            pt(x * cell + cell / 2, y * cell + cell / 2),
            t * cell as f32 * 0.6036, // mid way between sqrt(2)/2 and 1/2.
        )
        .fill_color(*BLACK)
        .no_stroke()
        .draw(canvas);
}

fn vline(cell: u32, x: u32, y: u32, t: f32, canvas: &mut Canvas) {
    let g = (t * cell as f32).round() as u32;
    let gs = bool_vec(cell as usize, g as usize);
    for l in 0..cell {
        if gs[l as usize] {
            Shape::new()
                .line(
                    pt(x * cell + l, y * cell),
                    pt(x * cell + l, y * cell + cell),
                )
                .no_fill()
                .stroke_color(*BLACK)
                .stroke_weight(1.0)
                .draw(canvas);
        }
    }
}

fn hline(cell: u32, x: u32, y: u32, t: f32, canvas: &mut Canvas) {
    let g = (t * cell as f32).round() as u32;
    let gs = bool_vec(cell as usize, g as usize);
    for l in 0..cell {
        if gs[l as usize] {
            Shape::new()
                .line(
                    pt(x * cell, y * cell + l),
                    pt(x * cell + cell, y * cell + l),
                )
                .no_fill()
                .stroke_color(*BLACK)
                .stroke_weight(1.0)
                .draw(canvas);
        }
    }
}

fn cross(cell: u32, x: u32, y: u32, t: f32, canvas: &mut Canvas) {
    let c = Color::from_rgba8(0, 0, 0, 127);
    let g = (t * cell as f32).round() as u32;
    let gs = bool_vec(cell as usize, g as usize);
    for l in 0..cell {
        if gs[l as usize] {
            Shape::new()
                .line(
                    pt(x * cell + l, y * cell),
                    pt(x * cell + l, y * cell + cell),
                )
                .no_fill()
                .stroke_color(c)
                .stroke_weight(1.0)
                .draw(canvas);
        }
    }
    let gs = bool_vec(cell as usize, g as usize);
    for l in 0..cell {
        if gs[l as usize] {
            Shape::new()
                .line(
                    pt(x * cell, y * cell + l),
                    pt(x * cell + cell, y * cell + l),
                )
                .no_fill()
                .stroke_color(c)
                .stroke_weight(1.0)
                .draw(canvas);
        }
    }
}

fn stipple(cell: u32, x: u32, y: u32, t: f32, rng: &mut SmallRng, canvas: &mut Canvas) {
    let n = t * (cell * cell) as f32;
    let ps = halton_seq(cell as f32, cell as f32, n as u32, rng.gen());
    let qs = ps
        .into_iter()
        .map(|p| pt((x * cell) as f32 + p.x, (y * cell) as f32 + p.y));
    for p in qs {
        canvas.dot(p.x, p.y, *BLACK)
    }
}

fn grid(cell: u32, x: u32, y: u32, t: f32, canvas: &mut Canvas) {
    let s = (1.0 / t).clamp(1.0, cell as f32);
    let x0 = (cell * x) as f32;
    let y0 = (cell * y) as f32;
    let mut i = x0;
    while i < x0 + cell as f32 {
        let mut j = y0;
        while j < y0 + cell as f32 {
            canvas.dot(i, j, *BLACK);
            j += s;
        }
        i += s;
    }
}

#[tauri::command]
fn gen_image(cell: u32, style: Style, state: tauri::State<State>) -> Picture {
    let img = generate(cell, style, state);
    let scale = W / img.width() as f32;
    let nwidth = (img.width() as f32 * scale) as u32;
    let nhight = (img.height() as f32 * scale) as u32;
    let new_img = imageops::resize(&img, nwidth, nhight, imageops::FilterType::Lanczos3);
    Picture {
        width: nwidth,
        height: nhight,
        data: new_img.into_vec(),
    }
}

fn generate(cell: u32, style: Style, state: tauri::State<State>) -> RgbaImage {
    let mut rng = SmallRng::from_entropy();
    let in_img = state
        .base_image
        .lock()
        .expect("Could not lock state mutex")
        .clone();
    let width = cell * in_img.width();
    let height = cell * in_img.height();
    let mut canvas = Canvas::new(width, height);
    canvas.fill(*WHITE);
    for x in 0..in_img.width() {
        for y in 0..in_img.height() {
            let pixel = in_img.get_pixel(x, y);
            let color =
                (0.2989 * pixel[0] as f32 + 0.5870 * pixel[1] as f32 + 0.1140 * pixel[2] as f32)
                    / 255.0;
            let t = 1.0 - color;
            match style {
                Style::Dots => dots(cell, x, y, t, &mut canvas),
                Style::VLines => vline(cell, x, y, t, &mut canvas),
                Style::HLines => hline(cell, x, y, t, &mut canvas),
                Style::Cross => cross(cell, x, y, t, &mut canvas),
                Style::Stipple => stipple(cell, x, y, t, &mut rng, &mut canvas),
                Style::Grid => grid(cell, x, y, t, &mut canvas),
                Style::Multi => {
                    let hue = pixel_to_hue(pixel);
                    match hue {
                        15..=45 => cross(cell, x, y, t, &mut canvas), // orange
                        46..=75 => stipple(cell, x, y, t, &mut rng, &mut canvas), // yellow
                        76..=165 => vline(cell, x, y, t, &mut canvas), // green
                        166..=255 => dots(cell, x, y, t, &mut canvas), // blue
                        256..=345 => grid(cell, x, y, t, &mut canvas), // purple
                        _ => hline(cell, x, y, t, &mut canvas),       // red
                    }
                }
            }
        }
    }
    let out_img = canvas.into();
    out_img
}

#[tauri::command]
fn save_image(path: &str, cell: u32, style: Style, state: tauri::State<State>) {
    let gen = generate(cell, style, state);
    let _ = gen.save(path);
}

fn bool_vec(n: usize, k: usize) -> Vec<bool> {
    let mut rng = SmallRng::from_entropy();
    let mut vec = vec![true; k];
    vec.extend(vec![false; n - k]);
    vec.shuffle(&mut rng);
    vec
}

fn pixel_to_hue(pixel: &Rgba<u8>) -> i32 {
    let r = pixel[0] as f32 / 255.0;
    let g = pixel[1] as f32 / 255.0;
    let b = pixel[2] as f32 / 255.0;

    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;

    if delta == 0.0 {
        // Achromatic case (grey scale), hue is undefined
        0
    } else {
        let hue = if max == r {
            // Red is max
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            // Green is max
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            // Blue is max
            60.0 * (((r - g) / delta) + 4.0)
        };

        let hue = hue.round() as i32;
        if hue < 0 {
            hue + 360
        } else {
            hue
        }
    }
}

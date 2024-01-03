// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use image::{imageops, RgbaImage};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use wassily::prelude::*;

const W: f32 = 1024.0;
const CELL_SIZE: u32 = 20;

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

#[tauri::command]
fn gen_image(state: tauri::State<State>) -> Picture {
    let img = generate(state);
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

fn generate(state: tauri::State<State>) -> RgbaImage {
    let in_img = state
        .base_image
        .lock()
        .expect("Could not lock state mutex")
        .clone();
    let width = CELL_SIZE * in_img.width();
    let height = CELL_SIZE * in_img.height();
    let mut canvas = Canvas::new(width, height);
    canvas.fill(*WHITE);
    for x in 0..in_img.width() {
        for y in 0..in_img.height() {
            let pixel = in_img.get_pixel(x, y);
            let color = (Color::from_rgba8(pixel[0], pixel[1], pixel[2], 255)).grayscale();
            let radius = 1.0 - color.red();
            Shape::new()
                .circle(
                    pt(x * CELL_SIZE + CELL_SIZE / 2, y * CELL_SIZE + CELL_SIZE / 2),
                    radius * CELL_SIZE as f32 * 0.5,
                )
                .fill_color(*SADDLEBROWN)
                .no_stroke()
                .draw(&mut canvas);
        }
    }
    let out_img = canvas.into();
    out_img
}

#[tauri::command]
fn save_image(path: &str, state: tauri::State<State>) {
    let gen = generate(state);
    let _ = gen.save(path);
}

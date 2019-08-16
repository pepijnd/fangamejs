#![feature(test)]

extern crate test;

extern crate bigdecimal;
extern crate num;
extern crate packed_simd;
extern crate palette;
extern crate rug;
extern crate sdl2;
extern crate threadpool;
extern crate time;

mod complex;
mod render;

use std::time::Duration;
use threadpool::ThreadPool;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::pixels;
use sdl2::rect::Rect;

const WINDOW_WIDTH: u32 = 1600;
const WINDOW_HEIGHT: u32 = 900;

use rug::Float;

use complex::{Bound, BoundsChecker, BoundsSettings};
use render::{render, RenderSettings, RenderEngine};

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("mandelbrot-rust", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window
        .into_canvas()
        .target_texture()
        .accelerated()
        .present_vsync()
        .build()
        .unwrap();
    let texture_creator = canvas.texture_creator();

    let mut update = true;

    let precision: u32 = 53;
    macro_rules! float_new {
        ($e:expr) => {
            Float::with_val(precision, $e)
        };
    }

    let mut x = float_new!(-0.5);
    let mut y = float_new!(0.0);
    let mut scale = float_new!(1.75);

    let mut mstart_x: i32 = 0;
    let mut mstart_y: i32 = 0;

    let mut mcurr_x: i32 = 0;
    let mut mcurr_y: i32 = 0;

    let mut drawrect = false;

    let mut pool = ThreadPool::new(8);

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut man_texture = texture_creator.create_texture_target(None, 1, 1).unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,

                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => {
                    x -= float_new!(&scale / 3.0);
                    update = true;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    x += float_new!(&scale / 3.0);
                    update = true;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    y -= float_new!(&scale / 3.0);
                    update = true;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    y += float_new!(&scale / 3.0);
                    update = true;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::KpPlus),
                    ..
                } => {
                    scale *= 0.97;
                    update = true;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::KpMinus),
                    ..
                } => {
                    scale /= 0.97;
                    update = true;
                }

                Event::KeyDown {
                    keycode: Some(Keycode::Home),
                    ..
                } => {
                    x = float_new!(-0.5);
                    y = float_new!(0.0);
                    scale = float_new!(1.75);
                    update = true;
                }

                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    x,
                    y,
                    ..
                } => {
                    mstart_x = x;
                    mstart_y = y;
                    drawrect = true;
                }

                Event::MouseButtonUp {
                    mouse_btn: MouseButton::Left,
                    ..
                } => {
                    drawrect = false;

                    let xx = float_new!((mstart_x + mcurr_x) / 2 - (WINDOW_WIDTH as i32) / 2);
                    let yy = float_new!((mstart_y + mcurr_y) / 2 - (WINDOW_HEIGHT as i32) / 2);

                    let ratio = float_new!(WINDOW_WIDTH as f64 / WINDOW_HEIGHT as f64);
                    x += float_new!(&scale * &ratio) * (xx / WINDOW_WIDTH);
                    y += float_new!(&scale) * (yy / WINDOW_HEIGHT);

                    let w = float_new!(mcurr_x - mstart_x) / WINDOW_WIDTH;
                    let h = float_new!(mcurr_y - mstart_y) / WINDOW_HEIGHT;

                    scale *= (w + h) / 2.0;

                    update = true;
                }

                Event::MouseMotion { x, y, .. } => {
                    mcurr_x = x;
                    mcurr_y = y;
                }

                _ => {}
            }
        }

        if update {
            let settings = RenderSettings::new(
                float_new!(&x),
                float_new!(&y),
                float_new!(&scale),
                1600,
                900,
                RenderEngine::MPC,
                BoundsSettings::new(500, precision),
            );
            man_texture = render(&mut canvas, &texture_creator, Some(&mut pool), settings);
            println!("rendered");
            update = false;
        }
        canvas.copy(&man_texture, None, None).unwrap();
        if drawrect {
            canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
            canvas
                .draw_rect(Rect::new(
                    mstart_x,
                    mstart_y,
                    (mcurr_x - mstart_x) as u32,
                    (mcurr_y - mstart_y) as u32,
                ))
                .unwrap();
        }
        canvas.present();

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

// #[cfg(test)]
// mod tests {

//     use super::*;
//     use test::Bencher;

//     #[bench]
//     fn mandelbrot_render(b: &mut Bencher) {
//         let width = WINDOW_WIDTH / 4;
//         let height = WINDOW_HEIGHT / 4;

//         let surface =
//             sdl2::surface::Surface::new(width, height, sdl2::pixels::PixelFormatEnum::RGB24)
//                 .unwrap();
//         let mut canvas = sdl2::render::Canvas::from_surface(surface).unwrap();
//         let text_crt = canvas.texture_creator();

//         let x = 0.428860;
//         let y = -0.231332;
//         let scale = 0.000005;
//         b.iter(|| {
//             let mandelbrot: Mandelbrot<Simd<[f64; 4]>, f64, i64> =
//                 Mandelbrot::new(x, y, scale, 500, 5, 0);
//             let man_texture = mandelbrot.render(&mut canvas, &text_crt, None, 1600, 900);
//             test::black_box(man_texture);
//         });
//     }
// }

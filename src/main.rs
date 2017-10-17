#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_mut)]

#![feature(iterator_step_by)]
#![feature(test)]
extern crate test;

extern crate palette;
extern crate sdl2;
extern crate time;
extern crate threadpool;
extern crate simd;

use threadpool::ThreadPool;
use std::sync::mpsc::channel;
use std::env;
use std::time::Duration;
use std::collections::HashSet;

use simd::x86::avx::f64x4;
use simd::x86::avx::bool64ix4;
use simd::x86::avx::i64x4;

use palette::Hue;

use time::PreciseTime;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels;
use sdl2::rect::{Point, Rect};
use sdl2::video::{Window, WindowContext};
use sdl2::render::{Canvas, Texture, TextureCreator};

use sdl2::gfx::primitives::DrawRenderer;

const WINDOW_WIDTH: u32 = 1600;
const WINDOW_HEIGHT: u32 = 900;

struct Complex {
    real: f64,
    imag: f64,
}

impl Complex {
    fn new(real: f64, imag: f64) -> Complex {
        Complex {
            real,
            imag,
        }
    }

    fn square(&mut self) {
        let old_real = self.real;
        let old_imag = self.imag;
        self.real = old_real.powi(2) - old_imag.powi(2);
        self.imag = 2.0 * old_real * old_imag;
    }

    fn add<'a>(&mut self, other: &'a Complex) {
        self.real = self.real + other.real;
        self.imag = self.imag + other.imag;
    }

    fn distance_squared(&self) -> f64 {
        (self.real * self.real) + (self.imag * self.imag)
    }

    fn iterate<'a>(&mut self, other: &'a Complex) {
        self.square();
        self.add(other);
    }

    fn print(&self) {
        if self.imag >= 0.0 {
            println!("{}+{}i", self.real, self.imag);
        } else {
            println!("{}-{}i", self.real, -self.imag);
        }
    }
}


struct Mandelbrot {
    xpos: f64,
    ypos: f64,
    scale: f64,
    iterations: u32,
    color_step: u32,
    hue_shift: i32,
}

impl Mandelbrot {
    fn squared_distance(input: (f64x4, f64x4)) -> f64x4 {
        input.0 * input.0 + input.1 * input.1
    }

    fn iterate(input: (f64x4, f64x4), other: (f64x4, f64x4)) -> (f64x4, f64x4) {
        (input.0 * input.0 - input.1 * input.1 + other.0,
         (input.0 * input.1 + other.1) + (input.0 * input.1 + other.1))
    }

    fn check_unbounded(input: (f64x4, f64x4), max_iteration: u32) -> i64x4 {
        let mut count = i64x4::splat(0);

        let mut z = (f64x4::splat(0_f64), f64x4::splat(0_f64));

        for _ in 0..max_iteration {
            let distance = Mandelbrot::squared_distance(z);
            let mask = distance.lt(f64x4::splat(4.0));
            if !mask.any() {
                break
            }
            count = count + mask.to_i().to_repr();

            z = Mandelbrot::iterate(z, input);
        }
        count
    }

    fn render<'a, T, B>(&self, canvas: &mut Canvas<T>, texture_creator: &'a TextureCreator<B>) -> Texture<'a> where T: sdl2::render::RenderTarget {

        let time_start = PreciseTime::now();

        let width = WINDOW_WIDTH;
        let height = WINDOW_HEIGHT;

        let ratio = width as f64 / height as f64;

        let x_start: f64 = self.xpos - ((self.scale * ratio) / 2.0);
        let x_start = f64x4::splat(x_start);
        let x_step: f64 = (self.scale * ratio) / width as f64;
        let x_step = f64x4::splat(x_step);

        let y_start: f64 = self.ypos - ((self.scale / ratio) / 2.0);
        let y_start = f64x4::splat(y_start);
        let y_step: f64 = (self.scale / ratio) / height as f64;
        let y_step = f64x4::splat(y_step);

        let mask = f64x4::new(0_f64, 1_f64, 2_f64, 3_f64);

        let mut target_texture: Texture = texture_creator.create_texture_target(None, width, height).unwrap();

        let (tx, rx) = channel();
        let pool = ThreadPool::new(8);

        let max_iterations = self.iterations;
        let color_step = self.color_step;
        let hue_shift = self.hue_shift;

        for y in 0..height {
            let tx = tx.clone();
            pool.execute(move || {
                let mut row = Vec::with_capacity(width as usize);
                for x in (0..width).step_by(4) {
                    let xx = x_start + ((f64x4::splat(x as f64) + mask) * x_step);
                    let yy = y_start +  (f64x4::splat(y as f64) * y_step);


                    let iterations = Mandelbrot::check_unbounded((xx, yy), max_iterations);
                    for i in 0..4 {
                        let iterations = iterations.extract(i) as u32;

                        if iterations != max_iterations {
                            let step = (iterations as f64 % (360.0 / color_step as f64)) * color_step as f64;
                            let color = palette::Hsv::new(palette::RgbHue::from(step), 0.7, 0.7);
                            let color = color.shift_hue(palette::RgbHue::from(hue_shift as f64));
                            let rgb = palette::Rgb::from(color);

                            row.push((y as i16, pixels::Color::RGB((rgb.red * 255.0) as u8, (rgb.green * 255.0) as u8, (rgb.blue * 255.0) as u8)));
                        } else {
                            row.push((y as i16, pixels::Color::RGB(0_u8, 0_u8, 0_u8)));
                        }
                    }
                }
                tx.send(row).unwrap();
            });
        }

        canvas.with_texture_canvas(&mut target_texture, |texture_canvas| {
            for _ in 0..height {
                let k = rx.recv().unwrap();
                for (i, j) in k.iter().enumerate() {
                    texture_canvas.pixel(i as i16, j.0, j.1).unwrap();
                }
            }
        }).unwrap();

        println!("{}", time_start.to(PreciseTime::now()));

        target_texture
    }

    fn send_pixel() {

    }

    fn new(xpos: f64, ypos: f64, scale: f64, iterations: u32, color_step: u32, hue_shift: i32) -> Mandelbrot {
        Mandelbrot {
            xpos,
            ypos,
            scale,
            iterations,
            color_step,
            hue_shift,
        }
    }
}


fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("mandelbrot-rust", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas()
        .target_texture()
        .accelerated()
        .present_vsync()
        .build().unwrap();
    let texture_creator = canvas.texture_creator();

    let mut x = -0.7442;
    let mut y = -0.1042;
    let mut scale = 0.0005;

    let mandelbrot = Mandelbrot::new(x, y, scale, 500, 5, 0);
    let man_texture = mandelbrot.render(&mut canvas, &texture_creator);

    canvas.copy(&man_texture, None, None).unwrap();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                }

                Event::KeyDown { keycode: Some(Keycode::Left), ..} => {
                    x -= scale / 3.0;
                    let mandelbrot = Mandelbrot::new(x, y, scale, 500, 5, 0);
                    let man_texture = mandelbrot.render(&mut canvas, &texture_creator);
                    canvas.copy(&man_texture, None, None).unwrap();
                    canvas.present();
                }

                Event::KeyDown { keycode: Some(Keycode::Right), ..} => {
                    x += scale / 3.0;
                    let mandelbrot = Mandelbrot::new(x, y, scale, 500, 5, 0);
                    let man_texture = mandelbrot.render(&mut canvas, &texture_creator);
                    canvas.copy(&man_texture, None, None).unwrap();
                    canvas.present();
                }

                Event::KeyDown { keycode: Some(Keycode::Up), ..} => {
                    y -= scale / 3.0;
                    let mandelbrot = Mandelbrot::new(x, y, scale, 500, 5, 0);
                    let man_texture = mandelbrot.render(&mut canvas, &texture_creator);
                    canvas.copy(&man_texture, None, None).unwrap();
                    canvas.present();
                }

                Event::KeyDown { keycode: Some(Keycode::Down), ..} => {
                    y += scale / 3.0;
                    let mandelbrot = Mandelbrot::new(x, y, scale, 500, 5, 0);
                    let man_texture = mandelbrot.render(&mut canvas, &texture_creator);
                    canvas.copy(&man_texture, None, None).unwrap();
                    canvas.present();
                }

                Event::KeyDown { keycode: Some(Keycode::KpPlus), ..} => {
                    scale *= 0.97;
                    let mandelbrot = Mandelbrot::new(x, y, scale, 500, 5, 0);
                    let man_texture = mandelbrot.render(&mut canvas, &texture_creator);
                    canvas.copy(&man_texture, None, None).unwrap();
                    canvas.present();
                }

                Event::KeyDown { keycode: Some(Keycode::KpMinus), ..} => {
                    scale /= 0.97;
                    let mandelbrot = Mandelbrot::new(x, y, scale, 500, 5, 0);
                    let man_texture = mandelbrot.render(&mut canvas, &texture_creator);
                    canvas.copy(&man_texture, None, None).unwrap();
                    canvas.present();
                }
                _ => {}
            }
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use test::{Bencher, BenchMode};

    #[test]
    fn mandelbrot_render() {
        for _ in 0..200 {

            let surface = sdl2::surface::Surface::new(WINDOW_WIDTH, WINDOW_HEIGHT, sdl2::pixels::PixelFormatEnum::RGB24).unwrap();
            let mut canvas = sdl2::render::Canvas::from_surface(surface).unwrap();
            let text_crt = canvas.texture_creator();

            let mut x = 0.428860;
            let mut y = -0.231332;
            let scale = 0.000005;

            let mandelbrot = Mandelbrot::new(x, y, scale, 5000, 5, 0);
            let man_texture = mandelbrot.render(&mut canvas, &text_crt);
            canvas.copy(&man_texture, Some(Rect::new(WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32, WINDOW_WIDTH, WINDOW_HEIGHT)), None).unwrap();

            test::black_box(canvas);
        }
    }
}

use sdl2::gfx::primitives::DrawRenderer;
use sdl2::pixels;
use sdl2::render::{Canvas, Texture, TextureCreator};
use std::sync::mpsc::channel;
use threadpool::ThreadPool;

use palette::Hue;
use rug::{Float, Complex};

use packed_simd::{
    f64x4
};

use complex::{Bound, BoundsChecker, BoundsSettings};

#[derive(Clone, Copy)]
pub enum RenderEngine {
    Single,
    Double,
    MPC,
    SimdF64x4
}

pub struct RenderSettings {
    x: Float,
    y: Float,
    scale: Float,
    width: u32,
    height: u32,
    engine: RenderEngine,
    bounds: BoundsSettings,
}

impl Clone for RenderSettings {
    fn clone(&self) -> Self {
        RenderSettings::new(
            self.x.clone(),
            self.y.clone(),
            self.scale.clone(),
            self.width,
            self.height,
            self.engine,
            self.bounds,
        )
    }
}

impl RenderSettings {
    pub fn new(
        x: Float,
        y: Float,
        scale: Float,
        width: u32,
        height: u32,
        engine: RenderEngine,
        bounds: BoundsSettings,
    ) -> RenderSettings {
        RenderSettings {
            x,
            y,
            scale,
            width,
            height,
            engine,
            bounds,
        }
    }
}

pub fn render<'a, R, B>(
    canvas: &mut Canvas<R>,
    texture_creator: &'a TextureCreator<B>,
    global_pool: Option<&mut ThreadPool>,
    settings: RenderSettings) -> Texture<'a> where
    R: sdl2::render::RenderTarget, {
        match settings.engine {
            RenderEngine::Single =>      render_with_engine::<_, _, f32>(canvas, texture_creator, global_pool, settings),
            RenderEngine::Double =>      render_with_engine::<_, _, f64>(canvas, texture_creator, global_pool, settings),
            RenderEngine::MPC =>     render_with_engine::<_, _, Complex>(canvas, texture_creator, global_pool, settings),
            RenderEngine::SimdF64x4 => render_with_engine::<_, _, f64x4>(canvas, texture_creator, global_pool, settings),

        }
}

fn render_with_engine<'a, R, B, T: BoundsChecker + 'static>(
    canvas: &mut Canvas<R>,
    texture_creator: &'a TextureCreator<B>,
    global_pool: Option<&mut ThreadPool>,
    settings: RenderSettings,
) -> Texture<'a>
where
    R: sdl2::render::RenderTarget,
{
    let precision = settings.bounds.precision;
    macro_rules! float_new {
        ($e:expr) => {
            Float::with_val(precision, $e)
        };
    }

    let w = float_new!(settings.width);
    let h = float_new!(settings.height);
    let ratio = float_new!(&w / &h);

    let x_start = float_new!(&settings.x - (float_new!(&settings.scale * &ratio) / 2));
    let x_step = float_new!(&settings.scale * &ratio) / &w;
    let y_start = float_new!(&settings.y - (float_new!(&settings.scale / 2)));
    let y_step = float_new!(&settings.scale / &h);

    let mut target_texture: Texture = texture_creator
        .create_texture_target(None, settings.width, settings.height)
        .unwrap();

    let (tx, rx) = channel();

    let mut pool;
    let thread_pool = {
        if let Some(pool) = global_pool {
            pool
        } else {
            pool = ThreadPool::new(8);
            &mut pool
        }
    };

    let _max_iterations = settings.bounds.limit;
    let color_step = 2;
    let hue_shift = 0;

    //let mask = T::mask();
    let step = T::mask().len();

    for y in 0..settings.height {
        let tx = tx.clone();
        let settings = settings.clone();
        let x_start = float_new!(&x_start);
        let x_step = float_new!(&x_step);
        let y_start = float_new!(&y_start);
        let y_step = float_new!(&y_step);
        thread_pool.execute(move || {
            let mut output: Vec<Bound> = Vec::with_capacity(settings.width as usize);
            let yy = float_new!(&y_start + float_new!(&y_step * y));
            for x in (0..settings.width).step_by(step) {
                let mut xx: Vec<Float> = Vec::with_capacity(step);
                for i in 0..step {
                    xx.push(&x_start + &x_step * float_new!(x + i as u32))
                }
                let yy = vec![float_new!(&yy); step];

                let mut out = vec![Bound::Bounded; step];
                T::check_bounded(&xx, &yy, settings.bounds, &mut out);
                output.append(&mut out);
            }
            let row: Vec<(i16, pixels::Color)> = output
                .iter()
                .map(|pixel| match pixel {
                    Bound::Bounded => {
                        (y as i16, pixels::Color::RGB(0_u8, 0_u8, 0_u8))
                    }
                    Bound::Unbounded(n) => {
                        let color = palette::Hsv::new(
                            palette::RgbHue::from((n / color_step) as f64),
                            0.7,
                            0.7,
                        )
                        .shift_hue(palette::RgbHue::from(hue_shift as f64));

                        let rgb = palette::Rgb::from(color);

                        (
                            y as i16,
                            pixels::Color::RGB(
                                (rgb.red * 255.0) as u8,
                                (rgb.green * 255.0) as u8,
                                (rgb.blue * 255.0) as u8,
                            ),
                        )
                    }
                })
                .collect();
            tx.send(row).unwrap();
        });
    }

    canvas
        .with_texture_canvas(&mut target_texture, |texture_canvas| {
            for n in 0..settings.height {
                let k = rx.recv().unwrap();
                for (i, j) in k.iter().enumerate() {
                    texture_canvas.pixel(i as i16, j.0, j.1).unwrap();
                }
            }
        })
        .unwrap();

    target_texture
}

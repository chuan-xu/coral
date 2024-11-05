//! generate captcha

use image::{DynamicImage, GenericImage, ImageEncoder};
use std::{
    path::Path,
    str::Chars,
    time::{SystemTime, UNIX_EPOCH},
};

/// 文字点选验证码
#[derive(Default)]
pub struct TxtClickCaptchaBuilder {
    /// 展示的点选文字
    total_chars: Vec<char>,

    /// 需要点选的文字顺序
    sel_chars: Vec<usize>,

    /// 文字块大小
    txt_size: f32,

    /// 文字框宽度
    box_width: f32,

    /// 文字框高度
    box_height: f32,
}

impl TxtClickCaptchaBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_total_chars(mut self, chars: Chars) -> Self {
        for c in chars {
            self.total_chars.push(c.to_owned())
        }
        self
    }

    pub fn set_sel_chars(mut self, sel: &[usize]) -> CaptchaResult<Self> {
        if sel.len() > self.total_chars.len() {
            return Err(CaptchaError::InvalidTxtNums);
        }
        for s in sel {
            if *s > self.total_chars.len() {
                return Err(CaptchaError::InvalidTxtNums);
            }
            self.sel_chars.push(*s);
        }
        Ok(self)
    }

    pub fn set_txt_size(mut self, size: f32) -> Self {
        self.txt_size = size;
        self
    }

    pub fn set_box_size(mut self, width: f32, height: f32) -> Self {
        self.box_width = width;
        self.box_height = height;
        self
    }

    fn check(&self) -> CaptchaResult<()> {
        let bounds = (self.txt_size.powi(2) * 2.0).sqrt().ceil();
        if bounds > self.box_width || bounds > self.box_height {
            return Err(CaptchaError::InvalidTxtSize);
        }
        Ok(())
    }

    pub fn open<P: AsRef<Path>>(self, path: P) -> CaptchaResult<TxtClickCaptcha> {
        self.check()?;
        let buf = image::open(path)?;
        let width = self.total_chars.len() as u32 * self.box_width as u32;
        let buf = buf.resize_exact(
            width,
            self.box_height as u32,
            image::imageops::FilterType::Nearest,
        );
        Ok(TxtClickCaptcha {
            total_chars: self.total_chars,
            sel_chars: self.sel_chars,
            txt_size: self.txt_size,
            box_width: self.box_width,
            box_height: self.box_height,
            buf,
        })
    }

    pub fn rgba8(self) -> CaptchaResult<TxtClickCaptcha> {
        self.check()?;
        let width = self.total_chars.len() as u32 * self.box_width as u32;
        let buf = image::DynamicImage::new_rgba8(width, self.box_height as u32);
        Ok(TxtClickCaptcha {
            total_chars: self.total_chars,
            sel_chars: self.sel_chars,
            txt_size: self.txt_size,
            box_width: self.box_width,
            box_height: self.box_height,
            buf,
        })
    }
}

/// 文字点选验证码
pub struct TxtClickCaptcha {
    /// 展示的点选文字
    total_chars: Vec<char>,

    /// 需要点选的文字顺序
    sel_chars: Vec<usize>,

    /// 文字块大小
    txt_size: f32,

    /// 文字框宽度
    box_width: f32,

    /// 文字框高度
    box_height: f32,

    /// 图片验证码
    buf: image::DynamicImage,
}

struct CaptchaChar {
    /// 文字的位置信息
    metrics: fontdue::Metrics,

    /// 文字的位图信息
    glyph_vec: Vec<u8>,

    /// 外部偏移量
    outter_offset: f32,

    /// 内部宽度偏移量
    width_offset: f32,

    /// 内部高度偏移量
    height_offset: f32,

    /// 偏转角度
    rotation: f32,
}

impl CaptchaChar {
    fn new(f: &fontdue::Font, px: f32, ch: char) -> Self {
        let (metrics, glyph_vec) = f.rasterize(ch, px);
        Self {
            metrics,
            glyph_vec,
            outter_offset: 0.0,
            width_offset: 0.0,
            height_offset: 0.0,
            rotation: 0.0,
        }
    }

    fn set_outter_offset(&mut self, offset: f32) {
        self.outter_offset = offset;
    }

    fn set_width_offset(&mut self, offset: f32) {
        self.width_offset = offset;
    }

    fn set_height_offset(&mut self, offset: f32) {
        self.height_offset = offset;
    }

    fn set_rotation(&mut self, rotation: f32) {
        self.rotation = rotation;
    }

    fn rotation_trans(&self, x: f32, y: f32, radians: f32) -> (u32, u32) {
        let nx = (x * radians.cos() - y * radians.sin()) + self.outter_offset + self.width_offset;
        let ny = (x * radians.sin() + y * radians.cos()) + self.height_offset;
        (nx.ceil() as u32, ny.ceil() as u32)
    }

    fn fill(&self, img: &mut DynamicImage, color: &[u8]) -> [(u32, u32); 4] {
        let angle_radians = self.rotation * std::f32::consts::PI / 180.0;
        let width_start = -cal_start_index(self.metrics.width);
        let mut height_start = -cal_start_index(self.metrics.height);
        let mut i = 0;
        // 计算4个顶点位置
        let mut bounds = [(0, 0); 4];
        bounds[0] = self.rotation_trans(width_start, height_start, angle_radians);
        bounds[1] = self.rotation_trans(
            width_start + self.metrics.width as f32,
            height_start,
            angle_radians,
        );
        bounds[2] = self.rotation_trans(
            width_start,
            height_start + self.metrics.height as f32,
            angle_radians,
        );
        bounds[3] = self.rotation_trans(
            width_start + self.metrics.width as f32,
            height_start + self.metrics.height as f32,
            angle_radians,
        );

        // 渲染
        for _ in 0..self.metrics.height {
            let mut tmp_ws = width_start;
            for _ in 0..self.metrics.width {
                let (x, y) = self.rotation_trans(tmp_ws, height_start, angle_radians);
                img.put_pixel(
                    x,
                    y,
                    image::Rgba([color[0], color[1], color[2], self.glyph_vec[i]]),
                );
                tmp_ws += 1.0;
                i += 1;
            }
            height_start += 1.0;
        }
        bounds
    }
}

/// 将中心移至原点后的坐标
fn cal_start_index(n: usize) -> f32 {
    if n % 2 == 0 {
        (n / 2) as f32 - 0.5
    } else {
        (n / 2) as f32
    }
}

impl TxtClickCaptcha {
    pub fn save(self) {
        // self.buf.save_with_format(, )
        // self.buf.save_with_format(, )
        // self.buf.into_bytes()
    }

    pub fn generate(
        mut self,
        font: fontdue::Font,
    ) -> CaptchaResult<(Vec<u8>, Vec<char>, Vec<[(u32, u32); 4]>)> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let expand_size = (self.txt_size.powi(2) * 2.0).sqrt().trunc();
        let box_width_half = self.box_width / 2.0;
        let box_height_half = self.box_height / 2.0;
        let max_width_offset = self.box_width - expand_size;
        let max_height_offset = self.box_height - expand_size;
        let max_width_offset_half = max_width_offset / 2.0;
        let max_height_offset_half = max_height_offset / 2.0;

        // 随机偏移量
        let rand_width_offsets = lcg_f32(now, self.total_chars.len(), max_width_offset as u64 + 1);
        let rand_height_offsets =
            lcg_f32(now, self.total_chars.len(), max_height_offset as u64 + 1);
        let radians = lcg_f32(now, self.total_chars.len(), 361);
        let colors = lcg_u8(now, self.total_chars.len() * 3, 256);
        let mut i = 0;
        let mut bounds = Vec::with_capacity(self.total_chars.len());
        for c in self.total_chars.iter() {
            let mut ch = CaptchaChar::new(&font, self.txt_size, *c);
            ch.set_outter_offset(i as f32 * self.box_width);
            ch.set_width_offset(box_width_half + max_width_offset_half - rand_width_offsets[i]);
            ch.set_height_offset(box_height_half + max_height_offset_half - rand_height_offsets[i]);
            ch.set_rotation(radians[i]);
            let bound = ch.fill(&mut self.buf, &colors[i..i + 3]);
            bounds.push(bound);
            i += 1;
        }
        let mut sel_bounds = Vec::with_capacity(self.sel_chars.len());
        let mut sel_chars = Vec::with_capacity(self.sel_chars.len());
        for sel in self.sel_chars.iter() {
            sel_bounds.push(bounds[*sel]);
            sel_chars.push(self.total_chars[*sel]);
        }
        let mut buf = Vec::new();
        image::codecs::png::PngEncoder::new(&mut buf).write_image(
            self.buf.as_bytes(),
            self.buf.width(),
            self.buf.height(),
            image::ExtendedColorType::Rgba8,
        )?;
        Ok((buf, sel_chars, sel_bounds))
        // for
    }
}

/// 线性同余生成器（LCG）
fn lcg_f32(seed: u64, count: usize, m: u64) -> Vec<f32> {
    let mut numbers = vec![0.0; count];
    let a = 1664525_u64;
    let c = 1013904223_u64;
    let mut x = seed;
    for i in 0..count {
        x = (a.wrapping_mul(x).wrapping_add(c)) % m;
        numbers[i] = x as f32;
    }
    numbers
}

fn lcg_u8(seed: u64, count: usize, m: u64) -> Vec<u8> {
    let mut numbers = vec![0; count];
    let a = 1664525_u64;
    let c = 1013904223_u64;
    let mut x = seed;
    for i in 0..count {
        x = (a.wrapping_mul(x).wrapping_add(c)) % m;
        numbers[i] = x as u8;
    }
    numbers
}

type CaptchaResult<T> = Result<T, CaptchaError>;

#[derive(thiserror::Error, Debug)]
pub enum CaptchaError {
    #[error("Text Number need bigger than Select Number")]
    InvalidTxtNums,

    #[error("Text Size need bigger than Box Size")]
    InvalidTxtSize,

    #[error("image error")]
    ImageError(#[from] image::ImageError),

    #[error("system time error")]
    TimeError(#[from] std::time::SystemTimeError),
}

#[cfg(test)]
mod test {
    use crate::{CaptchaChar, TxtClickCaptchaBuilder};

    #[test]
    fn gen_save() {
        let cs = "你好啊世界".chars();
        let mut img = TxtClickCaptchaBuilder::new()
            .set_total_chars(cs)
            .set_sel_chars(&[1])
            .unwrap()
            .set_txt_size(30.0)
            .set_box_size(100.0, 100.0)
            .rgba8()
            .unwrap();
        let font_settings = fontdue::FontSettings::default();
        let font_data = std::fs::read("/root/host/simkai.ttf").unwrap();
        let font = fontdue::Font::from_bytes(font_data.as_slice(), font_settings).unwrap();
        let max_add = (img.txt_size.powi(2) * 2.0).sqrt().trunc();
        println!("{}", max_add);
        let mut idx = 0.0;
        let color = [255, 0, 0];
        for c in img.total_chars {
            let mut cap_ch = CaptchaChar::new(&font, img.txt_size, c);
            cap_ch.set_outter_offset(idx * img.box_width);
            cap_ch.set_width_offset(img.box_width / 2.0 - 29.0);
            cap_ch.set_height_offset(img.box_height / 2.0 + 29.0);
            cap_ch.set_rotation(45.0);
            cap_ch.fill(&mut img.buf, &color);
            idx += 1.0;
        }
        img.buf
            .save_with_format("/root/host/dist/demo.png", image::ImageFormat::Png)
            .unwrap();
    }

    #[test]
    fn gen_txt_captcha() {
        let cs = "你好啊世界".chars();
        let img = TxtClickCaptchaBuilder::new()
            .set_total_chars(cs)
            .set_sel_chars(&[1])
            .unwrap()
            .set_txt_size(30.0)
            .set_box_size(60.0, 60.0)
            .rgba8()
            .unwrap();
        let font_settings = fontdue::FontSettings::default();
        let font_data = std::fs::read("/root/host/simkai.ttf").unwrap();
        let font = fontdue::Font::from_bytes(font_data.as_slice(), font_settings).unwrap();
        let (buf, _, _) = img.generate(font).unwrap();
        std::fs::write("/root/host/dist/demo.png", buf).unwrap();
        // img.buf
        //     .save_with_format("/root/host/dist/demo.png", image::ImageFormat::Png)
        //     .unwrap();
    }

    fn rotate_point(x: f64, y: f64, angle_degrees: f64) -> (f64, f64) {
        // 将角度转换为弧度
        let angle_radians = angle_degrees * std::f64::consts::PI / 180.0;

        // 计算旋转后的坐标
        let new_x = x * angle_radians.cos() - y * angle_radians.sin();
        let new_y = x * angle_radians.sin() + y * angle_radians.cos();

        (new_x, new_y)
    }

    #[test]
    fn test_rotation() {
        let (x, y) = rotate_point(2.0, 0.0, 45.0);
        let v = -0.5_f64;
        println!("{}, {} {}", x, y, v.ceil());
    }

    #[test]
    fn test_rand() {
        let num = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let c = num.to_be_bytes();
        println!("{:?}", c);
    }
}

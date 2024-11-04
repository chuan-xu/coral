//! generate captcha

use image::ImageResult;
use std::{path::Path, str::Chars};

/// 文字点选验证码
#[derive(Default)]
pub struct TxtClickCaptchaBuilder {
    /// 展示的点选文字
    total_chars: Vec<char>,

    /// 需要点选的文字顺序
    sel_chars: Vec<usize>,

    /// 文字块大小
    txt_size: u32,

    /// 文字框宽度
    box_width: u32,

    /// 文字框高度
    box_height: u32,
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

    pub fn set_txt_size(mut self, size: u32) -> Self {
        self.txt_size = size;
        self
    }

    pub fn set_box_size(mut self, width: u32, height: u32) -> Self {
        self.box_width = width;
        self.box_height = height;
        self
    }

    fn check(&self) -> CaptchaResult<()> {
        if self.txt_size > self.box_width || self.txt_size > self.box_height {
            return Err(CaptchaError::InvalidTxtSize);
        }
        Ok(())
    }

    pub fn open<P: AsRef<Path>>(self, path: P) -> CaptchaResult<TxtClickCaptcha> {
        self.check()?;
        let buf = image::open(path)?;
        let width = self.total_chars.len() as u32 * self.box_width;
        let buf = buf.resize_exact(width, self.box_height, image::imageops::FilterType::Nearest);
        Ok(TxtClickCaptcha {
            total_chars: self.total_chars,
            sel_chars: self.sel_chars,
            txt_size: self.txt_size,
            box_width: self.box_width,
            box_height: self.box_height,
            width,
            height: self.box_height,
            buf,
        })
    }

    pub fn rgba8(self) -> CaptchaResult<TxtClickCaptcha> {
        self.check()?;
        let width = self.total_chars.len() as u32 * self.box_width;
        let buf = image::DynamicImage::new_rgba8(width, self.box_height);
        Ok(TxtClickCaptcha {
            total_chars: self.total_chars,
            sel_chars: self.sel_chars,
            txt_size: self.txt_size,
            box_width: self.box_width,
            box_height: self.box_height,
            width,
            height: self.box_height,
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
    txt_size: u32,

    /// 文字框宽度
    box_width: u32,

    /// 文字框高度
    box_height: u32,

    /// 验证图片的宽度
    width: u32,

    /// 验证图片的高度
    height: u32,

    /// 图片验证码
    buf: image::DynamicImage,
}

impl TxtClickCaptcha {
    pub fn save(self) {
        // self.buf.save_with_format(, )
        // self.buf.save_with_format(, )
        // self.buf.into_bytes()
    }
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
}

#[cfg(test)]
mod test {
    use nalgebra::{self as na, Const};

    use crate::TxtClickCaptchaBuilder;

    #[test]
    fn gen_save() {
        let c = "你好啊世界".chars();
        let img = TxtClickCaptchaBuilder::new()
            .set_total_chars(c)
            .set_sel_chars(&[1, 3, 4])
            .unwrap()
            .set_txt_size(100)
            .set_box_size(150, 150)
            .rgba8()
            .unwrap();
        img.buf
            .save_with_format("/root/document/captcha/demo.png", image::ImageFormat::Png)
            .unwrap();
    }

    #[test]
    fn test_rotation() {
        // let m = vec![vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8]];
        // let row = 255;
        // let col = 255;
        // let r1 = Ro
        // na::Matrix::from_rows()
        let point = na::Vector2::new(2.0, 0.0);
        let angle = -std::f64::consts::PI / 4.0;
        let rotation = na::Rotation2::new(angle);
        let npoint = rotation * point;
        println!("{:?}", npoint);
    }
}

#![forbid(unsafe_code)]

use core::ops::Range;
use rand::prelude::*;
use std::time;

use crate::RmatrixCrosstermRender;

#[derive(Debug, Clone)]
pub struct Rain {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) length: u16,
    pub(crate) speed: u16,

    pub(crate) symbl_pos: u16,
    pub(crate) brightnes: Option<f32>,
}

#[derive(Debug, Clone, Default)]
#[repr(transparent)]
pub struct RColor(pub (i16, i16, i16));
impl RColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self::from((r, g, b))
    }
    #[inline]
    pub fn tuple(&self) -> (u8, u8, u8) {
        (
            self.0 .0.clamp(0, u8::MAX as i16) as u8,
            self.0 .1.clamp(0, u8::MAX as i16) as u8,
            self.0 .2.clamp(0, u8::MAX as i16) as u8,
        )
    }
    #[inline]
    pub fn interpolate(&self, dc: f32) -> Self {
        Self((
            (self.0 .0 as f32 * dc) as i16,
            (self.0 .1 as f32 * dc) as i16,
            (self.0 .2 as f32 * dc) as i16,
        ))
    }
    #[inline]
    pub const fn add(&self, lft: &RColor) -> Self {
        Self((
            self.0 .0.saturating_add(lft.0 .0),
            self.0 .1.saturating_add(lft.0 .1),
            self.0 .2.saturating_add(lft.0 .2),
        ))
    }
    #[inline]
    pub const fn sub(&self, lft: &RColor) -> Self {
        Self((
            self.0 .0.saturating_sub(lft.0 .0),
            self.0 .1.saturating_sub(lft.0 .1),
            self.0 .2.saturating_sub(lft.0 .2),
        ))
    }
}

impl From<(u8, u8, u8)> for RColor {
    #[inline]
    fn from(val: (u8, u8, u8)) -> Self {
        Self((val.0 as i16, val.1 as i16, val.2 as i16))
    }
}

impl From<RColor> for (u8, u8, u8) {
    #[inline]
    fn from(val: RColor) -> Self {
        val.tuple()
    }
}

#[derive(Debug, Clone)]
pub struct Rmatrix<R>
where
    R: Rng,
{
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) rains: Vec<Rain>,
    pub(crate) symbls: Vec<char>,

    pub(crate) rng: R,
    pub(crate) speed: Range<u16>,
    pub(crate) len: Range<u16>,

    pub(crate) start_gradient_color: Option<RColor>,
    pub(crate) color: RColor,
    pub(crate) head_color: Option<RColor>,
    pub(crate) interpolate_color_koef: Option<f32>,
    pub(crate) min_brightnes: Option<f32>,
    pub(crate) density: f32,

    pub(crate) is_bold: bool,
    pub(crate) is_default_rain: bool,
    pub(crate) delay: time::Duration,
}

impl Default for Rmatrix<StdRng> {
    #[inline]
    fn default() -> Self {
        let std_rng =
            StdRng::from_rng(thread_rng()).expect("Fail to create StdRng from ThreadRng.");

        let mut rmatrix = Rmatrix {
            width: 0,
            height: 0,
            rains: Vec::new(),
            symbls: vec![],

            rng: std_rng,
            speed: 1..3,
            len: 5..20,

            start_gradient_color: Some(RColor::new(0, 0, 0)),
            color: RColor::new(0, 255, 0),
            head_color: Some(RColor::new(255, 255, 255)),

            interpolate_color_koef: Some(1.5),
            min_brightnes: Some(0.1),
            density: 0.7,

            is_bold: true,
            is_default_rain: true,
            delay: time::Duration::from_millis(16),
        };

        rmatrix.set_utf8();
        rmatrix.rains.reserve_exact(rmatrix.expected_capacity());
        rmatrix.fill_rains();
        rmatrix
    }
}

impl<R> Rmatrix<R>
where
    R: Rng,
{
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.rains = Vec::with_capacity(self.expected_capacity());
        self.fill_rains();
    }

    #[inline]
    pub fn delay(&self) -> time::Duration {
        self.delay
    }

    #[inline]
    pub fn set_utf8(&mut self) {
        self.symbls = vec![
            'ﾊ', 'ﾐ', 'ﾋ', 'ｰ', 'ｳ', 'ｼ', 'ﾅ', 'ﾓ', 'ﾆ', 'ｻ', 'ﾜ', 'ﾂ', 'ｵ', 'ﾘ', 'ｱ', 'ﾎ', 'ﾃ',
            'ﾏ', 'ｹ', 'ﾒ', 'ｴ', 'ｶ', 'ｷ', 'ﾑ', 'ﾕ', 'ﾗ', 'ｾ', 'ﾈ', 'ｽ', 'ﾀ', 'ﾇ', 'ﾍ', 'ﾄ', '0',
            '1', '2', '3', '4', '5', '7', '8', '9', 'Z', ':', '.', '･', '=', '*', '+', '-', '<',
            '>', '¦', '|', '╌', ' ', '\"', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'l', 'k',
            'j', 'a', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
        ];

        assert!(self.symbls.len() <= u16::MAX as usize);
    }
    #[inline]
    pub fn set_ascii(&mut self) {
        self.symbls = vec![
            '0', '1', '2', '3', '4', '5', '7', '8', '9', 'Z', ':', '.', '･', '=', '*', '+', '-',
            '<', '>', '¦', '|', '╌', ' ', '\"', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'l',
            'k', 'j', 'a', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
        ];

        assert!(self.symbls.len() <= u16::MAX as usize);
    }

    #[inline]
    fn fill_rains(&mut self) {
        while self.possible_add() {
            let x = self.rng.gen_range(0..self.width);
            let y = self.rng.gen_range(0..self.height);
            self.add_rain(x, y);
        }
    }
    #[inline]
    fn expected_capacity(&self) -> usize {
        (self.width as f32 * self.density) as usize
    }
    #[inline]
    fn possible_add(&self) -> bool {
        self.rains.len() < self.expected_capacity()
    }
    #[inline]
    pub fn add_rain(&mut self, x: u16, y: u16) {
        let length = self.rng.gen_range(self.len.clone());
        let speed = self.rng.gen_range(self.speed.clone());
        let symbl_pos = self.rng.gen_range(0..self.symbls.len()) as u16;

        let brightnes = if let Some(min_brightnes) = self.min_brightnes {
            Some(self.rng.gen_range(min_brightnes..=1.0))
        } else {
            None
        };

        self.rains.push(Rain {
            x,
            y,
            speed,
            length,
            symbl_pos,
            brightnes,
        });
    }
    pub fn update(&mut self) {
        let mut len = self.rains.len();
        let mut idx = 0;
        while idx < len {
            let head = &mut self.rains[idx];

            if head.y.saturating_sub(head.length) > self.height {
                self.rains.swap_remove(idx);
                len -= 1;
                continue;
            }

            head.y = head.y.saturating_add(head.speed);
            idx += 1;
        }

        while self.possible_add() {
            let x = self.rng.gen_range(0..self.width);
            self.add_rain(x, 0);
        }
    }

    pub(crate) fn to_crossterm_render(&mut self) -> RmatrixCrosstermRender<R> {
        RmatrixCrosstermRender { rmatrix: self }
    }
}

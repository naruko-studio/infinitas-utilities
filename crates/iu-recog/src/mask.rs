//! Binary masks: a rectangle of the screen reduced to "white or not".

use image::RgbImage;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A pixel counts as white when every channel is above this. F12 screenshots
/// and Spout frames differ by at most ±1, far from this threshold.
pub const WHITE_THRESHOLD: u8 = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    width: u32,
    height: u32,
    bits: Vec<bool>,
}

impl Mask {
    pub fn new(width: u32, height: u32, bits: Vec<bool>) -> Self {
        assert_eq!(bits.len(), (width * height) as usize, "mask size mismatch");
        Self {
            width,
            height,
            bits,
        }
    }

    /// Binarizes `img` at (x, y) with this size. Pixels outside the image
    /// count as black.
    pub fn from_image(img: &RgbImage, x: u32, y: u32, width: u32, height: u32) -> Self {
        let bits = (y..y + height)
            .flat_map(|py| (x..x + width).map(move |px| (px, py)))
            .map(|(px, py)| {
                img.get_pixel_checked(px, py)
                    .is_some_and(|p| p.0.iter().all(|&c| c > WHITE_THRESHOLD))
            })
            .collect();
        Self::new(width, height, bits)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn get(&self, x: u32, y: u32) -> bool {
        self.bits[(y * self.width + x) as usize]
    }

    pub fn count_ones(&self) -> u32 {
        self.bits.iter().filter(|&&b| b).count() as u32
    }

    /// Number of differing pixels. Masks must have the same size.
    pub fn distance(&self, other: &Mask) -> u32 {
        assert_eq!(
            (self.width, self.height),
            (other.width, other.height),
            "comparing masks of different sizes"
        );
        self.bits
            .iter()
            .zip(&other.bits)
            .filter(|(a, b)| a != b)
            .count() as u32
    }

    /// Distance between this mask and `img` at (x, y), without allocating.
    pub fn distance_at(&self, img: &RgbImage, x: u32, y: u32) -> u32 {
        let mut d = 0;
        for j in 0..self.height {
            for i in 0..self.width {
                let white = img
                    .get_pixel_checked(x + i, y + j)
                    .is_some_and(|p| p.0.iter().all(|&c| c > WHITE_THRESHOLD));
                d += u32::from(white != self.get(i, j));
            }
        }
        d
    }
}

/// JSON form: `{ "rows": ["..##..", ...] }`, readable and diffable.
#[derive(Serialize, Deserialize)]
struct MaskRepr {
    rows: Vec<String>,
}

impl Serialize for Mask {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let rows = self
            .bits
            .chunks(self.width as usize)
            .map(|row| row.iter().map(|&b| if b { '#' } else { '.' }).collect())
            .collect();
        MaskRepr { rows }.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Mask {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let repr = MaskRepr::deserialize(d)?;
        let width = repr.rows.first().map_or(0, |r| r.len());
        if repr.rows.iter().any(|r| r.len() != width) {
            return Err(serde::de::Error::custom("mask rows differ in length"));
        }
        let bits: Option<Vec<bool>> = repr
            .rows
            .iter()
            .flat_map(|r| r.chars())
            .map(|c| match c {
                '#' => Some(true),
                '.' => Some(false),
                _ => None,
            })
            .collect();
        let bits =
            bits.ok_or_else(|| serde::de::Error::custom("mask rows use only '#' and '.'"))?;
        Ok(Mask::new(width as u32, repr.rows.len() as u32, bits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    fn img() -> RgbImage {
        let mut img = RgbImage::new(4, 3);
        img.put_pixel(1, 1, Rgb([255, 255, 255]));
        img.put_pixel(2, 1, Rgb([200, 129, 129]));
        img.put_pixel(3, 1, Rgb([255, 128, 255])); // one channel at threshold: not white
        img
    }

    #[test]
    fn binarizes_on_all_channels() {
        let m = Mask::from_image(&img(), 0, 0, 4, 3);
        assert_eq!(m.count_ones(), 2);
        assert!(m.get(1, 1) && m.get(2, 1) && !m.get(3, 1));
    }

    #[test]
    fn distance_and_distance_at_agree() {
        let m = Mask::from_image(&img(), 0, 0, 3, 2);
        let empty = Mask::new(3, 2, vec![false; 6]);
        assert_eq!(empty.distance(&m), 2);
        assert_eq!(empty.distance_at(&img(), 0, 0), 2);
        assert_eq!(m.distance_at(&img(), 0, 0), 0);
        // Out-of-bounds pixels count as black.
        assert_eq!(empty.distance_at(&img(), 3, 2), 0);
    }

    #[test]
    fn json_round_trip() {
        let m = Mask::from_image(&img(), 0, 0, 4, 3);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r##"{"rows":["....",".##.","...."]}"##);
        assert_eq!(serde_json::from_str::<Mask>(&json).unwrap(), m);
        assert!(serde_json::from_str::<Mask>(r#"{"rows":["..","..."]}"#).is_err());
    }
}

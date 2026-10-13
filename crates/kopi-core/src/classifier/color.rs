//! Colors: CSS color text, and the `NSColor` archives that color pickers put on the
//! pasteboard.

use std::io::Cursor;

use plist::{Dictionary, Value};

/// Red, green, blue and alpha, 0 to 255.
pub(super) type Rgba = [u8; 4];

/// Longer text isn't one color value.
const MAX_BYTES: usize = 64;

/// `NSID` of the sRGB color space inside an `NSColor` archive.
const SRGB_SPACE_ID: u64 = 7;

/// The normalised form: `#RRGGBBAA`, uppercase.
pub(super) fn hex([r, g, b, a]: Rgba) -> String {
    format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
}

/// The color, if the whole of `text` is one CSS color: `#RGB`, `#RGBA`, `#RRGGBB`,
/// `#RRGGBBAA`, `rgb()`, `rgba()`, `hsl()` or `hsla()`. Named colors ("red") stay text.
pub(super) fn parse(text: &str) -> Option<Rgba> {
    if text.len() > MAX_BYTES {
        return None;
    }
    if let Some(digits) = text.strip_prefix('#') {
        return parse_hex(digits);
    }
    let lower = text.to_ascii_lowercase();
    let (name, args) = lower.strip_suffix(')')?.split_once('(')?;
    let (channels, alpha) = split_args(args)?;
    let [r, g, b] = match name {
        "rgb" | "rgba" => [
            rgb_channel(channels[0])?,
            rgb_channel(channels[1])?,
            rgb_channel(channels[2])?,
        ],
        "hsl" | "hsla" => hsl(channels)?,
        _ => return None,
    };
    let a = alpha.map_or(Some(1.0), alpha_value)?;
    Some([r, g, b, unit_to_byte(a)])
}

fn parse_hex(digits: &str) -> Option<Rgba> {
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |i: usize| u8::from_str_radix(&digits[i..=i], 16).ok().map(|n| n * 17);
    let byte = |i: usize| u8::from_str_radix(&digits[i * 2..i * 2 + 2], 16).ok();
    match digits.len() {
        3 => Some([nibble(0)?, nibble(1)?, nibble(2)?, 255]),
        4 => Some([nibble(0)?, nibble(1)?, nibble(2)?, nibble(3)?]),
        6 => Some([byte(0)?, byte(1)?, byte(2)?, 255]),
        8 => Some([byte(0)?, byte(1)?, byte(2)?, byte(3)?]),
        _ => None,
    }
}

/// The three channels and the alpha, from `30, 144, 255`, `30, 144, 255, 0.5` or
/// `30 144 255 / 50%`.
fn split_args(args: &str) -> Option<([&str; 3], Option<&str>)> {
    let (main, slash_alpha) = match args.split_once('/') {
        Some((main, alpha)) => (main, Some(alpha.trim())),
        None => (args, None),
    };
    let commas = main.contains(',');
    let parts: Vec<&str> = if commas {
        main.split(',').map(str::trim).collect()
    } else {
        main.split_whitespace().collect()
    };
    match (parts.as_slice(), slash_alpha) {
        (&[a, b, c], alpha) => Some(([a, b, c], alpha)),
        (&[a, b, c, alpha], None) if commas => Some(([a, b, c], Some(alpha))),
        _ => None,
    }
}

fn number(text: &str) -> Option<f64> {
    text.parse::<f64>().ok().filter(|n| n.is_finite())
}

fn percent(text: &str) -> Option<f64> {
    number(text.strip_suffix('%')?).map(|p| p / 100.0)
}

fn rgb_channel(text: &str) -> Option<u8> {
    let unit = match percent(text) {
        Some(unit) => unit,
        None => number(text)? / 255.0,
    };
    Some(unit_to_byte(unit))
}

fn alpha_value(text: &str) -> Option<f64> {
    percent(text).or_else(|| number(text))
}

fn hsl([hue, saturation, lightness]: [&str; 3]) -> Option<[u8; 3]> {
    let hue = number(hue.strip_suffix("deg").unwrap_or(hue))?;
    let s = percent(saturation)?.clamp(0.0, 1.0);
    let l = percent(lightness)?.clamp(0.0, 1.0);
    let h = hue.rem_euclid(360.0) / 360.0;
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let channel = |t: f64| {
        let t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        unit_to_byte(v)
    };
    Some([channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0)])
}

fn unit_to_byte(unit: f64) -> u8 {
    (unit.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The color in a `com.apple.cocoa.pasteboard.color` archive (an `NSKeyedArchiver` plist).
///
/// An `NSColor` keeps its components as ASCII numbers. sRGB colors, the ones the color panel
/// makes, hold them in `NSComponents` and an approximation in a calibrated space in `NSRGB`.
/// Older colors have only `NSRGB` or `NSWhite`, and catalog colors such as `systemBlue` wrap
/// the resolved color in `NSColor`. Colors outside sRGB (Display P3) come out as their
/// `NSRGB` approximation.
pub(super) fn from_archive(data: &[u8]) -> Option<Rgba> {
    let archive = Value::from_reader(Cursor::new(data)).ok()?;
    let archive = archive.as_dictionary()?;
    let objects = archive.get("$objects")?.as_array()?;
    let root = archive
        .get("$top")?
        .as_dictionary()?
        .get("root")?
        .as_uid()?;
    archived_color(objects, root.get(), 0)
}

fn archived_color(objects: &[Value], index: u64, depth: u8) -> Option<Rgba> {
    let color = object(objects, index)?;
    let components = numbers(color, "NSComponents");
    let srgb = color
        .get("NSCustomColorSpace")
        .and_then(Value::as_uid)
        .and_then(|space| object(objects, space.get()))
        .and_then(|space| space.get("NSID"))
        .and_then(Value::as_unsigned_integer)
        == Some(SRGB_SPACE_ID);

    let unit = match (
        &components,
        numbers(color, "NSRGB"),
        numbers(color, "NSWhite"),
    ) {
        (Some(c), _, _) if srgb && c.len() == 4 => [c[0], c[1], c[2], c[3]],
        (Some(c), _, _) if c.len() == 2 => [c[0], c[0], c[0], c[1]],
        (_, Some(rgb), _) if rgb.len() >= 3 => {
            let alpha = rgb.get(3).or_else(|| components.as_ref()?.last()).copied();
            [rgb[0], rgb[1], rgb[2], alpha.unwrap_or(1.0)]
        }
        (_, _, Some(white)) if !white.is_empty() => {
            let w = white[0];
            [w, w, w, white.get(1).copied().unwrap_or(1.0)]
        }
        _ if depth < 4 => {
            let inner = color.get("NSColor")?.as_uid()?;
            return archived_color(objects, inner.get(), depth + 1);
        }
        _ => return None,
    };
    Some(unit.map(unit_to_byte))
}

fn object(objects: &[Value], index: u64) -> Option<&Dictionary> {
    objects.get(usize::try_from(index).ok()?)?.as_dictionary()
}

/// `b"0.5 0.25 0 1\0"` as numbers.
fn numbers(color: &Dictionary, key: &str) -> Option<Vec<f64>> {
    let text = std::str::from_utf8(color.get(key)?.as_data()?).ok()?;
    text.trim_end_matches('\0')
        .split_whitespace()
        .map(number)
        .collect()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use plist::Uid;

    #[test]
    fn css_colors() {
        let cases: &[(&str, Option<Rgba>)] = &[
            ("#1E90FF", Some([30, 144, 255, 255])),
            ("#1e90ff", Some([30, 144, 255, 255])),
            ("#1E90FF80", Some([30, 144, 255, 128])),
            ("#fff", Some([255, 255, 255, 255])),
            ("#f008", Some([255, 0, 0, 136])),
            ("rgb(30, 144, 255)", Some([30, 144, 255, 255])),
            ("RGB(30,144,255)", Some([30, 144, 255, 255])),
            ("rgba(30, 144, 255, 0.5)", Some([30, 144, 255, 128])),
            ("rgb(30 144 255 / 50%)", Some([30, 144, 255, 128])),
            ("rgb(100%, 0%, 50%)", Some([255, 0, 128, 255])),
            ("rgb(300, -5, 12.6)", Some([255, 0, 13, 255])),
            ("hsl(0, 100%, 50%)", Some([255, 0, 0, 255])),
            ("hsl(120deg 100% 25%)", Some([0, 128, 0, 255])),
            ("hsla(210, 100%, 56%, 0.25)", Some([31, 143, 255, 64])),
            ("hsl(-120, 100%, 50%)", Some([0, 0, 255, 255])),
            ("hsl(0, 0%, 50%)", Some([128, 128, 128, 255])),
            // Not colors.
            ("", None),
            ("#", None),
            ("#12", None),
            ("#12345", None),
            ("#GGGGGG", None),
            ("1E90FF", None),
            ("#1E90FF is my favourite", None),
            ("red", None),
            ("rgb(1, 2)", None),
            ("rgb(1, 2, 3, 4, 5)", None),
            ("rgb(a, b, c)", None),
            ("rgb(1 2 3 4)", None),
            ("hsl(0, 100, 50)", None),
            ("cmyk(0, 0, 0, 0)", None),
            ("rgb(inf, 0, 0)", None),
            ("#issue-42", None),
        ];
        for (text, want) in cases {
            assert_eq!(parse(text), *want, "{text:?}");
        }
    }

    #[test]
    fn normalised_hex() {
        assert_eq!(hex([30, 144, 255, 255]), "#1E90FFFF");
        assert_eq!(hex([0, 0, 0, 0]), "#00000000");
    }

    /// `NSColor(calibratedRed: 0.5, green: 0.25, blue: 0, alpha: 0.5)` as macOS 27 writes it.
    const CALIBRATED: &str = "62706c6973743030d4010203040506070a582476657273696f6e592461726368697665725424746f7058246f626a6563747312000186a05f100f4e534b657965644172636869766572d1080954726f6f748001a30b0c1355246e756c6cd30d0e0f101112554e535247425c4e53436f6c6f7253706163655624636c6173734f100f302e3520302e3235203020302e350010018002d2141516175a24636c6173736e616d655824636c6173736573574e53436f6c6f72a21618584e534f626a65637408111a24293237494c5153575d646a777e90929499a4adb5b800000000000001010000000000000019000000000000000000000000000000c1";

    /// `NSColor(deviceRed: 1, green: 0, blue: 0, alpha: 1)`: no alpha in `NSRGB`.
    pub(in crate::classifier) const DEVICE: &str = "62706c6973743030d4010203040506070a582476657273696f6e592461726368697665725424746f7058246f626a6563747312000186a05f100f4e534b657965644172636869766572d1080954726f6f748001a30b0c1355246e756c6cd30d0e0f101112554e535247425c4e53436f6c6f7253706163655624636c6173734631203020300010028002d2141516175a24636c6173736e616d655824636c6173736573574e53436f6c6f72a21618584e534f626a65637408111a24293237494c5153575d646a777e8587898e99a2aaad00000000000001010000000000000019000000000000000000000000000000b6";

    pub(in crate::classifier) fn unhex(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    /// An archive with `objects` after `$null`, rooted at the first.
    fn archive(objects: Vec<Value>) -> Vec<u8> {
        let mut all = vec![Value::String("$null".into())];
        all.extend(objects);
        let mut top = Dictionary::new();
        top.insert("root".into(), Value::Uid(Uid::new(1)));
        let mut root = Dictionary::new();
        root.insert("$archiver".into(), "NSKeyedArchiver".into());
        root.insert("$version".into(), 100_000.into());
        root.insert("$top".into(), top.into());
        root.insert("$objects".into(), all.into());
        let mut out = Vec::new();
        Value::Dictionary(root).to_writer_binary(&mut out).unwrap();
        out
    }

    fn dict(entries: &[(&str, Value)]) -> Value {
        Value::Dictionary(
            entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
        )
    }

    fn data(text: &str) -> Value {
        Value::Data(text.as_bytes().to_vec())
    }

    #[test]
    fn archived_colors() {
        let srgb = archive(vec![
            dict(&[
                ("NSComponents", data("0.1176470588 0.5647058824 1 1")),
                ("NSRGB", data("0.1077078134 0.4726666212 0.9984526634\0")),
                ("NSColorSpace", 1.into()),
                ("NSCustomColorSpace", Value::Uid(Uid::new(2))),
            ]),
            dict(&[("NSID", 7.into())]),
        ]);
        let gray = archive(vec![
            dict(&[
                ("NSComponents", data("0.5 1")),
                ("NSWhite", data("0.4246723652\0")),
                ("NSColorSpace", 3.into()),
                ("NSCustomColorSpace", Value::Uid(Uid::new(2))),
            ]),
            dict(&[("NSID", 9.into())]),
        ]);
        let old_gray = archive(vec![dict(&[("NSWhite", data("0 0.5\0"))])]);
        let catalog = archive(vec![
            dict(&[
                ("NSColorSpace", 6.into()),
                ("NSColor", Value::Uid(Uid::new(2))),
            ]),
            dict(&[("NSRGB", data("0 0 1\0")), ("NSColorSpace", 1.into())]),
        ]);
        let display_p3 = archive(vec![
            dict(&[
                ("NSComponents", data("1 0 0 0.5")),
                ("NSRGB", data("1 0 0\0")),
                ("NSCustomColorSpace", Value::Uid(Uid::new(2))),
            ]),
            dict(&[("NSSpaceID", 12.into())]),
        ]);
        let pattern = archive(vec![dict(&[("NSColorSpace", 10.into())])]);

        let cases: &[(&str, Vec<u8>, Option<Rgba>)] = &[
            ("calibrated", unhex(CALIBRATED), Some([128, 64, 0, 128])),
            ("device", unhex(DEVICE), Some([255, 0, 0, 255])),
            ("srgb", srgb, Some([30, 144, 255, 255])),
            ("gray", gray, Some([128, 128, 128, 255])),
            ("old gray", old_gray, Some([0, 0, 0, 128])),
            ("catalog", catalog, Some([0, 0, 255, 255])),
            ("display p3", display_p3, Some([255, 0, 0, 128])),
            ("pattern", pattern, None),
            ("not a plist", b"#FF0000".to_vec(), None),
            ("empty", Vec::new(), None),
        ];
        for (name, bytes, want) in cases {
            assert_eq!(from_archive(bytes), *want, "{name}");
        }
    }
}

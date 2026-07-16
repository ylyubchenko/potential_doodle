use bevy::{asset::Assets, asset::Handle, image::Image};

/// Rasterize an SVG (white, 2x for crisp scaling) into a Bevy image.
/// Bevy has no native SVG support, so icons are rendered with resvg at
/// startup.
pub fn svg_icon(images: &mut Assets<Image>, svg_source: &str) -> Handle<Image> {
  use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

  // Whiten icons regardless of how the source colors strokes/fills.
  let svg = svg_source
      .replace("currentColor", "#ffffff")
      .replace("#000000", "#ffffff");
  let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())
      .expect("icon svg parses");
  let size = 48u32;
  let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("pixmap");
  let scale = size as f32 / tree.size().width();
  resvg::render(
      &tree,
      resvg::tiny_skia::Transform::from_scale(scale, scale),
      &mut pixmap.as_mut(),
  );
  // tiny-skia produces premultiplied alpha; Bevy expects straight.
  let mut data = pixmap.take();
  for px in data.chunks_exact_mut(4) {
      let a = px[3] as u32;
      if a > 0 {
          px[0] = (px[0] as u32 * 255 / a).min(255) as u8;
          px[1] = (px[1] as u32 * 255 / a).min(255) as u8;
          px[2] = (px[2] as u32 * 255 / a).min(255) as u8;
      }
  }
  images.add(Image::new(
      Extent3d {
          width: size,
          height: size,
          depth_or_array_layers: 1,
      },
      TextureDimension::D2,
      data,
      TextureFormat::Rgba8UnormSrgb,
      Default::default(),
  ))
}

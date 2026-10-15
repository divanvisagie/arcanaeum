//! Renders `packaging/linux/arcanaeum.svg` to PNG: `cargo run --example
//! render_icon -- <size> <out.png>`. Used by `make icons`, so building the
//! icons needs no system SVG tools.

fn main() {
    let mut args = std::env::args().skip(1);
    let size: u32 = args
        .next()
        .and_then(|s| s.parse().ok())
        .expect("usage: render_icon <size> <out.png>");
    let out = args.next().expect("usage: render_icon <size> <out.png>");
    let svg =
        std::fs::read("packaging/linux/arcanaeum.svg").expect("read packaging/linux/arcanaeum.svg");
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default())
        .expect("parse the SVG");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("pixmap");
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png(&out).expect("write the PNG");
    println!("{out}");
}

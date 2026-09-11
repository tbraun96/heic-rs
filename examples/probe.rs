//! Print what a HEIF file says about itself, without decoding it.
//!
//! ```text
//! cargo run --example probe -- tests/fixtures/photo-2048.heic
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: probe <file.heic>");
        std::process::exit(2);
    };
    let info = heic_rs::io::probe_file(&path)?;
    println!("file           {path}");
    println!("brand          {:?}", info.brand);
    println!("primary item   {}", info.primary_item);
    println!("size           {}x{}", info.width, info.height);
    println!("coded size     {}x{}", info.coded_width, info.coded_height);
    println!("bit depth      {}", info.bit_depth);
    println!("chroma         {:?}", info.chroma);
    println!("rotation       {} degrees ccw", info.rotation.degrees());
    println!("mirror         {:?}", info.mirror);
    println!("alpha          {}", info.has_alpha);
    match info.grid {
        Some(g) => println!(
            "grid           {} rows x {} columns of {}x{}",
            g.rows, g.columns, g.tile_width, g.tile_height
        ),
        None => println!("grid           no (single coded item)"),
    }
    println!("exif           {}", info.has_exif);
    println!("icc            {}", info.has_icc);
    println!("xmp            {}", info.has_xmp);
    Ok(())
}

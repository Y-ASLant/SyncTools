use std::fs::File;
use std::io::Write;

fn main() {
    let mut ico = vec![];

    // ICO header: Reserved(2), Type(2)=1 for ICO, Count(2)=1
    ico.extend_from_slice(&[0u8, 0, 1, 0, 1, 0]);

    // Directory entry: Width(1), Height(1), Colors(1), Reserved(1)=0, Planes(2)=1, BPP(2)=32, Size(4)=40, Offset(4)=22
    ico.extend_from_slice(&[16u8, 0, 0, 0, 1, 0, 32, 0]);
    ico.extend_from_slice(&40u32.to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());

    // DIB header (BITMAPINFOHEADER)
    ico.extend_from_slice(&40u32.to_le_bytes()); // biSize
    ico.extend_from_slice(&1u32.to_le_bytes());  // biWidth
    ico.extend_from_slice(&2u32.to_le_bytes());  // biHeight (double for XOR+AND masks)
    ico.extend_from_slice(&1u16.to_le_bytes());  // biPlanes
    ico.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    ico.extend_from_slice(&0u32.to_le_bytes());  // biCompression
    ico.extend_from_slice(&0u32.to_le_bytes());  // biSizeImage
    ico.extend_from_slice(&0u32.to_le_bytes());  // biXPelsPerMeter
    ico.extend_from_slice(&0u32.to_le_bytes());  // biYPelsPerMeter
    ico.extend_from_slice(&0u32.to_le_bytes());  // biClrUsed
    ico.extend_from_slice(&0u32.to_le_bytes());  // biClrImportant

    // XOR mask (1 pixel, 32-bit ARGB) - transparent black
    ico.extend_from_slice(&0u32.to_le_bytes());

    // AND mask (1 row, 1 bit per pixel, padded to 4 bytes)
    ico.extend_from_slice(&[0u8, 0, 0, 0]);

    let mut f = File::create(r"F:\demo\SyncTools\src-tauri\icons\icon.ico").unwrap();
    f.write_all(&ico).unwrap();
    println!("Generated icon.ico: {} bytes", ico.len());
}

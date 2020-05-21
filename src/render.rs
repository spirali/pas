use crate::aset::AutomaticSet;
use crate::words::{get_max, iterate_words};
use std::fs::File;
use std::io::BufWriter;
use crate::dfa::Dfa;

pub fn render_set(dfa: &Dfa, path: &std::path::Path) {
    assert_eq!(dfa.n_tracks(), 2);
    let nfa = dfa.clone().to_nfa();
    let size_x = get_max(&nfa, 0).to_limit().unwrap();
    let size_y = get_max(&nfa, 1).to_limit().unwrap();

    let mut data : Vec<u8> = vec![0u8; 3 * size_x * size_y];
    iterate_words(&dfa, None, |output| {
       let idx = 3 * (output[1] * size_x + output[0]);
       data[idx] = 255;
    });

    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, size_x as u32, size_y as u32);
    encoder.set_color(png::ColorType::RGB);
    encoder.set_depth(png::BitDepth::Eight);
     let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&data).unwrap();
}
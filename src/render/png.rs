use std::fs::File;
use std::io::{BufWriter, Write};

use crate::aset::AutomaticSet;
use crate::dfa::Dfa;
use crate::elements::{get_max_value, iterate_elements};

pub fn render_set_png<W: Write>(dfas: &[&Dfa], colors: &[[u8; 3]], writer: &mut W) {
    assert!(dfas.iter().all(|x| x.n_tracks() == 2));
    let nfas: Vec<_> = dfas.iter().map(|dfa| dfa.as_nfa()).collect();
    let size_x: usize = nfas.iter().map(|nfa| get_max_value(nfa, 0).to_limit().unwrap()).max().unwrap();
    let size_y: usize = nfas.iter().map(|nfa| get_max_value(nfa, 1).to_limit().unwrap()).max().unwrap();

    let mut data: Vec<u8> = vec![0u8; 3 * size_x * size_y];

    for (i, dfa) in dfas.iter().enumerate() {
        iterate_elements(&dfa, None, |element| {
            let slice = element.as_slice();
            let idx = 3 * (slice[1] * size_x + slice[0]);
            data[idx] = colors[i][0];
            data[idx + 1] = colors[i][1];
            data[idx + 2] = colors[i][2];
        });
    }

    let mut encoder = png::Encoder::new(writer, size_x as u32, size_y as u32);
    encoder.set_color(png::ColorType::RGB);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&data).unwrap();
}

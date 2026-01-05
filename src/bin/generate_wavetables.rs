use std::fs;
use std::path::Path;

const TABLE_SIZE: usize = 1024;
const MAX_HARMONICS: usize = 32;

fn generate_sine_table() -> [f32; TABLE_SIZE] {
    std::array::from_fn(|i| {
        let phase = (i as f32) * 2.0 * std::f32::consts::PI / (TABLE_SIZE as f32);
        phase.sin()
    })
}

fn generate_cosine_table() -> [f32; TABLE_SIZE] {
    std::array::from_fn(|i| {
        let phase = (i as f32) * 2.0 * std::f32::consts::PI / (TABLE_SIZE as f32);
        phase.cos()
    })
}

fn generate_sawtooth_table() -> [f32; TABLE_SIZE] {
    std::array::from_fn(|i| {
        let phase = (i as f32) * 2.0 * std::f32::consts::PI / (TABLE_SIZE as f32);
        let mut sample = 0.0;
        for h in 1..=MAX_HARMONICS {
            let harmonic_amp = 1.0 / (h as f32);
            sample += harmonic_amp * (phase * h as f32).sin();
        }
        sample * 0.5
    })
}

fn generate_square_table() -> [f32; TABLE_SIZE] {
    std::array::from_fn(|i| {
        let phase = (i as f32) * 2.0 * std::f32::consts::PI / (TABLE_SIZE as f32);
        let mut sample = 0.0;
        for h in (1..=MAX_HARMONICS).step_by(2) {
            let harmonic_amp = 1.0 / (h as f32);
            sample += harmonic_amp * (phase * h as f32).sin();
        }
        sample * 0.8
    })
}

fn generate_triangle_table() -> [f32; TABLE_SIZE] {
    std::array::from_fn(|i| {
        let phase = (i as f32) * 2.0 * std::f32::consts::PI / (TABLE_SIZE as f32);
        let mut sample = 0.0;
        for h in (1..=MAX_HARMONICS).step_by(2) {
            let harmonic_amp = 1.0 / ((h * h) as f32);
            let sign = if ((h - 1) / 2) % 2 == 0 { 1.0 } else { -1.0 };
            sample += sign * harmonic_amp * (phase * h as f32).sin();
        }
        sample * 0.8
    })
}

fn write_table_file(table: &[f32; TABLE_SIZE], filename: &str) -> std::io::Result<()> {
    let content = format!(
        "[\n{}\n]",
        table
            .chunks(8)
            .map(|chunk| {
                format!(
                    "    {}",
                    chunk
                        .iter()
                        .map(|&f| format!("{:.10}", f))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(",\n")
    );

    fs::write(filename, content)
}

fn main() -> std::io::Result<()> {
    let wavetables_dir = Path::new("wavetables");
    fs::create_dir_all(wavetables_dir)?;

    println!("Generating wavetables...");

    let sine_table = generate_sine_table();
    write_table_file(&sine_table, "wavetables/sine_1024.dat")?;
    println!("Generated sine_1024.dat");

    let cosine_table = generate_cosine_table();
    write_table_file(&cosine_table, "wavetables/cosine_1024.dat")?;
    println!("Generated cosine_1024.dat");

    let sawtooth_table = generate_sawtooth_table();
    write_table_file(&sawtooth_table, "wavetables/sawtooth_1024.dat")?;
    println!("Generated sawtooth_1024.dat");

    let square_table = generate_square_table();
    write_table_file(&square_table, "wavetables/square_1024.dat")?;
    println!("Generated square_1024.dat");

    let triangle_table = generate_triangle_table();
    write_table_file(&triangle_table, "wavetables/triangle_1024.dat")?;
    println!("Generated triangle_1024.dat");

    println!("All wavetables generated successfully!");
    Ok(())
}

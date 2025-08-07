pub type ZobristTable = ([([[u64; 2]; 64], [u64; 2]); 64], [u64; 2]);

fn generate_zobrist_table() -> ZobristTable {
    let mut table = ([([[0u64; 2]; 64], [0u64; 2]); 64], [0u64; 2]);

    for i in 0..64 {
        for j in 0..64 {
            for k in 0..2 {
                table.0[i].0[j][k] = rand::random();
            }
        }
        for k in 0..2 {
            table.0[i].1[k] = rand::random();
        }
    }

    for i in 0..2 {
        table.1[i] = rand::random();
    }

    table
}

fn main() {
    let zobrist_table = generate_zobrist_table();
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("zobrist.rs");
    std::fs::write(
        &dest_path,
        format!(
            "pub const ZOBRIST_TABLE: ZobristTable = {:?};",
            zobrist_table
        ),
    )
    .unwrap();

    println!("cargo::rerun-if-changed=build.rs");
}

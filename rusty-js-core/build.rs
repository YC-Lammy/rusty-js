use std::io::Read;



fn main() {
    let f = std::fs::read_dir("./llvm");
    
    let rustc  = std::env::var("RUSTC").unwrap();

    let cmd = std::process::Command::new(rustc)
    .arg("--version")
    .output().unwrap();

    let v = String::from_utf8_lossy(&cmd.stdout);

    if !v.contains("1.62."){
        panic!("Building this crate requires a ructc version of 1.62")
    }

    // install llvm-13.0.0
    if f.is_err(){
        let reader = ureq::get("https://github.com/YC-Lammy/llvm-built/releases/download/13.0.0/llvm-13.0.0.tar.gz")
        .call()
        .unwrap()
        .into_reader();

        let decoder = flate2::read::GzDecoder::new(reader);

        let mut ar = tar::Archive::new(decoder);
        ar.unpack("llvm").unwrap();
    }
    println!("cargo:rustc-cfg=nightly")
}

fn main() {
    println!("cargo:rerun-if-changed=src/hixl_c_wrapper.h");
    println!("cargo:rerun-if-changed=src/hixl_c_wrapper.cpp");

    let hixl_include = std::env::current_dir().unwrap().join("hixl/include");

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("src/hixl_c_wrapper.cpp")
        .include(&hixl_include)
        .compile("hixl_wrapper");

    println!("cargo:rustc-link-lib=cann_hixl");
    // Assuming cann_hixl is in LD_LIBRARY_PATH or similar.
    // If user needs to point to specific location, they can set RUSTFLAGS or LIBRARY_PATH.

    let bindings = bindgen::Builder::default()
        .header("src/hixl_c_wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}


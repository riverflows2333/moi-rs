pub fn main() {
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-search=D:\\env\\gurobi1100\\win64\\lib");
        println!("cargo:rustc-link-lib=gurobi110");
        // let bindings = bindgen::Builder::default()
        //     .header("wrapper_win.h")
        //     .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        //     .generate()
        //     .expect("Unable to generate bindings");
        // bindings
        //     .write_to_file("src/grb_bindings_windows_c.rs")
        //     .expect("Couldn't write bindings!");
    } else if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-search=/usr/lib");
        println!("cargo:rustc-link-lib=gurobi");
        // let bindings = bindgen::Builder::default()
        //     .header("wrapper.h")
        //     .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        //     .generate()
        //     .expect("Unable to generate bindings");
        // bindings
        //     .write_to_file("src/grb_bindings_linux_c.rs")
        //     .expect("Couldn't write bindings!");
    } else if cfg!(target_os = "macos") {
    }

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
}

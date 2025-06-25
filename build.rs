use std::process::Command;

fn main() {
    // Only compile Swift bridge on macOS
    if cfg!(target_os = "macos") {
        println!("cargo:rerun-if-changed=src/collectors/gpu/apple_silicon_bridge.swift");
        
        // Compile Swift bridge to object file
        let output = Command::new("swiftc")
            .args(&[
                "-c",
                "-emit-object",
                "-o", "target/gpu_bridge.o",
                "src/collectors/gpu/apple_silicon_bridge.swift",
            ])
            .output()
            .expect("Failed to compile Swift bridge");

        if !output.status.success() {
            panic!("Swift compilation failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Create static library
        let ar_output = Command::new("ar")
            .args(&[
                "rcs",
                "target/libgpu_bridge.a",
                "target/gpu_bridge.o",
            ])
            .output()
            .expect("Failed to create static library");

        if !ar_output.status.success() {
            panic!("Archive creation failed: {}", String::from_utf8_lossy(&ar_output.stderr));
        }

        println!("cargo:rustc-link-search=native=target");
        println!("cargo:rustc-link-lib=static=gpu_bridge");
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
}
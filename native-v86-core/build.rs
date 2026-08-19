fn main() {
    println!("cargo:rerun-if-changed=csrc/softfloat/softfloat.c");
    println!("cargo:rerun-if-changed=csrc/softfloat_shim.c");

    cc::Build::new()
        .file("csrc/softfloat/softfloat.c")
        .file("csrc/softfloat_shim.c")
        .define("SOFTFLOAT_FAST_INT64", None)
        .define("INLINE_LEVEL", Some("5"))
        .define("SOFTFLOAT_FAST_DIV32TO16", None)
        .define("SOFTFLOAT_FAST_DIV64TO32", None)
        .warnings(false)
        .opt_level(2)
        .compile("v86_softfloat");
}

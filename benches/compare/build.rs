use std::{
    env,
    path::{Path, PathBuf},
};

const OPT_LEVEL: u32 = 3;
const APPLE_CC: &str = "/usr/bin/clang";
const APPLE_CXX: &str = "/usr/bin/clang++";
const HOMEBREW_PREFIXES: [&str; 2] = ["/opt/homebrew", "/usr/local"];
const GLIB_MODULES: [&str; 3] = ["gio-2.0", "gobject-2.0", "gmodule-2.0"];
const APPLE_DYLIBS: [&str; 7] = [
    "gio-2.0",
    "gobject-2.0",
    "gmodule-2.0",
    "glib-2.0",
    "intl",
    "iconv",
    "z",
];
const LINUX_DYLIBS: [&str; 2] = ["z", "m"];
const SHIMS: [&str; 6] = [
    "tally.h",
    "utf8.c",
    "gmime.c",
    "vmime.cpp",
    "libetpan.c",
    "dovecot.c",
];

struct Library {
    name: &'static str,
    archive: &'static str,
    static_lib: &'static str,
}

const LIBRARIES: [Library; 6] = [
    Library {
        name: "gmime",
        archive: "lib/libgmime-3.0.a",
        static_lib: "gmime-3.0",
    },
    Library {
        name: "vmime",
        archive: "lib/libvmime.a",
        static_lib: "vmime",
    },
    Library {
        name: "libetpan",
        archive: "lib/libetpan-mime.a",
        static_lib: "etpan-mime",
    },
    Library {
        name: "dovecot",
        archive: "lib/libdovecot-mail.a",
        static_lib: "dovecot-mail",
    },
    Library {
        name: "dovecot",
        archive: "lib/libdovecot-charset.a",
        static_lib: "dovecot-charset",
    },
    Library {
        name: "dovecot",
        archive: "lib/libdovecot-lib.a",
        static_lib: "dovecot-lib",
    },
];

#[derive(Default)]
struct Glib {
    includes: Vec<PathBuf>,
    link_paths: Vec<PathBuf>,
    libs: Vec<String>,
}

impl Glib {
    fn link(&self) {
        for path in &self.link_paths {
            println!("cargo:rustc-link-search=native={}", path.display());
        }
        for lib in &self.libs {
            println!("cargo:rustc-link-lib=dylib={lib}");
        }
    }
}

struct Toolchain {
    apple: bool,
    glib: Glib,
}

impl Toolchain {
    fn detect() -> Toolchain {
        let apple = env::var("CARGO_CFG_TARGET_VENDOR").is_ok_and(|vendor| vendor == "apple");
        for variable in ["GLIB_PREFIX", "GETTEXT_PREFIX", "HOMEBREW_PREFIX"] {
            println!("cargo:rerun-if-env-changed={variable}");
        }
        let glib = if apple {
            homebrew_glib()
        } else {
            pkg_config_glib()
        };
        Toolchain { apple, glib }
    }

    fn c(&self, shim: &Path) -> cc::Build {
        let mut build = cc::Build::new();
        if self.apple && env::var_os("CC").is_none() {
            build.compiler(APPLE_CC);
        }
        build
            .opt_level(OPT_LEVEL)
            .flag("-std=gnu11")
            .include(shim)
            .warnings(false);
        build
    }

    fn cpp(&self, shim: &Path) -> cc::Build {
        let mut build = cc::Build::new();
        build.cpp(true);
        if self.apple && env::var_os("CXX").is_none() {
            build.compiler(APPLE_CXX);
        }
        build
            .opt_level(OPT_LEVEL)
            .std("c++17")
            .include(shim)
            .warnings(false);
        build
    }

    fn with_glib(&self, mut build: cc::Build) -> cc::Build {
        build.includes(&self.glib.includes);
        build
    }
}

fn homebrew_prefix() -> PathBuf {
    env::var_os("HOMEBREW_PREFIX").map_or_else(
        || {
            HOMEBREW_PREFIXES
                .iter()
                .map(PathBuf::from)
                .find(|prefix| prefix.join("opt/glib").is_dir())
                .unwrap_or_else(|| PathBuf::from(HOMEBREW_PREFIXES[0]))
        },
        PathBuf::from,
    )
}

fn homebrew_glib() -> Glib {
    let homebrew = homebrew_prefix();
    let glib = env::var_os("GLIB_PREFIX").map_or_else(|| homebrew.join("opt/glib"), PathBuf::from);
    let gettext =
        env::var_os("GETTEXT_PREFIX").map_or_else(|| homebrew.join("opt/gettext"), PathBuf::from);
    assert!(
        glib.join("include/glib-2.0/glib.h").is_file(),
        "GLib not found in {} (brew install glib, or set GLIB_PREFIX)",
        glib.display()
    );
    Glib {
        includes: vec![
            glib.join("include/glib-2.0"),
            glib.join("lib/glib-2.0/include"),
            gettext.join("include"),
        ],
        link_paths: vec![glib.join("lib"), gettext.join("lib")],
        libs: APPLE_DYLIBS.iter().map(ToString::to_string).collect(),
    }
}

fn pkg_config_glib() -> Glib {
    let mut glib = Glib::default();
    for module in GLIB_MODULES {
        let library = pkg_config::Config::new()
            .cargo_metadata(false)
            .probe(module)
            .unwrap_or_else(|error| panic!("pkg-config cannot find {module}: {error}"));
        glib.includes.extend(library.include_paths);
        glib.link_paths.extend(library.link_paths);
        glib.libs.extend(library.libs);
    }
    glib.libs
        .extend(LINUX_DYLIBS.iter().map(ToString::to_string));
    glib
}

fn main() {
    let manifest =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let prefix = manifest.join("vendor").join("prefix");
    let shim = manifest.join("shim");
    for library in &LIBRARIES {
        let archive = prefix.join(library.name).join(library.archive);
        assert!(
            archive.is_file(),
            "{} is missing: run vendor/build.sh first",
            archive.display()
        );
        println!("cargo:rerun-if-changed={}", archive.display());
    }
    for file in SHIMS {
        println!("cargo:rerun-if-changed={}", shim.join(file).display());
    }

    let toolchain = Toolchain::detect();
    toolchain
        .with_glib(toolchain.c(&shim))
        .file(shim.join("utf8.c"))
        .compile("cmp_utf8");
    toolchain
        .with_glib(toolchain.c(&shim))
        .include(prefix.join("gmime/include/gmime-3.0"))
        .file(shim.join("gmime.c"))
        .compile("cmp_gmime");
    toolchain
        .c(&shim)
        .include(prefix.join("libetpan/include"))
        .flag("-iquote")
        .flag(
            prefix
                .join("libetpan/include/libetpan")
                .display()
                .to_string(),
        )
        .file(shim.join("libetpan.c"))
        .compile("cmp_libetpan");
    toolchain
        .c(&shim)
        .include(prefix.join("dovecot/include"))
        .define("HAVE_CONFIG_H", None)
        .file(shim.join("dovecot.c"))
        .compile("cmp_dovecot");
    toolchain
        .cpp(&shim)
        .include(prefix.join("vmime/include"))
        .define("VMIME_STATIC", None)
        .file(shim.join("vmime.cpp"))
        .compile("cmp_vmime");

    for library in &LIBRARIES {
        println!(
            "cargo:rustc-link-search=native={}",
            prefix.join(library.name).join("lib").display()
        );
        println!("cargo:rustc-link-lib=static={}", library.static_lib);
    }
    toolchain.glib.link();
}

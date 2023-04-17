/* -------------------------------------------------------- *\
 *                                                          *
 *      ███╗░░░███╗░█████╗░░██████╗██╗░░██╗██╗███╗░░██╗     *
 *      ████╗░████║██╔══██╗██╔════╝██║░░██║██║████╗░██║     *
 *      ██╔████╔██║███████║╚█████╗░███████║██║██╔██╗██║     *
 *      ██║╚██╔╝██║██╔══██║░╚═══██╗██╔══██║██║██║╚████║     *
 *      ██║░╚═╝░██║██║░░██║██████╔╝██║░░██║██║██║░╚███║     *
 *      ╚═╝░░░░░╚═╝╚═╝░░╚═╝╚═════╝░╚═╝░░╚═╝╚═╝╚═╝░░╚══╝     *
 *                                         by Nutshimit     *
 * -------------------------------------------------------- *
 *                                                          *
 *   This file is dual-licensed as Apache-2.0 or GPL-3.0.   *
 *   see LICENSE for license details.                       *
 *                                                          *
\* ---------------------------------------------------------*/

pub fn build() {
	if let Ok(version) = std::env::var("CARGO_PKG_VERSION") {
		println!("cargo:rustc-env=MASHIN_PKG_VERSION={}", version);
	}

	if let Ok(name) = std::env::var("CARGO_PKG_NAME") {
		println!("cargo:rustc-env=MASHIN_PKG_NAME={}", name);
	}

	if let Ok(target) = std::env::var("CARGO_MANIFEST_DIR") {
		println!("cargo:rustc-env=TARGET={}", target);
	}
}

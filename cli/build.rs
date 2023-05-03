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
 *  This file is licensed as MIT. See LICENSE for details.  *
 *                                                          *
\* ---------------------------------------------------------*/

fn main() {
	println!("cargo:rustc-env=TARGET={}", std::env::var("TARGET").unwrap());
	println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_commit_hash());
	println!("cargo:rerun-if-env-changed=GIT_COMMIT_HASH");
	println!("cargo:rustc-env=GIT_COMMIT_HASH_SHORT={}", &git_commit_hash()[..7]);
}

fn git_commit_hash() -> String {
	if let Ok(output) =
		std::process::Command::new("git").arg("rev-list").arg("-1").arg("HEAD").output()
	{
		if output.status.success() {
			std::str::from_utf8(&output.stdout[..40]).unwrap().to_string()
		} else {
			// When not in git repository
			// (e.g. when the user install by `cargo install deno`)
			"UNKNOWN".to_string()
		}
	} else {
		// When there is no git command for some reason
		"UNKNOWN".to_string()
	}
}

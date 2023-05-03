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

#![allow(unused_variables, dead_code)]
use super::display::write_to_stdout_ignore_sigpipe;
use crate::Result;
use console::style;
use mashin_runtime::ExecutedResources;
use mashin_sdk::ResourceAction;

macro_rules! skip_empty {
	($res:expr) => {
		match $res {
			Some(val) => val,
			None => continue,
		}
	};
}

pub fn print_diff(executed_resource: &ExecutedResources) -> Result<()> {
	let mut to_add = 0;
	let mut to_update = 0;
	let mut to_remove = 0;
	let all_pending_actions = executed_resource.actions();

	if !all_pending_actions.is_empty() {
		write_to_stdout_ignore_sigpipe(
			"\n\nResource actions are indicated with the following symbols:\n".as_bytes(),
		)?;
		for action in &all_pending_actions {
			match action {
				ResourceAction::Create => {
					to_add += 1;
					write_to_stdout_ignore_sigpipe(
						format!("  {} create\n", style("+").green().bold()).as_bytes(),
					)?;
				},
				ResourceAction::Delete => {
					to_remove += 1;
					write_to_stdout_ignore_sigpipe(
						format!("  {} delete\n", style("-").red().bold()).as_bytes(),
					)?;
				},
				ResourceAction::Update { .. } => {
					to_update += 1;
					write_to_stdout_ignore_sigpipe(
						format!("  {} update\n", style("*").cyan().bold()).as_bytes(),
					)?;
				},
				_ => {},
			}
		}
	}

	write_to_stdout_ignore_sigpipe(
		"\nMashin will perform the following actions:\n\n".to_string().as_bytes(),
	)?;

	for (urn, executed_resource) in executed_resource.iter() {
		let resource_action = skip_empty!(&executed_resource.required_change);
		let resource_diff = skip_empty!(&executed_resource.diff);

		let total_changes = resource_diff.len();
		let mut total_changes_processed = 0;

		let arrow = match &resource_action {
			ResourceAction::Update { .. } => style("   ==>").cyan().bold().to_string(),
			ResourceAction::Create => style("   ==>").green().bold().to_string(),
			ResourceAction::Delete => style("   ==>").red().bold().to_string(),
			_ => "".to_string(),
		};

		let description_line = format!(
			"{arrow} [{}]: Need to be {}\n",
			style(urn.replace("urn:provider:", "")).bold(),
			resource_action.action_past_str().to_lowercase()
		);

		write_to_stdout_ignore_sigpipe(description_line.as_bytes())?;

		for resource_diff in resource_diff.iter() {
			if resource_diff.is_eq() {
				continue
			}

			let closing_line = resource_diff.print_diff()?.unwrap_or_default();

			total_changes_processed += 1;

			if total_changes_processed == total_changes {
				log::info!("   {}", closing_line,);
			}
		}
	}

	Ok(())
}

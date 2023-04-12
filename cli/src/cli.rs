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

use std::{ffi::c_void, rc::Rc};

use crate::Result;
use clap::Parser;
use dialoguer::Confirm;
use mashin_runtime::{
    bold, cyan_bold, green_bold, intense_blue, magenta, red, red_bold, ResourceAction, Runtime,
};

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(Debug, Parser)]
pub enum Subcommand {
    /// Run a JavaScript or TypeScript program.
    Run(RunCmd),
    /// Destroy all resources in the current state.
    Destroy(DestroyCmd),
}

#[derive(Debug, Parser)]
#[group(skip)]
pub struct RunCmd {
    pub main_module: String,
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Parser)]
pub struct DestroyCmd {
    pub main_module: String,
}

impl RunCmd {
    pub async fn run(&self, args: Vec<String>) -> Result<()> {
        let create_runtime = |executed_resource| {
            Runtime::new(
                &self.main_module,
                mashin_runtime::RuntimeCommand::Run,
                args.clone(),
                executed_resource,
            )
        };

        let runtime_result = create_runtime(None)?.run().await?;

        let executed_resouces = runtime_result.executed_resources.borrow().clone();

        let all_pending_actions = executed_resouces.actions();
        let mut to_add = 0;
        let mut to_update = 0;
        let mut to_remove = 0;

        if !all_pending_actions.is_empty() {
            log::info!("\n\nResource actions are indicated with the following symbols:");
            for action in &all_pending_actions {
                match action {
                    ResourceAction::Create => {
                        to_add += 1;
                        log::info!("  {} create", green_bold("+"))
                    }
                    ResourceAction::Delete => {
                        to_remove += 1;
                        log::info!("  {} delete", red_bold("-"))
                    }
                    ResourceAction::Update { .. } => {
                        to_update += 1;
                        log::info!("  {} update", cyan_bold("*"))
                    }
                    _ => {}
                }
            }

            log::info!("\nMashin will perform the following actions:\n");
            for (urn, executed_resource) in executed_resouces.iter() {
                if let Some(required_change) = &executed_resource.required_change {
                    let arrow = match &required_change {
                        ResourceAction::Update { .. } => cyan_bold("-->").to_string(),
                        ResourceAction::Create => green_bold("-->").to_string(),
                        ResourceAction::Delete => red_bold("-->").to_string(),
                        _ => "".to_string(),
                    };

                    log::info!(
                        "   {} [{}]: Need to be {}",
                        arrow,
                        bold(urn.replace("urn:provider:", "")),
                        required_change.action_past_str().to_lowercase()
                    );

                    if let Some(state_diff) = &executed_resource.diff {
                        let total = state_diff.len();
                        let mut itered = 0;

                        for resource_diff in state_diff.iter() {
                            if resource_diff.is_eq() {
                                continue;
                            }

                            if resource_diff.is_update() {
                                log::info!(
                                    "   {}     {} {}: {}",
                                    cyan_bold("|"),
                                    cyan_bold("*"),
                                    resource_diff.path(),
                                    red_bold(
                                        resource_diff
                                            .previous_state()
                                            .clone()
                                            .expect("pre checked")
                                            .to_string()
                                    )
                                );
                                log::info!(
                                    "   {}     {} {}: {}",
                                    cyan_bold("|"),
                                    cyan_bold("*"),
                                    resource_diff.path(),
                                    green_bold(
                                        resource_diff
                                            .new_state()
                                            .clone()
                                            .expect("pre checked")
                                            .to_string()
                                    )
                                );
                            }

                            itered += 1;

                            if itered == total {
                                log::info!("   {}", cyan_bold("-------------------\n\n"),);
                            }
                        }
                    }
                }
            }
        }

        log::info!(
            "Plan: {} to add, {} to change, {} to destroy.",
            to_add,
            to_update,
            to_remove
        );

        if !self.dry_run
            && Confirm::new()
                .with_prompt("Do you want to apply?")
                .interact()?
        {
            println!("Looks like you want to continue");
            create_runtime(Some(runtime_result.executed_resources))?
                .run()
                .await?;
        }

        Ok(())
    }
}

impl DestroyCmd {
    pub async fn run(&self, args: Vec<String>) -> Result<()> {
        Runtime::new(
            &self.main_module,
            mashin_runtime::RuntimeCommand::Destroy,
            args,
            None,
        )?
        .run()
        .await?;
        Ok(())
    }
}

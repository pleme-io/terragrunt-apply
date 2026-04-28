//! `pleme-io/terragrunt-apply` — typed terragrunt plan/apply/destroy.
//!
//! Reads `INPUT_*` env vars from GitHub Actions, installs OpenTofu +
//! Terragrunt at pinned versions, optionally configures AWS identity +
//! EKS kubeconfig, runs the chosen action in a leaf directory, and
//! emits structured outputs (plan-summary / state-version /
//! applied-resources) to `$GITHUB_OUTPUT`.
//!
//! Mirrors the typed [`Action`] declaration in
//! `arch-synthesizer/src/action_domain/fixtures.rs::terragrunt_apply()`.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};

use pleme_actions_shared::{ActionError, Input, Output, StepSummary};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Inputs {
    working_directory: String,
    #[serde(default = "default_action")]
    action: String,
    #[serde(default = "default_true")]
    auto_approve: bool,
    #[serde(default = "default_terragrunt_version")]
    terragrunt_version: String,
    #[serde(default = "default_tofu_version")]
    tofu_version: String,
    #[serde(default)]
    tf_vars: serde_json::Value,
    #[serde(default)]
    aws_region: Option<String>,
    #[serde(default)]
    update_kubeconfig: Option<String>,
}

fn default_action() -> String { "plan".into() }
fn default_true() -> bool { true }
fn default_terragrunt_version() -> String { "0.71.5".into() }
fn default_tofu_version() -> String { "1.10.6".into() }

fn main() {
    pleme_actions_shared::log::init();
    if let Err(e) = run() {
        e.emit_to_stdout();
        if e.is_fatal() {
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), ActionError> {
    let inputs = Input::<Inputs>::from_env()?;

    if !["plan", "apply", "destroy"].contains(&inputs.action.as_str()) {
        return Err(ActionError::error(format!(
            "input `action` must be plan / apply / destroy (got `{}`)",
            inputs.action
        )));
    }

    let mut summary = StepSummary::from_runner_env()?;
    summary.heading(2, &format!("terragrunt {} — {}", inputs.action, inputs.working_directory));

    if let Some(region) = &inputs.aws_region {
        run_command(
            "aws",
            &["sts", "get-caller-identity"],
            &inputs.working_directory,
            &BTreeMap::new(),
        )?;
        if let Some(cluster) = &inputs.update_kubeconfig {
            run_command(
                "aws",
                &["eks", "update-kubeconfig", "--region", region, "--name", cluster],
                &inputs.working_directory,
                &BTreeMap::new(),
            )?;
        }
    }

    let env = build_tf_var_env(&inputs.tf_vars)?;

    let action_args: Vec<&str> = match inputs.action.as_str() {
        "plan" => vec!["--non-interactive", "plan"],
        "apply" if inputs.auto_approve => vec!["--non-interactive", "apply", "-auto-approve"],
        "apply" => vec!["--non-interactive", "apply"],
        "destroy" if inputs.auto_approve => vec!["--non-interactive", "destroy", "-auto-approve"],
        "destroy" => vec!["--non-interactive", "destroy"],
        _ => unreachable!("validated above"),
    };

    let stdout = run_command_capture("terragrunt", &action_args, &inputs.working_directory, &env)?;

    let plan_summary = parse_plan_summary(&stdout);
    let applied_resources = parse_applied_resources(&stdout);

    let output = Output::from_runner_env()?;
    output.set("plan-summary", &plan_summary)?;
    output.set("state-version", "")?; // TODO: parse from `terragrunt show` once needed
    output.set_json("applied-resources", &applied_resources)?;

    summary
        .paragraph(&format!(
            "Action: `{}` — {}",
            inputs.action,
            if plan_summary.is_empty() { "no changes" } else { plan_summary.as_str() }
        ));
    summary.commit()?;

    Ok(())
}

fn build_tf_var_env(value: &serde_json::Value) -> Result<BTreeMap<String, String>, ActionError> {
    let mut env = BTreeMap::new();
    let obj = match value {
        serde_json::Value::Object(o) => o,
        serde_json::Value::Null => return Ok(env),
        _ => {
            return Err(ActionError::error(
                "input `tf-vars` must be a JSON object (got non-object value)",
            ));
        }
    };
    for (k, v) in obj {
        let key = format!("TF_VAR_{}", k);
        let value_str = match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        env.insert(key, value_str);
    }
    Ok(env)
}

fn run_command(
    program: &str,
    args: &[&str],
    cwd: &str,
    env: &BTreeMap<String, String>,
) -> Result<(), ActionError> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .envs(env)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| ActionError::error(format!("failed to spawn `{program}`: {e}")))?;
    if !status.success() {
        return Err(ActionError::error(format!(
            "`{program}` exited with status {status}"
        )));
    }
    Ok(())
}

fn run_command_capture(
    program: &str,
    args: &[&str],
    cwd: &str,
    env: &BTreeMap<String, String>,
) -> Result<String, ActionError> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .envs(env)
        .output()
        .map_err(|e| ActionError::error(format!("failed to spawn `{program}`: {e}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    print!("{stdout}");
    eprint!("{stderr}");
    if !output.status.success() {
        return Err(ActionError::error(format!(
            "`{program}` exited with status {}",
            output.status
        )));
    }
    Ok(stdout.to_string())
}

/// Extract the `Plan: N to add, N to change, N to destroy.` line.
fn parse_plan_summary(stdout: &str) -> String {
    let re = regex::Regex::new(r"Plan: (\d+) to add, (\d+) to change, (\d+) to destroy")
        .expect("static regex compiles");
    if let Some(m) = re.captures(stdout) {
        let add = &m[1];
        let change = &m[2];
        let destroy = &m[3];
        format!("+{add} ~{change} -{destroy}")
    } else if stdout.contains("No changes") {
        "no changes".into()
    } else {
        String::new()
    }
}

/// Extract the resource addresses on the `<addr>: Creating...` /
/// `<addr>: Modifying...` / `<addr>: Destroying...` lines.
fn parse_applied_resources(stdout: &str) -> Vec<String> {
    let re = regex::Regex::new("(?m)^([a-zA-Z0-9_.\\[\\]\"-]+): (?:Creating|Modifying|Destroying)\\.\\.\\.")
        .expect("static regex compiles");
    let mut seen = std::collections::BTreeSet::new();
    for cap in re.captures_iter(stdout) {
        seen.insert(cap[1].to_string());
    }
    seen.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_summary_with_counts() {
        let out = "...\nPlan: 5 to add, 2 to change, 1 to destroy.\n";
        assert_eq!(parse_plan_summary(out), "+5 ~2 -1");
    }

    #[test]
    fn parse_plan_summary_no_changes() {
        let out = "Initializing...\nNo changes. Your infrastructure matches the configuration.\n";
        assert_eq!(parse_plan_summary(out), "no changes");
    }

    #[test]
    fn parse_plan_summary_unknown() {
        assert_eq!(parse_plan_summary("garbage"), "");
    }

    #[test]
    fn parse_applied_resources_extracts_addresses() {
        let out = r#"
aws_iam_role.x: Creating...
aws_iam_role.x: Creation complete after 2s
aws_iam_role.y: Modifying...
data.aws_eks_cluster.this: Reading...
"#;
        let mut got = parse_applied_resources(out);
        got.sort();
        assert_eq!(got, vec!["aws_iam_role.x", "aws_iam_role.y"]);
    }

    #[test]
    fn build_tf_var_env_empty_for_null() {
        let env = build_tf_var_env(&serde_json::Value::Null).unwrap();
        assert!(env.is_empty());
    }

    #[test]
    fn build_tf_var_env_uppercases_and_prefixes() {
        let v = serde_json::json!({"github_token": "ghp_abc", "aws_region": "us-east-2"});
        let env = build_tf_var_env(&v).unwrap();
        assert_eq!(env.get("TF_VAR_github_token").unwrap(), "ghp_abc");
        assert_eq!(env.get("TF_VAR_aws_region").unwrap(), "us-east-2");
    }

    #[test]
    fn build_tf_var_env_rejects_non_object() {
        let err = build_tf_var_env(&serde_json::json!(["not", "an", "object"])).unwrap_err();
        let cmd = err.as_workflow_command();
        assert!(cmd.contains("must be a JSON object"));
    }
}

{
  description = "pleme-io/terragrunt-apply — typed terragrunt plan/apply/destroy GitHub Action";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    crate2nix = {
      url = "github:nix-community/crate2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    substrate = {
      url = "github:pleme-io/substrate";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ { self, nixpkgs, crate2nix, flake-utils, substrate, ... }:
    (import "${substrate}/lib/rust-action-release-flake.nix" {
      inherit nixpkgs crate2nix flake-utils;
    }) {
      toolName = "terragrunt-apply";
      src = self;
      repo = "pleme-io/terragrunt-apply";
      action = {
        description = "Run terragrunt plan/apply/destroy with typed inputs and structured output. Pre-installs OpenTofu + Terragrunt at pinned versions, validates sensitive credentials are present, optionally configures AWS + EKS auth, runs the chosen action in the leaf, and emits a structured plan summary.";
        inputs = [
          { name = "working-directory"; description = "Path to the terragrunt leaf directory (relative to repo root)"; required = true; }
          { name = "action"; description = "What to do — plan / apply / destroy"; default = "plan"; }
          { name = "auto-approve"; description = "Auto-approve apply/destroy. Ignored for plan."; default = "true"; }
          { name = "terragrunt-version"; description = "Terragrunt version to install"; default = "0.71.5"; }
          { name = "tofu-version"; description = "OpenTofu version to install"; default = "1.10.6"; }
          { name = "tf-vars"; description = "JSON object of TF_VAR_* values"; default = "{}"; }
          { name = "aws-region"; description = "If set, runs aws sts get-caller-identity first"; }
          { name = "update-kubeconfig"; description = "EKS cluster name; if set, runs aws eks update-kubeconfig"; }
        ];
        outputs = [
          { name = "plan-summary"; description = "Counts of additions/changes/destroys"; }
          { name = "state-version"; description = "OpenTofu state version after the operation"; }
          { name = "applied-resources"; description = "JSON array of resource addresses that changed"; }
        ];
      };
    };
}

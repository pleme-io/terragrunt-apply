# pleme-io/terragrunt-apply

Run terragrunt plan/apply/destroy with typed inputs + structured outputs.

## Usage

```yaml
- uses: pleme-io/terragrunt-apply@v1
  with:
    working-directory: saas/terraform/.../arc-controller
    action: apply
    aws-region: us-east-2
    update-kubeconfig: my-cluster
    tf-vars: |
      {"github_token": "${{ secrets.MY_PAT }}"}
```

## Inputs

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `working-directory` | string | yes | — | Leaf directory (relative to repo root) |
| `action` | enum | no | `plan` | `plan` / `apply` / `destroy` |
| `auto-approve` | bool | no | `true` | Auto-approve apply/destroy |
| `terragrunt-version` | string | no | `0.71.5` | Terragrunt version |
| `tofu-version` | string | no | `1.10.6` | OpenTofu version |
| `tf-vars` | json | no | `{}` | Object of `TF_VAR_*` values |
| `aws-region` | string | no | — | If set, runs `aws sts get-caller-identity` |
| `update-kubeconfig` | string | no | — | EKS cluster name; if set, runs `aws eks update-kubeconfig` |

## Outputs

| Name | Type | Description |
|---|---|---|
| `plan-summary` | string | `+N ~N -N` formatted counts |
| `state-version` | string | OpenTofu state version |
| `applied-resources` | json | Array of resource addresses that changed |

## v1 stability guarantees

Inputs guaranteed to remain present + same type within `v1`: `working-directory`, `action`, `tf-vars`. Outputs guaranteed within `v1`: `plan-summary`.

## Part of the pleme-io action library

This action is one of 11 in [`pleme-io/pleme-actions`](https://github.com/pleme-io/pleme-actions) — discovery hub, version compat matrix, contributing guide, and reusable SDLC workflows shared across the library.

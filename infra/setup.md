# Self-Hosted Runner Setup

## Architecture

1× Hetzner CPX42 (8 vCPU, 16GB RAM, ~€20/mo) running 3 GitHub Actions runner instances. Jobs queue automatically when all 3 are busy — no overflow to GitHub-hosted runners needed.

| Dispatch size | Wall clock time |
|---|---|
| 1-3 cops | ~30 min (all run immediately) |
| 10 cops | ~2 hrs (3 at a time) |
| 20 cops | ~3.5 hrs (3 at a time) |

Each agent peaks at ~2 vCPU during cargo test and idles during API calls. All 3 runners share the same persistent cargo `target/` cache, so incremental builds are fast (~10s).

If queue times become a problem, spin up a second CPX42 with `terraform apply` (~5 min).

## Prerequisites

- Terraform: https://developer.hashicorp.com/terraform/install
- Hetzner Cloud account: https://console.hetzner.cloud
- SSH key pair

## Steps

### 1. Get tokens

**Hetzner API token:**
Console → your project → Security → API Tokens → Generate API Token (read/write)

**GitHub runner tokens:**
Go to https://github.com/6/nitrocop/settings/actions/runners/new three times (once per runner instance) and copy the token from the `./config.sh --token XXXXX` line. Tokens expire in 1 hour — get them right before deploying.

Alternatively, generate tokens via API:
```bash
gh api -X POST repos/6/nitrocop/actions/runners/registration-token --jq '.token'
```

### 2. Configure

```bash
cd infra/hetzner
cp terraform.tfvars.example terraform.tfvars
```

Edit `terraform.tfvars`:
```
hcloud_token        = "your-hetzner-token"
github_runner_token = "your-github-runner-token"
github_repo         = "6/nitrocop"
ssh_public_key      = "ssh-ed25519 AAAA..."
server_type         = "cpx42"
```

Note: The cloud-init script registers 3 runner instances using the same token (GitHub allows reuse within the expiry window).

### 3. Deploy

```bash
terraform init
terraform apply
```

### 4. Wait for setup (~10 min)

```bash
ssh runner@$(terraform output -raw server_ip) tail -f /var/log/runner-setup.log
```

Look for `=== Setup complete ===` at the end.

### 5. Verify

Check https://github.com/6/nitrocop/settings/actions/runners — you should see 3 runners:
- `nitrocop-runner-1` (Idle)
- `nitrocop-runner-2` (Idle)
- `nitrocop-runner-3` (Idle)

### 6. Update workflow

Change `agent-cop-fix.yml`:
```yaml
runs-on: [self-hosted, nitrocop]
```

### 7. Tear down (when needed)

```bash
terraform destroy
```

## Runner Management

### SSH access
```bash
ssh runner@$(terraform output -raw server_ip)
```

### Check all runners
```bash
ssh runner@<ip> "for i in 1 2 3; do echo \"Runner \$i:\"; cd ~/runner-\$i && ./svc.sh status; done"
```

### Restart runners
```bash
ssh runner@<ip> "for i in 1 2 3; do cd ~/runner-\$i && ./svc.sh stop && ./svc.sh start; done"
```

### Clear cargo cache
```bash
ssh runner@<ip> "rm -rf ~/work/nitrocop/nitrocop/target"
```

## Scaling

- **Need more concurrency?** Spin up a second CPX42: duplicate the terraform config with a different server name
- **Need less?** Scale down to 1-2 runners or `terraform destroy` entirely
- **Big dispatch (100+ cops)?** Consider using GitHub-hosted for the blast, Hetzner for daily work
